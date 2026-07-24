//! winit + wgpu window prototype: clear path + monospaced glyphs from rope.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use nvide_buffer::{Buffer, BufferId, RopeBuffer};
use nvide_render::{
    build_atlas_r8, cells_to_vertices, layout_buffer, preview_lines, LayoutOptions, ATLAS_HEIGHT,
    ATLAS_WIDTH, CLEAR_COLOR,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

pub fn run_ui(initial_text: String, max_frames: u32) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(initial_text, max_frames);
    let run_result = event_loop.run_app(&mut app);
    // Always tear GPU down with an explicit order before `App`/`Window` drop.
    app.destroy_gpu();
    if let Some(err) = app.fatal.take() {
        return Err(err);
    }
    // Smoke path: if we already presented the requested frames, compositor
    // disconnect errors during loop shutdown are not product failures (common
    // on headless/virtual Wayland where the socket resets on window destroy).
    let smoke_done = max_frames > 0 && app.frames >= max_frames && app.last_gpu_vertex_count > 0;
    match run_result {
        Ok(()) => {}
        Err(e) if smoke_done => {
            eprintln!("nvide-ui: event_loop_end_after_smoke ignored: {e}");
        }
        Err(e) => return Err(e.into()),
    }
    eprintln!(
        "nvide-ui: exit_clean frames={} gpu_glyph_draw={} gpu_vertices={}",
        app.frames, app.last_gpu_glyph_count, app.last_gpu_vertex_count
    );
    Ok(())
}

struct App {
    buffer: RopeBuffer,
    max_frames: u32,
    frames: u32,
    state: Option<RenderState>,
    fatal: Option<Box<dyn std::error::Error>>,
    last_preview: Vec<String>,
    /// Vertices submitted on the last frame (glyph path proof).
    last_gpu_glyph_count: u32,
    last_gpu_vertex_count: u32,
    /// Once true, no further redraw/present is requested.
    exiting: bool,
}

/// GPU + window state.
///
/// Field order matters for `Drop`: Rust drops fields in reverse declaration
/// order. We want window last, so it is declared first. Prefer
/// [`App::destroy_gpu`] for an explicit ordered teardown before event-loop exit.
struct RenderState {
    // Dropped last → first fields
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    /// Keeps atlas alive for the lifetime of `bind_group`'s texture view.
    atlas_texture: wgpu::Texture,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    // Dropped first → last fields
    vertex_capacity: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

const MAX_GLYPHS: u64 = 4096;
const VERTS_PER_GLYPH: u64 = 6;
const VERTEX_SIZE: u64 = std::mem::size_of::<GpuVertex>() as u64;

impl App {
    fn new(initial_text: String, max_frames: u32) -> Self {
        let buffer = RopeBuffer::from_str(BufferId(1), &initial_text);
        let last_preview = preview_lines(&layout_buffer(&buffer, LayoutOptions::default()));
        Self {
            buffer,
            max_frames,
            frames: 0,
            state: None,
            fatal: None,
            last_preview,
            last_gpu_glyph_count: 0,
            last_gpu_vertex_count: 0,
            exiting: false,
        }
    }

    /// Tear down wgpu resources in a safe order, then the window.
    ///
    /// Order: wait GPU → pipeline/bind_group/buffers/atlas → surface → queue/device → window.
    /// Call this before `event_loop.exit()` so drop does not race the winit teardown.
    fn destroy_gpu(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };
        // Ensure submitted frames finish before destroying swapchain/resources.
        state.device.poll(wgpu::Maintain::Wait);

        let RenderState {
            window,
            device,
            queue,
            surface,
            atlas_texture,
            config: _,
            pipeline,
            bind_group,
            vertex_buffer,
            vertex_capacity: _,
        } = state;

        drop(pipeline);
        drop(bind_group);
        drop(vertex_buffer);
        drop(atlas_texture);
        drop(surface);
        drop(queue);
        drop(device);
        drop(window);
        eprintln!("nvide-ui: gpu_teardown_ok");
    }

    fn request_exit(&mut self, event_loop: &ActiveEventLoop) {
        if self.exiting {
            return;
        }
        self.exiting = true;
        // Do not destroy the window/surface here: winit is still dispatching.
        // GPU teardown runs after `run_app` returns (see `run_ui`).
        event_loop.exit();
    }

    fn relayout(&mut self) {
        self.last_preview = preview_lines(&layout_buffer(&self.buffer, LayoutOptions::default()));
    }

    fn insert_char(&mut self, ch: char) {
        let pos = self.buffer.len_chars();
        let _ = self.buffer.insert_tracked(pos, &ch.to_string());
        self.relayout();
        if let Some(s) = &self.state {
            s.window.request_redraw();
        }
    }

    fn backspace(&mut self) {
        let len = self.buffer.len_chars();
        if len == 0 {
            return;
        }
        let _ = self.buffer.delete_tracked(len - 1..len);
        self.relayout();
        if let Some(s) = &self.state {
            s.window.request_redraw();
        }
    }

    fn init_wgpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let attrs = Window::default_attributes()
            .with_title("NVide Phase 0")
            .with_inner_size(LogicalSize::new(960.0, 640.0));
        let window = Arc::new(event_loop.create_window(attrs)?);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or("no suitable wgpu adapter found")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("nvide-device"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Bitmap font atlas → R8 texture sampled by the glyph pipeline.
        let atlas = build_atlas_r8();
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_WIDTH),
                rows_per_image: Some(ATLAS_HEIGHT),
            },
            wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-bg"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glyph-shader"),
            source: wgpu::ShaderSource::Wgsl(GLYPH_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glyph-pl"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glyph-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: VERTEX_SIZE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertex_capacity = MAX_GLYPHS * VERTS_PER_GLYPH * VERTEX_SIZE;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-vb"),
            size: vertex_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        eprintln!(
            "nvide-ui: wgpu ready backend_adapter clear=[{:.2},{:.2},{:.2},{:.2}] text={:?}",
            CLEAR_COLOR[0],
            CLEAR_COLOR[1],
            CLEAR_COLOR[2],
            CLEAR_COLOR[3],
            self.buffer.to_string()
        );
        eprintln!(
            "nvide-ui: glyph_atlas={}x{} r8 bytes={} glyph_preview={:?}",
            ATLAS_WIDTH,
            ATLAS_HEIGHT,
            atlas.len(),
            self.last_preview
        );

        self.state = Some(RenderState {
            window,
            device,
            queue,
            surface,
            atlas_texture,
            config,
            pipeline,
            bind_group,
            vertex_buffer,
            vertex_capacity,
        });
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        if let Some(state) = &mut self.state {
            if width > 0 && height > 0 {
                state.config.width = width;
                state.config.height = height;
                state.surface.configure(&state.device, &state.config);
            }
        }
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = &self.state else {
            return Ok(());
        };

        // Rope → monospaced GlyphCell → textured quads on the GPU.
        let cells = layout_buffer(&self.buffer, LayoutOptions::default());
        let vw = state.config.width as f32;
        let vh = state.config.height as f32;
        let cpu_verts = cells_to_vertices(&cells, (vw, vh), (16.0, 16.0));
        let gpu_verts: Vec<GpuVertex> = cpu_verts
            .iter()
            .map(|v| GpuVertex {
                pos: v.pos,
                uv: v.uv,
            })
            .collect();
        let glyph_draw_count = cells.iter().filter(|c| c.ch != ' ').count() as u32;
        let vertex_count = gpu_verts.len() as u32;

        if !gpu_verts.is_empty() {
            let bytes = bytemuck::cast_slice(&gpu_verts);
            if (bytes.len() as u64) <= state.vertex_capacity {
                state.queue.write_buffer(&state.vertex_buffer, 0, bytes);
            }
        }

        let frame = state.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nvide-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nvide-clear-and-glyphs"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: CLEAR_COLOR[0],
                            g: CLEAR_COLOR[1],
                            b: CLEAR_COLOR[2],
                            a: CLEAR_COLOR[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if vertex_count > 0 {
                pass.set_pipeline(&state.pipeline);
                pass.set_bind_group(0, &state.bind_group, &[]);
                pass.set_vertex_buffer(0, state.vertex_buffer.slice(..));
                pass.draw(0..vertex_count, 0..1);
            }
        }
        state.queue.submit(Some(encoder.finish()));
        frame.present();

        self.last_gpu_glyph_count = glyph_draw_count;
        self.last_gpu_vertex_count = vertex_count;
        self.frames = self.frames.saturating_add(1);
        if self.frames == 1 {
            eprintln!(
                "nvide-ui: first_frame_presented clear=yes gpu_glyph_draw={} gpu_vertices={} preview={:?}",
                glyph_draw_count, vertex_count, self.last_preview
            );
            eprintln!(
                "nvide-ui: on_surface_glyph_path=atlas_textured_quads buffer_text={:?}",
                self.buffer.to_string()
            );
        }
        Ok(())
    }
}

const GLYPH_SHADER: &str = r#"
struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = vec4<f32>(v.pos, 0.0, 1.0);
    out.uv = v.uv;
    return out;
}

@group(0) @binding(0) var atlas_tex: texture_2d<f32>;
@group(0) @binding(1) var atlas_smp: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let a = textureSample(atlas_tex, atlas_smp, in.uv).r;
    // Soft white-on-dark ink; discard fully transparent texels.
    if (a < 0.1) {
        discard;
    }
    return vec4<f32>(0.92, 0.94, 0.98, a);
}
"#;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.exiting {
            return;
        }
        if self.state.is_none() {
            if let Err(e) = self.init_wgpu(event_loop) {
                eprintln!("nvide-ui: failed to init wgpu/window: {e}");
                self.fatal = Some(e);
                self.request_exit(event_loop);
            } else if let Some(s) = &self.state {
                s.window.request_redraw();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.exiting {
            return;
        }
        match event {
            WindowEvent::CloseRequested => self.request_exit(event_loop),
            WindowEvent::Resized(size) => self.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render() {
                    eprintln!("nvide-ui: render error: {e}");
                    if let Some(state) = &self.state {
                        let size = state.window.inner_size();
                        self.resize(size.width.max(1), size.height.max(1));
                    }
                }
                if self.max_frames > 0 && self.frames >= self.max_frames {
                    eprintln!(
                        "nvide-ui: max_frames reached ({}) buffer={:?} preview={:?} gpu_glyph_draw={} gpu_vertices={}",
                        self.frames,
                        self.buffer.to_string(),
                        self.last_preview,
                        self.last_gpu_glyph_count,
                        self.last_gpu_vertex_count
                    );
                    self.request_exit(event_loop);
                    return;
                }
                if !self.exiting {
                    if let Some(s) = &self.state {
                        s.window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        text,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Escape) => self.request_exit(event_loop),
                Key::Named(NamedKey::Backspace) => self.backspace(),
                Key::Named(NamedKey::Enter) => self.insert_char('\n'),
                _ => {
                    if let Some(t) = text {
                        for ch in t.chars() {
                            if !ch.is_control() || ch == '\t' {
                                self.insert_char(ch);
                            }
                        }
                    }
                }
            },
            _ => {}
        }
    }
}
