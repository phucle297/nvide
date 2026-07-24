//! Thin UI binary: winit + wgpu clear path and monospaced glyph prototype.

mod app;

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use clap::{Parser, Subcommand};
use nvide_rpc_schema::ApplyEdit;

#[derive(Parser, Debug)]
#[command(name = "nvide", about = "NVide — native Rust IDE (Phase 0 shell)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open the GPU window prototype (clear + typed glyphs into a rope buffer).
    Ui {
        /// Initial buffer text.
        #[arg(long, default_value = "")]
        text: String,
        /// Auto-close after N frames (0 = run until closed). Useful for smoke tests.
        #[arg(long, default_value_t = 0)]
        max_frames: u32,
    },
    /// Multi-process edit roundtrip: spawn core, send ApplyEdit, print result text.
    NrpcRoundtrip {
        /// Text to insert at position 0.
        #[arg(long, default_value = "hello-nrpc")]
        text: String,
        /// Optional path to nvide-core binary (defaults to same-directory / PATH / cargo).
        #[arg(long)]
        core_bin: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Ui {
        text: String::new(),
        max_frames: 0,
    }) {
        Commands::Ui { text, max_frames } => {
            app::run_ui(text, max_frames)?;
        }
        Commands::NrpcRoundtrip { text, core_bin } => {
            run_nrpc_roundtrip(&text, core_bin.as_deref())?;
        }
    }
    Ok(())
}

fn run_nrpc_roundtrip(
    text: &str,
    core_bin: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(unix))]
    {
        let _ = (text, core_bin);
        return Err("NrpcRoundtrip requires Unix in Phase 0".into());
    }
    #[cfg(unix)]
    {
        use nvide_core::client_edit_roundtrip;
        use nvide_platform::unix_socket;

        let core = resolve_core_bin(core_bin)?;
        let socket = nvide_platform::temp_ipc_path("roundtrip");
        nvide_platform::remove_ipc_path(&socket);

        let mut child = Command::new(&core)
            .args([
                "serve",
                "--socket",
                socket.to_str().ok_or("socket path utf-8")?,
                "--once",
                "--print-socket",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", core.display()))?;

        // Wait until the socket accepts connections.
        let mut stream = unix_socket::connect_with_retry(&socket, 50, Duration::from_millis(20))?;

        let result = client_edit_roundtrip(
            &mut stream,
            "nvide-ui",
            ApplyEdit {
                buffer_id: 1,
                pos: 0,
                delete_len: 0,
                insert_text: text.to_string(),
            },
        )?;
        drop(stream);

        let status = child.wait()?;
        if !status.success() {
            return Err(format!("nvide-core exited with {status}").into());
        }

        println!("roundtrip_text={}", result.text);
        println!("roundtrip_version={}", result.version);
        if result.text != text {
            return Err(format!(
                "edit roundtrip mismatch: got {:?} expected {:?}",
                result.text, text
            )
            .into());
        }
        nvide_platform::remove_ipc_path(&socket);
        Ok(())
    }
}

fn resolve_core_bin(
    explicit: Option<&std::path::Path>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    // Prefer sibling binary next to current exe (cargo run --bin layouts).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("nvide-core");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    // Fall back to cargo-built path relative to CARGO_MANIFEST_DIR.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .ok_or("workspace root")?;
    for profile in ["debug", "release"] {
        let candidate = workspace.join("target").join(profile).join("nvide-core");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Ok(PathBuf::from("nvide-core"))
}
