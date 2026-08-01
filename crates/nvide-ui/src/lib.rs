//! Winit-owned Phase 0 UI, input routing, and core supervision.

use nvide_ipc::schema;
use std::{
    collections::VecDeque,
    env,
    error::Error,
    fmt, fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::Key,
    window::{Window, WindowId},
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const RESTART_WINDOW: Duration = Duration::from_secs(60);
const MAX_RESTARTS: usize = 3;
const EDIT_SENTINELS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

#[derive(Debug)]
pub struct UiError(String);

impl fmt::Display for UiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for UiError {}

pub fn run() -> Result<(), UiError> {
    let options = RunOptions::parse(env::args().skip(1))?;
    let event_loop = EventLoop::new().map_err(display_error)?;
    event_loop.set_control_flow(idle_control_flow(
        options.benchmark.is_some(),
        Instant::now() + HEARTBEAT_INTERVAL,
    ));
    let mut app = App::new(options);
    event_loop.run_app(&mut app).map_err(display_error)?;
    match app.failure.take() {
        Some(failure) => Err(UiError(failure)),
        None => Ok(()),
    }
}

fn idle_control_flow(benchmark: bool, next_heartbeat: Instant) -> ControlFlow {
    if benchmark {
        ControlFlow::Poll
    } else {
        ControlFlow::WaitUntil(next_heartbeat)
    }
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<nvide_render::Renderer>,
    core: Option<CoreSupervisor>,
    text: String,
    version: u64,
    trace_id: u64,
    heartbeat_sequence: u64,
    last_heartbeat: Instant,
    failure: Option<String>,
    benchmark: Option<Benchmark>,
}

impl App {
    fn new(options: RunOptions) -> Self {
        Self {
            window: None,
            renderer: None,
            core: None,
            text: String::new(),
            version: 0,
            trace_id: 0,
            heartbeat_sequence: 0,
            last_heartbeat: Instant::now(),
            failure: None,
            benchmark: options.benchmark,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Keep the native window alive until its GPU surface is gone.
        drop(self.renderer.take());
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let benchmark = self.benchmark.is_some();
        let attributes = Window::default_attributes()
            .with_title("NVide Phase 0")
            .with_inner_size(if benchmark {
                winit::dpi::PhysicalSize::new(1920, 1080)
            } else {
                winit::dpi::PhysicalSize::new(960, 600)
            });
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.fail(event_loop, error),
        };
        let size = window.inner_size();
        let mut renderer =
            match nvide_render::Renderer::new(window.clone(), size.width, size.height) {
                Ok(renderer) => renderer,
                Err(error) => return self.fail(event_loop, error),
            };
        let needs_core = !matches!(self.benchmark, Some(Benchmark::Clear { .. }));
        if needs_core {
            match core_executable().and_then(CoreSupervisor::start) {
                Ok(core) => self.core = Some(core),
                Err(error) => {
                    self.text = format!("Core unavailable: {error}");
                    if let Err(render_error) = renderer.set_text(&self.text) {
                        return self.fail(event_loop, render_error);
                    }
                }
            }
        }
        if let Some(benchmark) = self.benchmark.as_mut() {
            if let Err(error) = benchmark.start() {
                return self.fail(event_loop, error);
            }
        }
        self.renderer = Some(renderer);
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.resize(size.width, size.height) {
                        self.handle_render_error(event_loop, error);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let Key::Character(text) = event.logical_key {
                    if !text.chars().any(char::is_control) {
                        self.apply_text(event_loop, text.as_str());
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let capture = self
                    .benchmark
                    .as_ref()
                    .is_some_and(Benchmark::needs_readback);
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(capture) {
                        Ok(frame) => self.after_present(event_loop, frame),
                        Err(error) => self.handle_render_error(event_loop, error),
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self
            .benchmark
            .as_ref()
            .is_some_and(Benchmark::pending_timed_out)
        {
            self.fail(event_loop, "benchmark edit timed out after five seconds");
            return;
        }
        if self.last_heartbeat.elapsed() < HEARTBEAT_INTERVAL {
            event_loop.set_control_flow(idle_control_flow(
                self.benchmark.is_some(),
                self.last_heartbeat + HEARTBEAT_INTERVAL,
            ));
            return;
        }
        self.last_heartbeat = Instant::now();
        self.heartbeat_sequence = self.heartbeat_sequence.saturating_add(1);
        let health = self
            .core
            .as_mut()
            .map(|core| core.heartbeat(self.heartbeat_sequence));
        match health {
            Some(CoreHealth::Unhealthy(error)) => self.show_degraded(event_loop, &error),
            Some(CoreHealth::RestartRequired(error)) => {
                self.show_degraded(event_loop, &error);
                self.version = 0;
                self.text.clear();
                if let Some(Err(restart_error)) = self.core.as_mut().map(CoreSupervisor::restart) {
                    self.text = format!("Core disabled: {restart_error}");
                    if let Some(renderer) = self.renderer.as_mut() {
                        if let Err(render_error) = renderer.set_text(&self.text) {
                            self.handle_render_error(event_loop, render_error);
                        }
                    }
                }
            }
            Some(CoreHealth::Healthy | CoreHealth::Missed) | None => {}
        }
        event_loop.set_control_flow(idle_control_flow(
            self.benchmark.is_some(),
            self.last_heartbeat + HEARTBEAT_INTERVAL,
        ));
    }
}

impl App {
    fn show_degraded(&mut self, event_loop: &ActiveEventLoop, error: &str) {
        self.text = format!("Core degraded: {error}");
        if let Some(renderer) = self.renderer.as_mut() {
            if let Err(render_error) = renderer.set_text(&self.text) {
                self.handle_render_error(event_loop, render_error);
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn after_present(&mut self, event_loop: &ActiveEventLoop, frame: nvide_render::PresentedFrame) {
        let action = self
            .benchmark
            .as_mut()
            .map(|benchmark| benchmark.presented(frame));
        match action {
            Some(BenchmarkAction::Continue) => {
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Some(BenchmarkAction::DispatchEdit) => {
                self.dispatch_benchmark_edit(event_loop);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Some(BenchmarkAction::Finish) => {
                if let Some(benchmark) = self.benchmark.as_ref() {
                    if let Err(error) = benchmark.write_artifact() {
                        self.failure = Some(error.to_string());
                    } else if let Some(failure) = benchmark.failure_message() {
                        self.failure = Some(failure.to_owned());
                    }
                }
                event_loop.exit();
            }
            None => {}
        }
    }

    fn dispatch_benchmark_edit(&mut self, event_loop: &ActiveEventLoop) {
        let Some((trace_id, character, measured)) =
            self.benchmark.as_mut().and_then(Benchmark::next_edit)
        else {
            return;
        };
        let dispatch_ns = match nvide_platform::monotonic_ns() {
            Ok(timestamp) => timestamp,
            Err(error) => return self.fail(event_loop, error),
        };
        if let Some(benchmark) = self.benchmark.as_mut() {
            benchmark.edit_dispatched(trace_id, character, measured, dispatch_ns);
        }
        let Some(core) = self.core.as_mut() else {
            return self.fail(event_loop, "benchmark edit requires the core process");
        };
        let request = schema::EditRequest {
            trace_id,
            expected_version: self.version,
            char_offset: self.text.chars().count() as u64,
            text: character.to_string(),
            dispatch_ns,
        };
        match core.edit(&request) {
            Ok(viewport)
                if viewport.trace_id == trace_id && viewport.version == self.version + 1 =>
            {
                self.version = viewport.version;
                let viewport_receive_ns = match nvide_platform::monotonic_ns() {
                    Ok(timestamp) => timestamp,
                    Err(error) => return self.fail(event_loop, error),
                };
                if let Some(benchmark) = self.benchmark.as_mut() {
                    benchmark.edit_received(trace_id, self.version, &viewport, viewport_receive_ns);
                }
                self.text = viewport.text;
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.set_text(&self.text) {
                        self.handle_render_error(event_loop, error);
                    } else if let Some(benchmark) = self.benchmark.as_mut() {
                        benchmark.shaped(trace_id, renderer.glyph_count(), &self.text);
                    }
                }
            }
            Ok(_) => self.fail(event_loop, "benchmark received a stale viewport"),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn apply_text(&mut self, event_loop: &ActiveEventLoop, text: &str) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        self.trace_id = self.trace_id.saturating_add(1);
        let request = schema::EditRequest {
            trace_id: self.trace_id,
            expected_version: self.version,
            char_offset: self.text.chars().count() as u64,
            text: text.to_owned(),
            dispatch_ns: match nvide_platform::monotonic_ns() {
                Ok(timestamp) => timestamp,
                Err(error) => return self.fail(event_loop, error),
            },
        };
        match core.edit(&request) {
            Ok(viewport)
                if viewport.trace_id == self.trace_id && viewport.version == self.version + 1 =>
            {
                self.version = viewport.version;
                self.text = viewport.text;
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.set_text(&self.text) {
                        self.handle_render_error(event_loop, error);
                    }
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Ok(_) => self.fail(event_loop, "core returned a stale viewport"),
            Err(error) => {
                self.text = format!("Core edit failed: {error}");
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(render_error) = renderer.set_text(&self.text) {
                        self.handle_render_error(event_loop, render_error);
                    }
                }
            }
        }
    }

    fn handle_render_error(
        &mut self,
        event_loop: &ActiveEventLoop,
        error: nvide_render::RenderError,
    ) {
        match error {
            nvide_render::RenderError::Timeout if self.benchmark.is_some() => {
                self.fail(event_loop, "benchmark render surface timed out");
            }
            nvide_render::RenderError::Timeout => {}
            nvide_render::RenderError::SurfaceLost => {
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    if let Some(renderer) = self.renderer.as_mut() {
                        if let Err(error) = renderer.resize(size.width, size.height) {
                            self.fail(event_loop, error);
                        }
                    }
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            fatal => self.fail(event_loop, fatal),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl fmt::Display) {
        let message = error.to_string();
        if let Some(benchmark) = self.benchmark.as_mut() {
            benchmark.record_failure(&message);
            if let Err(write_error) = benchmark.write_artifact() {
                self.failure = Some(format!("{message}; artifact write failed: {write_error}"));
                event_loop.exit();
                return;
            }
        }
        self.failure = Some(message);
        event_loop.exit();
    }
}

struct RunOptions {
    benchmark: Option<Benchmark>,
}

impl RunOptions {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, UiError> {
        let args = args.collect::<Vec<_>>();
        let Some(index) = args
            .iter()
            .position(|argument| argument == "--phase0-benchmark")
        else {
            return Ok(Self { benchmark: None });
        };
        let kind = args
            .get(index + 1)
            .ok_or_else(|| UiError("missing benchmark kind".to_owned()))?;
        let run_id = value(&args, "--run-id")?;
        let output = PathBuf::from(value(&args, "--output")?);
        let benchmark = match kind.as_str() {
            "clear" => Benchmark::Clear {
                run_id,
                output,
                started: None,
                started_ns: None,
                warmup: Duration::from_secs(number(&args, "--warmup-seconds")?),
                measure: Duration::from_secs(number(&args, "--measure-seconds")?),
                frames: Vec::new(),
                failure: None,
            },
            "edit" => {
                let warmup_edits = count(&args, "--warmup-edits")?;
                let measure_edits = count(&args, "--measure-edits")?;
                if warmup_edits.saturating_add(measure_edits) > EDIT_SENTINELS.len() {
                    return Err(UiError(format!(
                        "edit benchmark supports at most {} unique sentinels",
                        EDIT_SENTINELS.len()
                    )));
                }
                Benchmark::Edit {
                    run_id,
                    output,
                    started: None,
                    started_ns: None,
                    warmup_edits,
                    measure_edits,
                    dispatched: 0,
                    pending: None,
                    traces: Vec::new(),
                    failure: None,
                }
            }
            _ => return Err(UiError(format!("unknown benchmark kind {kind}"))),
        };
        Ok(Self {
            benchmark: Some(benchmark),
        })
    }
}

enum Benchmark {
    Clear {
        run_id: String,
        output: PathBuf,
        started: Option<Instant>,
        started_ns: Option<u64>,
        warmup: Duration,
        measure: Duration,
        frames: Vec<ClearFrame>,
        failure: Option<String>,
    },
    Edit {
        run_id: String,
        output: PathBuf,
        started: Option<Instant>,
        started_ns: Option<u64>,
        warmup_edits: usize,
        measure_edits: usize,
        dispatched: usize,
        pending: Option<Trace>,
        traces: Vec<Trace>,
        failure: Option<String>,
    },
}

struct ClearFrame {
    sequence: u64,
    present_ns: u64,
}

struct Trace {
    started: Instant,
    trace_id: u64,
    version: Option<u64>,
    measured: bool,
    sentinel: char,
    dispatch_ns: u64,
    core_received_ns: Option<u64>,
    version_increment_ns: Option<u64>,
    viewport_emit_ns: Option<u64>,
    viewport_receive_ns: Option<u64>,
    shaped_glyphs: Option<usize>,
    sentinel_shaped: bool,
    frame_sequence: Option<u64>,
    present_ns: Option<u64>,
    readback: Option<nvide_render::FrameReadback>,
}

enum BenchmarkAction {
    Continue,
    DispatchEdit,
    Finish,
}

impl Benchmark {
    fn start(&mut self) -> Result<(), UiError> {
        let started_ns = nvide_platform::monotonic_ns().map_err(display_error)?;
        match self {
            Self::Clear {
                started,
                started_ns: timestamp,
                ..
            }
            | Self::Edit {
                started,
                started_ns: timestamp,
                ..
            } => {
                *started = Some(Instant::now());
                *timestamp = Some(started_ns);
            }
        }
        Ok(())
    }

    fn needs_readback(&self) -> bool {
        matches!(
            self,
            Self::Edit {
                pending: Some(_),
                ..
            }
        )
    }

    fn pending_timed_out(&self) -> bool {
        matches!(self, Self::Edit { pending: Some(trace), .. } if trace.started.elapsed() >= Duration::from_secs(5))
    }

    fn presented(&mut self, mut frame: nvide_render::PresentedFrame) -> BenchmarkAction {
        match self {
            Self::Clear {
                started,
                warmup,
                measure,
                frames,
                ..
            } => {
                let elapsed = started.map(|start| start.elapsed()).unwrap_or_default();
                if elapsed >= *warmup && elapsed < *warmup + *measure {
                    frames.push(ClearFrame {
                        sequence: frame.sequence,
                        present_ns: frame.present_ns,
                    });
                }
                if elapsed >= *warmup + *measure {
                    BenchmarkAction::Finish
                } else {
                    BenchmarkAction::Continue
                }
            }
            Self::Edit {
                warmup_edits,
                measure_edits,
                dispatched,
                pending,
                traces,
                failure,
                ..
            } => {
                if let Some(mut trace) = pending.take() {
                    trace.frame_sequence = Some(frame.sequence);
                    trace.present_ns = Some(frame.present_ns);
                    trace.readback = frame.readback.take();
                    if !trace.sentinel_shaped || trace.readback.is_none() {
                        *failure = Some(format!(
                            "trace {} lacks shaped sentinel or frame readback",
                            trace.trace_id
                        ));
                    }
                    if trace.measured {
                        traces.push(trace);
                    }
                }
                if *dispatched < *warmup_edits + *measure_edits {
                    BenchmarkAction::DispatchEdit
                } else {
                    BenchmarkAction::Finish
                }
            }
        }
    }

    fn next_edit(&mut self) -> Option<(u64, char, bool)> {
        let Self::Edit {
            warmup_edits,
            measure_edits,
            dispatched,
            ..
        } = self
        else {
            return None;
        };
        if *dispatched >= *warmup_edits + *measure_edits {
            return None;
        }
        let index = *dispatched;
        *dispatched += 1;
        let character = char::from(EDIT_SENTINELS[index]);
        Some((*dispatched as u64, character, index >= *warmup_edits))
    }

    fn edit_dispatched(&mut self, trace_id: u64, sentinel: char, measured: bool, dispatch_ns: u64) {
        if let Self::Edit { pending, .. } = self {
            *pending = Some(Trace {
                started: Instant::now(),
                trace_id,
                version: None,
                measured,
                sentinel,
                dispatch_ns,
                core_received_ns: None,
                version_increment_ns: None,
                viewport_emit_ns: None,
                viewport_receive_ns: None,
                shaped_glyphs: None,
                sentinel_shaped: false,
                frame_sequence: None,
                present_ns: None,
                readback: None,
            });
        }
    }

    fn edit_received(
        &mut self,
        trace_id: u64,
        version: u64,
        viewport: &schema::ViewportSnapshot,
        viewport_receive_ns: u64,
    ) {
        if let Self::Edit {
            pending, failure, ..
        } = self
        {
            let Some(trace) = pending.as_mut().filter(|trace| trace.trace_id == trace_id) else {
                *failure = Some(format!("trace {trace_id} has no matching dispatch"));
                return;
            };
            trace.version = Some(version);
            trace.core_received_ns = Some(viewport.core_received_ns);
            trace.version_increment_ns = Some(viewport.version_increment_ns);
            trace.viewport_emit_ns = Some(viewport.viewport_emit_ns);
            trace.viewport_receive_ns = Some(viewport_receive_ns);
            if !(trace.dispatch_ns <= viewport.core_received_ns
                && viewport.core_received_ns <= viewport.version_increment_ns
                && viewport.version_increment_ns <= viewport.viewport_emit_ns
                && viewport.viewport_emit_ns <= viewport_receive_ns)
            {
                *failure = Some(format!("trace {trace_id} has unordered timestamps"));
            }
        }
    }

    fn shaped(&mut self, trace_id: u64, glyphs: usize, text: &str) {
        if let Self::Edit { pending, .. } = self {
            if let Some(trace) = pending.as_mut().filter(|trace| trace.trace_id == trace_id) {
                trace.shaped_glyphs = Some(glyphs);
                trace.sentinel_shaped = glyphs > 0 && text.contains(trace.sentinel);
            }
        }
    }

    fn record_failure(&mut self, message: &str) {
        let failure = match self {
            Self::Clear { failure, .. } | Self::Edit { failure, .. } => failure,
        };
        *failure = Some(message.replace(['\n', '\r'], " "));
    }

    fn failure_message(&self) -> Option<&str> {
        match self {
            Self::Clear { failure, .. } | Self::Edit { failure, .. } => failure.as_deref(),
        }
    }

    fn write_artifact(&self) -> Result<(), UiError> {
        let (run_id, output_directory) = match self {
            Self::Clear { run_id, output, .. } | Self::Edit { run_id, output, .. } => {
                (run_id, output)
            }
        };
        fs::create_dir_all(output_directory).map_err(display_error)?;
        let failure = match self {
            Self::Clear { failure, .. } | Self::Edit { failure, .. } => failure,
        };
        let mut manifest = format!(
            "format=nvide-phase0-runtime-v2\nrun_id={run_id}\nstatus={}\npid={}\n",
            if failure.is_some() {
                "FAILED_DIAGNOSTIC"
            } else {
                "UNBOUND_DIAGNOSTIC"
            },
            std::process::id()
        );
        if let Some(failure) = failure {
            manifest.push_str(&format!("failure={failure}\n"));
        }
        let runtime = match self {
            Self::Clear {
                frames,
                started_ns,
                warmup,
                measure,
                ..
            } => {
                manifest.push_str("kind=clear\n");
                if let Some(started_ns) = started_ns {
                    let warmup_ns = warmup.as_nanos().min(u128::from(u64::MAX)) as u64;
                    let measure_ns = measure.as_nanos().min(u128::from(u64::MAX)) as u64;
                    let measurement_start = started_ns.saturating_add(warmup_ns);
                    manifest.push_str(&format!(
                        "measurement_start_ns={measurement_start}\nmeasurement_end_ns={}\n",
                        measurement_start.saturating_add(measure_ns)
                    ));
                }
                let mut output = "frame_sequence,present_call_ns\n".to_owned();
                for frame in frames {
                    output.push_str(&format!("{},{}\n", frame.sequence, frame.present_ns));
                }
                output
            }
            Self::Edit {
                traces,
                pending,
                started_ns,
                ..
            } => {
                manifest.push_str("kind=edit\n");
                if let Some(started_ns) = started_ns {
                    manifest.push_str(&format!("run_start_ns={started_ns}\n"));
                }
                let mut output = "trace_id,version,measured,sentinel,dispatch_ns,core_received_ns,version_increment_ns,viewport_emit_ns,viewport_receive_ns,shaped_glyphs,sentinel_shaped,frame_sequence,present_call_ns,readback\n".to_owned();
                for trace in traces.iter().chain(pending.iter()) {
                    let readback_name = trace
                        .frame_sequence
                        .filter(|_| trace.readback.is_some())
                        .map(|sequence| format!("frame-{sequence}.rgba"))
                        .unwrap_or_default();
                    output.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                        trace.trace_id,
                        optional(trace.version),
                        trace.measured,
                        trace.sentinel,
                        trace.dispatch_ns,
                        optional(trace.core_received_ns),
                        optional(trace.version_increment_ns),
                        optional(trace.viewport_emit_ns),
                        optional(trace.viewport_receive_ns),
                        optional(trace.shaped_glyphs),
                        trace.sentinel_shaped,
                        optional(trace.frame_sequence),
                        optional(trace.present_ns),
                        readback_name
                    ));
                    if let (Some(readback), Some(sequence)) =
                        (trace.readback.as_ref(), trace.frame_sequence)
                    {
                        fs::write(
                            output_directory.join(format!("frame-{sequence}.rgba")),
                            &readback.rgba,
                        )
                        .map_err(display_error)?;
                        manifest.push_str(&format!(
                            "frame_{sequence}_readback={}x{}-rgba8\n",
                            readback.width, readback.height
                        ));
                    }
                }
                output
            }
        };
        fs::write(output_directory.join("manifest.txt"), manifest).map_err(display_error)?;
        fs::write(output_directory.join("runtime.csv"), runtime).map_err(display_error)?;
        Ok(())
    }
}

fn optional(value: Option<impl fmt::Display>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn value(args: &[String], flag: &str) -> Result<String, UiError> {
    let index = args
        .iter()
        .position(|argument| argument == flag)
        .ok_or_else(|| UiError(format!("missing {flag}")))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| UiError(format!("missing value for {flag}")))
}

fn number(args: &[String], flag: &str) -> Result<u64, UiError> {
    value(args, flag)?
        .parse()
        .map_err(|_| UiError(format!("invalid integer for {flag}")))
}

fn count(args: &[String], flag: &str) -> Result<usize, UiError> {
    usize::try_from(number(args, flag)?).map_err(|_| UiError(format!("{flag} is too large")))
}

struct CoreSupervisor {
    command: CoreCommand,
    endpoint: String,
    listener: nvide_ipc::LocalListener,
    child: Child,
    client: nvide_ipc::Client<nvide_ipc::LocalStream>,
    restart_budget: RestartBudget,
    missed_heartbeats: u8,
    last_healthy: Instant,
}

impl CoreSupervisor {
    fn start(command: CoreCommand) -> Result<Self, UiError> {
        let endpoint = local_endpoint()?;
        let listener = nvide_ipc::LocalListener::bind(&endpoint).map_err(display_error)?;
        let (child, client) = spawn_core(&command, &endpoint, &listener)?;
        Ok(Self {
            command,
            endpoint,
            listener,
            child,
            client,
            restart_budget: RestartBudget::default(),
            missed_heartbeats: 0,
            last_healthy: Instant::now(),
        })
    }

    fn edit(
        &mut self,
        request: &schema::EditRequest,
    ) -> Result<schema::ViewportSnapshot, nvide_ipc::ProtocolError> {
        self.client.edit(request)
    }

    fn heartbeat(&mut self, sequence: u64) -> CoreHealth {
        let result = match self.child.try_wait() {
            Ok(Some(_)) => Err(UiError("core process exited".to_owned())),
            Ok(None) => self.client.heartbeat(sequence).map_err(display_error),
            Err(error) => Err(display_error(error)),
        };
        match result {
            Ok(()) => {
                self.missed_heartbeats = 0;
                self.last_healthy = Instant::now();
                CoreHealth::Healthy
            }
            Err(error) => {
                self.missed_heartbeats = self.missed_heartbeats.saturating_add(1);
                failed_core_health(
                    self.missed_heartbeats,
                    self.last_healthy.elapsed(),
                    error.to_string(),
                )
            }
        }
    }

    fn restart(&mut self) -> Result<(), UiError> {
        let now = Instant::now();
        let backoff = self.restart_budget.record(now)?;
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::thread::sleep(backoff);
        let (child, client) = spawn_core(&self.command, &self.endpoint, &self.listener)?;
        self.child = child;
        self.client = client;
        self.missed_heartbeats = 0;
        self.last_healthy = Instant::now();
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
enum CoreHealth {
    Healthy,
    Missed,
    Unhealthy(String),
    RestartRequired(String),
}

fn failed_core_health(missed: u8, elapsed: Duration, error: String) -> CoreHealth {
    if elapsed >= Duration::from_secs(5) {
        CoreHealth::RestartRequired(error)
    } else if missed >= 3 {
        CoreHealth::Unhealthy(error)
    } else {
        CoreHealth::Missed
    }
}

#[derive(Default)]
struct RestartBudget {
    crashes: VecDeque<Instant>,
}

impl RestartBudget {
    fn record(&mut self, now: Instant) -> Result<Duration, UiError> {
        while self
            .crashes
            .front()
            .is_some_and(|restart| now.duration_since(*restart) > RESTART_WINDOW)
        {
            self.crashes.pop_front();
        }
        if self.crashes.len() >= MAX_RESTARTS {
            return Err(UiError(format!(
                "restart budget exhausted ({MAX_RESTARTS} crashes in {}s)",
                RESTART_WINDOW.as_secs()
            )));
        }
        let backoff = Duration::from_millis(100 * (1_u64 << self.crashes.len().min(4)));
        self.crashes.push_back(now);
        Ok(backoff)
    }
}

impl Drop for CoreSupervisor {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_core(
    command: &CoreCommand,
    endpoint: &str,
    listener: &nvide_ipc::LocalListener,
) -> Result<(Child, nvide_ipc::Client<nvide_ipc::LocalStream>), UiError> {
    let mut process = Command::new(&command.executable);
    process.args(&command.arguments);
    if command.endpoint_via_env {
        process.env(nvide_platform::NRPC_ENDPOINT_ENV, endpoint);
    } else {
        process.arg("--endpoint").arg(endpoint);
    }
    let mut child = process
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(display_error)?;
    let stream = match listener.accept() {
        Ok(stream) => stream,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(display_error(error));
        }
    };
    let client = match nvide_ipc::Client::connect(stream, schema::Role::Ui) {
        Ok(client) => client,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(display_error(error));
        }
    };
    Ok((child, client))
}

#[derive(Clone)]
struct CoreCommand {
    executable: PathBuf,
    arguments: Vec<String>,
    endpoint_via_env: bool,
}

fn core_executable() -> Result<CoreCommand, UiError> {
    if let Some(path) = env::var_os("NVIDE_CORE_BIN") {
        return Ok(CoreCommand {
            executable: path.into(),
            arguments: Vec::new(),
            endpoint_via_env: false,
        });
    }
    let current = env::current_exe().map_err(display_error)?;
    let directory = current
        .parent()
        .ok_or_else(|| UiError("NVide executable has no parent directory".to_owned()))?;
    let core = directory.join(if cfg!(windows) {
        "nvide-core.exe"
    } else {
        "nvide-core"
    });
    if core.is_file() {
        Ok(CoreCommand {
            executable: core,
            arguments: Vec::new(),
            endpoint_via_env: false,
        })
    } else {
        Err(UiError(format!(
            "nvide-core was not found beside {}",
            current.display()
        )))
    }
}

fn local_endpoint() -> Result<String, UiError> {
    static NEXT_ENDPOINT: AtomicU64 = AtomicU64::new(1);
    let name = format!(
        "nvide-{}-{}",
        std::process::id(),
        NEXT_ENDPOINT.fetch_add(1, Ordering::Relaxed)
    );
    if cfg!(windows) {
        Ok(name)
    } else {
        env::temp_dir()
            .join(format!("{name}.sock"))
            .into_os_string()
            .into_string()
            .map_err(|_| UiError("temporary IPC path is not UTF-8".to_owned()))
    }
}

fn display_error(error: impl fmt::Display) -> UiError {
    UiError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_command(test_name: &str, ignored: bool) -> Result<CoreCommand, UiError> {
        let mut arguments = Vec::new();
        if ignored {
            arguments.push("--ignored".to_owned());
        }
        arguments.extend([
            "--exact".to_owned(),
            test_name.to_owned(),
            "--nocapture".to_owned(),
        ]);
        Ok(CoreCommand {
            executable: env::current_exe().map_err(display_error)?,
            arguments,
            endpoint_via_env: true,
        })
    }

    #[test]
    #[ignore]
    fn phase0_supervisor_core_fixture() -> Result<(), Box<dyn Error>> {
        let endpoint = env::var(nvide_platform::NRPC_ENDPOINT_ENV)?;
        let stream = nvide_ipc::LocalStream::connect(&endpoint)?;
        let mut version = 0_u64;
        let mut text = String::new();
        nvide_ipc::serve(stream, move |request| {
            let core_received_ns =
                nvide_ipc::platform_monotonic_ns().map_err(|error| schema::RpcError {
                    code: schema::ErrorCode::Internal,
                    message: error.to_string(),
                })?;
            if request.expected_version != version {
                return Err(schema::RpcError {
                    code: schema::ErrorCode::InvalidArgument,
                    message: "stale fixture version".to_owned(),
                });
            }
            version = version.saturating_add(1);
            text.push_str(&request.text);
            let version_increment_ns =
                nvide_ipc::platform_monotonic_ns().map_err(|error| schema::RpcError {
                    code: schema::ErrorCode::Internal,
                    message: error.to_string(),
                })?;
            Ok(schema::ViewportSnapshot {
                trace_id: request.trace_id,
                version,
                text: text.clone(),
                core_received_ns,
                version_increment_ns,
                viewport_emit_ns: nvide_ipc::platform_monotonic_ns().map_err(|error| {
                    schema::RpcError {
                        code: schema::ErrorCode::Internal,
                        message: error.to_string(),
                    }
                })?,
            })
        })?;
        Ok(())
    }

    #[test]
    fn restart_budget_exhausts_and_drops_old_crashes() -> Result<(), UiError> {
        let now = Instant::now();
        let mut budget = RestartBudget::default();
        for second in 0..MAX_RESTARTS {
            budget.record(now + Duration::from_secs(second as u64))?;
        }
        assert!(budget.record(now + Duration::from_secs(4)).is_err());
        budget.record(now + RESTART_WINDOW + Duration::from_secs(MAX_RESTARTS as u64))?;
        assert_eq!(budget.crashes.len(), 1);
        Ok(())
    }

    #[test]
    fn heartbeat_policy_degrades_then_restarts() {
        assert_eq!(
            failed_core_health(2, Duration::from_secs(2), "missed".to_owned()),
            CoreHealth::Missed
        );
        assert_eq!(
            failed_core_health(3, Duration::from_secs(3), "missed".to_owned()),
            CoreHealth::Unhealthy("missed".to_owned())
        );
        assert_eq!(
            failed_core_health(5, Duration::from_secs(5), "missed".to_owned()),
            CoreHealth::RestartRequired("missed".to_owned())
        );
    }

    #[test]
    fn idle_ui_schedules_heartbeat_wakeup() {
        assert!(matches!(
            idle_control_flow(false, Instant::now() + HEARTBEAT_INTERVAL),
            ControlFlow::WaitUntil(_)
        ));
        assert_eq!(idle_control_flow(true, Instant::now()), ControlFlow::Poll);
    }

    #[test]
    fn supervisor_restarts_rebinds_and_exhausts_real_budget() -> Result<(), Box<dyn Error>> {
        let command = fixture_command("tests::phase0_supervisor_core_fixture", true)?;
        let mut supervisor = CoreSupervisor::start(command)?;
        assert_eq!(supervisor.heartbeat(1), CoreHealth::Healthy);

        supervisor.child.kill()?;
        supervisor.child.wait()?;
        supervisor.last_healthy = Instant::now() - Duration::from_secs(5);
        assert!(matches!(
            supervisor.heartbeat(2),
            CoreHealth::RestartRequired(_)
        ));
        supervisor.restart()?;
        assert_eq!(supervisor.heartbeat(3), CoreHealth::Healthy);

        let dispatch_ns = nvide_ipc::platform_monotonic_ns()?;
        let viewport = supervisor.edit(&schema::EditRequest {
            trace_id: 9,
            expected_version: 0,
            char_offset: 0,
            text: "restart-ok".to_owned(),
            dispatch_ns,
        })?;
        assert_eq!(
            (viewport.version, viewport.text.as_str()),
            (1, "restart-ok")
        );

        for _ in 0..2 {
            supervisor.child.kill()?;
            supervisor.child.wait()?;
            supervisor.restart()?;
        }
        supervisor.child.kill()?;
        supervisor.child.wait()?;
        assert!(supervisor.restart().is_err());
        Ok(())
    }

    #[test]
    fn child_exit_before_connect_is_reported() -> Result<(), UiError> {
        let command = fixture_command("tests::no_such_test", false)?;
        let started = Instant::now();
        let error = CoreSupervisor::start(command)
            .err()
            .ok_or_else(|| UiError("fixture unexpectedly connected".to_owned()))?;
        assert!(error.to_string().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(7));
        Ok(())
    }
}
