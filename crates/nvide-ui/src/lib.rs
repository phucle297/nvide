//! Winit-owned Phase 0 UI, input routing, and core supervision.

use nvide_ipc::schema;
use std::{
    collections::VecDeque,
    env,
    error::Error,
    fmt, fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
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
    event_loop.set_control_flow(if options.benchmark.is_some() {
        ControlFlow::Poll
    } else {
        ControlFlow::Wait
    });
    let mut app = App::new(options);
    event_loop.run_app(&mut app).map_err(display_error)?;
    match app.failure.take() {
        Some(failure) => Err(UiError(failure)),
        None => Ok(()),
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
            benchmark.start();
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
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.render() {
                        self.handle_render_error(event_loop, error);
                        return;
                    }
                }
                self.after_present(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.last_heartbeat.elapsed() < HEARTBEAT_INTERVAL {
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

    fn after_present(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let action = self
            .benchmark
            .as_mut()
            .map(|benchmark| benchmark.presented(now));
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
                    }
                }
                event_loop.exit();
            }
            None => {}
        }
    }

    fn dispatch_benchmark_edit(&mut self, event_loop: &ActiveEventLoop) {
        let Some((trace_id, character, measured, dispatch_ns)) =
            self.benchmark.as_mut().and_then(Benchmark::next_edit)
        else {
            return;
        };
        let Some(core) = self.core.as_mut() else {
            return self.fail(event_loop, "benchmark edit requires the core process");
        };
        let request = schema::EditRequest {
            trace_id,
            expected_version: self.version,
            char_offset: self.text.chars().count() as u64,
            text: character.to_string(),
        };
        match core.edit(&request) {
            Ok(viewport)
                if viewport.trace_id == trace_id && viewport.version == self.version + 1 =>
            {
                self.version = viewport.version;
                self.text = viewport.text;
                let viewport_ns = self
                    .benchmark
                    .as_ref()
                    .map(Benchmark::elapsed_ns)
                    .unwrap_or(dispatch_ns);
                if let Some(benchmark) = self.benchmark.as_mut() {
                    benchmark.edit_received(
                        trace_id,
                        self.version,
                        measured,
                        dispatch_ns,
                        viewport_ns,
                    );
                }
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.set_text(&self.text) {
                        self.handle_render_error(event_loop, error);
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
            }
            fatal => self.fail(event_loop, fatal),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl fmt::Display) {
        self.failure = Some(error.to_string());
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
                warmup: Duration::from_secs(number(&args, "--warmup-seconds")?),
                measure: Duration::from_secs(number(&args, "--measure-seconds")?),
                frames: Vec::new(),
            },
            "edit" => Benchmark::Edit {
                run_id,
                output,
                started: None,
                warmup_edits: number(&args, "--warmup-edits")? as usize,
                measure_edits: number(&args, "--measure-edits")? as usize,
                dispatched: 0,
                pending: None,
                traces: Vec::new(),
            },
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
        warmup: Duration,
        measure: Duration,
        frames: Vec<u128>,
    },
    Edit {
        run_id: String,
        output: PathBuf,
        started: Option<Instant>,
        warmup_edits: usize,
        measure_edits: usize,
        dispatched: usize,
        pending: Option<Trace>,
        traces: Vec<Trace>,
    },
}

struct Trace {
    trace_id: u64,
    version: u64,
    measured: bool,
    dispatch_ns: u128,
    viewport_ns: u128,
    present_ns: Option<u128>,
}

enum BenchmarkAction {
    Continue,
    DispatchEdit,
    Finish,
}

impl Benchmark {
    fn start(&mut self) {
        match self {
            Self::Clear { started, .. } | Self::Edit { started, .. } => {
                *started = Some(Instant::now())
            }
        }
    }

    fn elapsed_ns(&self) -> u128 {
        match self {
            Self::Clear { started, .. } | Self::Edit { started, .. } => {
                started.map(|start| start.elapsed().as_nanos()).unwrap_or(0)
            }
        }
    }

    fn presented(&mut self, now: Instant) -> BenchmarkAction {
        match self {
            Self::Clear {
                started,
                warmup,
                measure,
                frames,
                ..
            } => {
                let elapsed = started
                    .map(|start| now.duration_since(start))
                    .unwrap_or_default();
                if elapsed >= *warmup && elapsed < *warmup + *measure {
                    frames.push(elapsed.as_nanos());
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
                started,
                ..
            } => {
                if let Some(mut trace) = pending.take() {
                    trace.present_ns = started.map(|start| now.duration_since(start).as_nanos());
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

    fn next_edit(&mut self) -> Option<(u64, char, bool, u128)> {
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
        let character = char::from(b'!' + (index % 90) as u8);
        Some((
            *dispatched as u64,
            character,
            index >= *warmup_edits,
            self.elapsed_ns(),
        ))
    }

    fn edit_received(
        &mut self,
        trace_id: u64,
        version: u64,
        measured: bool,
        dispatch_ns: u128,
        viewport_ns: u128,
    ) {
        if let Self::Edit { pending, .. } = self {
            *pending = Some(Trace {
                trace_id,
                version,
                measured,
                dispatch_ns,
                viewport_ns,
                present_ns: None,
            });
        }
    }

    fn write_artifact(&self) -> Result<(), UiError> {
        let (run_id, output) = match self {
            Self::Clear { run_id, output, .. } | Self::Edit { run_id, output, .. } => {
                (run_id, output)
            }
        };
        fs::create_dir_all(output).map_err(display_error)?;
        let mut manifest = format!(
            "format=nvide-phase0-runtime-v1\nrun_id={run_id}\nstatus=UNBOUND_DIAGNOSTIC\npid={}\n",
            std::process::id()
        );
        let runtime = match self {
            Self::Clear { frames, .. } => {
                manifest.push_str("kind=clear\n");
                let mut output = "frame,present_call_ns\n".to_owned();
                for (index, timestamp) in frames.iter().enumerate() {
                    output.push_str(&format!("{},{}\n", index + 1, timestamp));
                }
                output
            }
            Self::Edit { traces, .. } => {
                manifest.push_str("kind=edit\n");
                let mut output =
                    "trace_id,version,dispatch_ns,viewport_ns,present_call_ns\n".to_owned();
                for trace in traces {
                    output.push_str(&format!(
                        "{},{},{},{},{}\n",
                        trace.trace_id,
                        trace.version,
                        trace.dispatch_ns,
                        trace.viewport_ns,
                        trace.present_ns.unwrap_or(0)
                    ));
                }
                output
            }
        };
        fs::write(output.join("manifest.txt"), manifest).map_err(display_error)?;
        fs::write(output.join("runtime.csv"), runtime).map_err(display_error)?;
        Ok(())
    }
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
    if command.self_hosted {
        process.arg("--phase0-core");
    }
    let child = process
        .arg("--endpoint")
        .arg(endpoint)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(display_error)?;
    let stream = listener.accept().map_err(display_error)?;
    let client = nvide_ipc::Client::connect(stream, schema::Role::Ui).map_err(display_error)?;
    Ok((child, client))
}

#[derive(Clone)]
struct CoreCommand {
    executable: PathBuf,
    self_hosted: bool,
}

fn core_executable() -> Result<CoreCommand, UiError> {
    if let Some(path) = env::var_os("NVIDE_CORE_BIN") {
        return Ok(CoreCommand {
            executable: path.into(),
            self_hosted: false,
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
            self_hosted: false,
        })
    } else {
        Ok(CoreCommand {
            executable: current,
            self_hosted: true,
        })
    }
}

fn local_endpoint() -> Result<String, UiError> {
    let name = format!("nvide-{}", std::process::id());
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
}
