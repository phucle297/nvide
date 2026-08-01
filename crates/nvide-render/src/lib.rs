//! One-atlas shaped-text renderer for the Phase 0 viewport.

use cosmic_text::{
    Attrs, Buffer as ShapingBuffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
};
use std::{
    error::Error,
    fmt,
    future::Future,
    sync::{mpsc, Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt;

const ATLAS_SIZE: u32 = 1_024;
const SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@vertex
fn vs_main(@location(0) input: vec4<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.xy, 0.0, 1.0);
    output.uv = input.zw;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(atlas, atlas_sampler, input.uv);
}
"#;

#[derive(Debug)]
pub enum RenderError {
    Initialization(String),
    AtlasFull,
    SurfaceLost,
    Timeout,
    OutOfMemory,
    DeviceLost(String),
    Timestamp(String),
    ReadbackUnsupported,
    Readback(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialization(message) => {
                write!(formatter, "renderer initialization failed: {message}")
            }
            Self::AtlasFull => formatter.write_str("the Phase 0 glyph atlas is full"),
            Self::SurfaceLost => {
                formatter.write_str("render surface remained unavailable after reconfigure")
            }
            Self::Timeout => formatter.write_str("render surface timed out"),
            Self::OutOfMemory => formatter.write_str("GPU is out of memory"),
            Self::DeviceLost(message) => write!(formatter, "GPU device was lost: {message}"),
            Self::Timestamp(message) => {
                write!(formatter, "presentation timestamp failed: {message}")
            }
            Self::ReadbackUnsupported => {
                formatter.write_str("the render surface does not support frame readback")
            }
            Self::Readback(message) => write!(formatter, "frame readback failed: {message}"),
        }
    }
}

impl Error for RenderError {}

pub struct Renderer {
    adapter_info: wgpu::AdapterInfo,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    atlas: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    font_system: FontSystem,
    shaping: ShapingBuffer,
    swash_cache: SwashCache,
    text: String,
    benchmark_marker: Option<u64>,
    glyph_count: usize,
    frame_sequence: u64,
    readback_supported: bool,
    device_loss: Arc<Mutex<Option<String>>>,
}

pub struct PresentedFrame {
    pub sequence: u64,
    pub present_ns: u64,
    pub readback: Option<FrameReadback>,
}

pub struct FrameReadback {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Renderer {
    pub fn new<W>(window: Arc<W>, width: u32, height: u32) -> Result<Self, RenderError>
    where
        W: wgpu::rwh::HasDisplayHandle + wgpu::rwh::HasWindowHandle + Send + Sync + 'static,
    {
        block_on(Self::new_async(window, width, height))
    }

    async fn new_async<W>(window: Arc<W>, width: u32, height: u32) -> Result<Self, RenderError>
    where
        W: wgpu::rwh::HasDisplayHandle + wgpu::rwh::HasWindowHandle + Send + Sync + 'static,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window)
            .map_err(|error| RenderError::Initialization(error.to_string()))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| RenderError::Initialization("no compatible GPU adapter".to_owned()))?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("nvide-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .map_err(|error| RenderError::Initialization(error.to_string()))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| RenderError::Initialization("surface exposes no format".to_owned()))?;
        let alpha_mode = capabilities.alpha_modes.first().copied().ok_or_else(|| {
            RenderError::Initialization("surface exposes no alpha mode".to_owned())
        })?;
        let readback_supported = capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | if readback_supported {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let device_loss = Arc::new(Mutex::new(None));
        let callback_state = Arc::clone(&device_loss);
        device.set_device_lost_callback(move |reason, message| {
            if let Ok(mut state) = callback_state.lock() {
                *state = Some(format!("{reason:?}: {message}"));
            }
        });

        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nvide-glyph-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nvide-atlas-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nvide-atlas-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
            label: Some("nvide-atlas-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nvide-text-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nvide-text-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nvide-text-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let vertex_buffer =
            empty_buffer(&device, "nvide-empty-vertices", wgpu::BufferUsages::VERTEX);
        let index_buffer = empty_buffer(&device, "nvide-empty-indices", wgpu::BufferUsages::INDEX);
        let mut font_system = FontSystem::new();
        let shaping = ShapingBuffer::new(&mut font_system, Metrics::new(24.0, 32.0));
        Ok(Self {
            adapter_info,
            surface,
            device,
            queue,
            config,
            pipeline,
            atlas,
            bind_group,
            vertex_buffer,
            index_buffer,
            index_count: 0,
            font_system,
            shaping,
            swash_cache: SwashCache::new(),
            text: String::new(),
            benchmark_marker: None,
            glyph_count: 0,
            frame_sequence: 0,
            readback_supported,
            device_loss,
        })
    }

    pub fn benchmark_adapter_manifest(&self) -> String {
        adapter_manifest(&self.adapter_info)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        let text = match self.benchmark_marker {
            Some(sequence) => format!("{}\nframe:{sequence}", self.text),
            None => self.text.clone(),
        };
        self.shape(&text)
    }

    pub fn set_text(&mut self, text: &str) -> Result<(), RenderError> {
        self.text.clear();
        self.text.push_str(text);
        self.benchmark_marker = None;
        self.shape(text)
    }

    pub fn set_benchmark_text(&mut self, text: &str) -> Result<u64, RenderError> {
        self.text.clear();
        self.text.push_str(text);
        let sequence = self.frame_sequence.saturating_add(1);
        self.benchmark_marker = Some(sequence);
        self.shape(&format!("{text}\nframe:{sequence}"))?;
        Ok(sequence)
    }

    fn shape(&mut self, text: &str) -> Result<(), RenderError> {
        self.shaping.set_size(
            &mut self.font_system,
            Some(self.config.width as f32),
            Some(self.config.height as f32),
        );
        self.shaping.set_text(
            &mut self.font_system,
            text,
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );
        let (atlas_bytes, vertices, indices) = build_atlas(
            &self.shaping,
            &mut self.font_system,
            &mut self.swash_cache,
            self.config.width,
            self.config.height,
        )?;
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE * 4),
                rows_per_image: Some(ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("nvide-glyph-vertices"),
                contents: &vertices,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("nvide-glyph-indices"),
                contents: &indices,
                usage: wgpu::BufferUsages::INDEX,
            });
        self.index_count = u32::try_from(indices.len() / 4).map_err(|_| RenderError::AtlasFull)?;
        self.glyph_count = self.index_count as usize / 6;
        Ok(())
    }

    pub fn glyph_count(&self) -> usize {
        self.glyph_count
    }

    pub fn first_line_glyph_count(&self) -> usize {
        self.shaping
            .layout_runs()
            .next()
            .map_or(0, |run| run.glyphs.len())
    }

    pub fn render(
        &mut self,
        capture_deadline: Option<Instant>,
    ) -> Result<PresentedFrame, RenderError> {
        self.check_device()?;
        if capture_deadline.is_some() && !self.readback_supported {
            return Err(RenderError::ReadbackUnsupported);
        }
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture().map_err(surface_error)?
            }
            Err(error) => return Err(surface_error(error)),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nvide-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nvide-clear-and-text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.035,
                            g: 0.04,
                            b: 0.055,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if self.index_count > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }
        let readback = capture_deadline
            .map(|deadline| (self.encode_readback(&mut encoder, &frame.texture), deadline));
        self.queue.submit([encoder.finish()]);
        frame.present();
        let present_ns = nvide_platform::monotonic_ns()
            .map_err(|error| RenderError::Timestamp(error.to_string()))?;
        self.frame_sequence = self.frame_sequence.saturating_add(1);
        let readback = readback
            .map(|(pending, deadline)| self.finish_readback(pending, deadline))
            .transpose()?;
        self.check_device()?;
        Ok(PresentedFrame {
            sequence: self.frame_sequence,
            present_ns,
            readback,
        })
    }

    fn check_device(&self) -> Result<(), RenderError> {
        let state = self
            .device_loss
            .lock()
            .map_err(|_| RenderError::DeviceLost("device-loss state is poisoned".to_owned()))?;
        match state.as_ref() {
            Some(message) => Err(RenderError::DeviceLost(message.clone())),
            None => Ok(()),
        }
    }

    fn encode_readback(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> PendingReadback {
        let width = self.config.width;
        let height = self.config.height.min(64);
        let row_bytes = width * 4;
        let padded_row_bytes = row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nvide-frame-readback"),
            size: u64::from(padded_row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        PendingReadback {
            buffer,
            width,
            height,
            row_bytes,
            padded_row_bytes,
        }
    }

    fn finish_readback(
        &self,
        pending: PendingReadback,
        deadline: Instant,
    ) -> Result<FrameReadback, RenderError> {
        let slice = pending.buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let mapped = loop {
            self.device.poll(wgpu::Maintain::Poll);
            if Instant::now() >= deadline {
                return Err(readback_deadline_error());
            }
            match receiver.try_recv() {
                Ok(result) => break result,
                Err(mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(mpsc::TryRecvError::Empty) => return Err(readback_deadline_error()),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(RenderError::Readback(
                        "mapping callback disconnected".to_owned(),
                    ))
                }
            }
        };
        mapped.map_err(|error| RenderError::Readback(error.to_string()))?;
        if Instant::now() >= deadline {
            return Err(readback_deadline_error());
        }
        let mapped = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((pending.row_bytes * pending.height) as usize);
        for row in mapped.chunks_exact(pending.padded_row_bytes as usize) {
            rgba.extend_from_slice(&row[..pending.row_bytes as usize]);
        }
        drop(mapped);
        pending.buffer.unmap();
        if Instant::now() >= deadline {
            return Err(readback_deadline_error());
        }
        Ok(FrameReadback {
            width: pending.width,
            height: pending.height,
            rgba,
        })
    }
}

fn adapter_manifest(info: &wgpu::AdapterInfo) -> String {
    let one_line = |value: &str| value.replace(['\r', '\n'], " ");
    format!(
        "format=nvide-phase0-renderer-v1\nwgpu_backend={:?}\nadapter_name={}\nadapter_vendor=0x{:04X}\nadapter_device=0x{:04X}\nadapter_type={:?}\nadapter_driver={}\nadapter_driver_info={}\n",
        info.backend,
        one_line(&info.name),
        info.vendor,
        info.device,
        info.device_type,
        one_line(&info.driver),
        one_line(&info.driver_info)
    )
}

fn readback_deadline_error() -> RenderError {
    RenderError::Readback("mapping exceeded the trace deadline".to_owned())
}

struct PendingReadback {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    row_bytes: u32,
    padded_row_bytes: u32,
}

fn empty_buffer(device: &wgpu::Device, label: &str, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: 4,
        usage,
        mapped_at_creation: false,
    })
}

fn surface_error(error: wgpu::SurfaceError) -> RenderError {
    match error {
        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => RenderError::SurfaceLost,
        wgpu::SurfaceError::Timeout => RenderError::Timeout,
        wgpu::SurfaceError::OutOfMemory => RenderError::OutOfMemory,
    }
}

#[allow(clippy::type_complexity)]
fn build_atlas(
    shaping: &ShapingBuffer,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), RenderError> {
    let mut atlas = vec![0_u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut atlas_x = 1_u32;
    let mut atlas_y = 1_u32;
    let mut row_height = 0_u32;
    let mut glyph_index = 0_u32;

    for run in shaping.layout_runs() {
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let Some(image) = cache.get_image(font_system, physical.cache_key).clone() else {
                continue;
            };
            let glyph_width = image.placement.width;
            let glyph_height = image.placement.height;
            if glyph_width == 0 || glyph_height == 0 {
                continue;
            }
            if atlas_x + glyph_width + 1 > ATLAS_SIZE {
                atlas_x = 1;
                atlas_y = atlas_y.saturating_add(row_height + 1);
                row_height = 0;
            }
            if atlas_y + glyph_height + 1 > ATLAS_SIZE {
                return Err(RenderError::AtlasFull);
            }
            row_height = row_height.max(glyph_height);
            copy_glyph(&mut atlas, atlas_x, atlas_y, &image);

            let x = physical.x + image.placement.left;
            let y = run.line_y as i32 + physical.y - image.placement.top;
            let x0 = pixel_to_ndc_x(x as f32, width);
            let x1 = pixel_to_ndc_x((x + glyph_width as i32) as f32, width);
            let y0 = pixel_to_ndc_y(y as f32, height);
            let y1 = pixel_to_ndc_y((y + glyph_height as i32) as f32, height);
            let u0 = atlas_x as f32 / ATLAS_SIZE as f32;
            let u1 = (atlas_x + glyph_width) as f32 / ATLAS_SIZE as f32;
            let v0 = atlas_y as f32 / ATLAS_SIZE as f32;
            let v1 = (atlas_y + glyph_height) as f32 / ATLAS_SIZE as f32;
            for vertex in [
                [x0, y0, u0, v0],
                [x1, y0, u1, v0],
                [x1, y1, u1, v1],
                [x0, y1, u0, v1],
            ] {
                for value in vertex {
                    vertices.extend(value.to_ne_bytes());
                }
            }
            for index in [0, 1, 2, 0, 2, 3] {
                indices.extend((glyph_index * 4 + index).to_ne_bytes());
            }
            glyph_index = glyph_index.checked_add(1).ok_or(RenderError::AtlasFull)?;
            atlas_x += glyph_width + 1;
        }
    }
    Ok((atlas, vertices, indices))
}

fn copy_glyph(atlas: &mut [u8], x: u32, y: u32, image: &cosmic_text::SwashImage) {
    let width = image.placement.width as usize;
    let height = image.placement.height as usize;
    for row in 0..height {
        for column in 0..width {
            let destination = ((y as usize + row) * ATLAS_SIZE as usize + x as usize + column) * 4;
            match image.content {
                SwashContent::Mask => {
                    let alpha = image.data[row * width + column];
                    atlas[destination..destination + 4].copy_from_slice(&[238, 241, 255, alpha]);
                }
                SwashContent::SubpixelMask | SwashContent::Color => {
                    let source = (row * width + column) * 4;
                    atlas[destination..destination + 4]
                        .copy_from_slice(&image.data[source..source + 4]);
                }
            }
        }
    }
}

fn pixel_to_ndc_x(value: f32, width: u32) -> f32 {
    value * 2.0 / width.max(1) as f32 - 1.0
}

fn pixel_to_ndc_y(value: f32, height: u32) -> f32 {
    1.0 - value * 2.0 / height.max(1) as f32
}

pub fn shape_text(text: &str) -> usize {
    let mut font_system = FontSystem::new();
    let mut buffer = ShapingBuffer::new(&mut font_system, Metrics::new(16.0, 22.0));
    buffer.set_size(&mut font_system, Some(800.0), Some(600.0));
    buffer.set_text(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    buffer.layout_runs().map(|run| run.glyphs.len()).sum()
}

fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWake(thread::Thread);
    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosmic_text_shapes_bidi_and_fallback_text() {
        assert!(shape_text("NVide → λ مرحبا") > 5);
    }

    #[test]
    fn coordinates_cover_the_surface() {
        assert_eq!(pixel_to_ndc_x(0.0, 100), -1.0);
        assert_eq!(pixel_to_ndc_x(100.0, 100), 1.0);
        assert_eq!(pixel_to_ndc_y(0.0, 100), 1.0);
        assert_eq!(pixel_to_ndc_y(100.0, 100), -1.0);
    }

    #[test]
    fn device_loss_is_a_typed_error() {
        assert_eq!(
            RenderError::DeviceLost("reset".to_owned()).to_string(),
            "GPU device was lost: reset"
        );
    }

    #[test]
    fn adapter_manifest_is_single_line_per_field() {
        let manifest = adapter_manifest(&wgpu::AdapterInfo {
            name: "AMD\n860M".to_owned(),
            vendor: 0x1002,
            device: 0x150E,
            device_type: wgpu::DeviceType::IntegratedGpu,
            driver: "AMD\rdriver".to_owned(),
            driver_info: "32.0".to_owned(),
            backend: wgpu::Backend::Vulkan,
        });
        assert!(manifest.contains("wgpu_backend=Vulkan\nadapter_name=AMD 860M\n"));
        assert!(manifest.contains("adapter_vendor=0x1002\nadapter_device=0x150E\n"));
        assert!(manifest.contains("adapter_driver=AMD driver\n"));
    }
}
