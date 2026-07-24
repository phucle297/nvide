//! Standalone multi-process NRPC edit client used by verification scripts.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use clap::Parser;
use nvide_core::client_edit_roundtrip;
use nvide_platform::{remove_ipc_path, temp_ipc_path};
use nvide_rpc_schema::ApplyEdit;

#[derive(Parser, Debug)]
#[command(name = "nvide-nrpc-client")]
struct Cli {
    #[arg(long, default_value = "hello-nrpc")]
    text: String,
    #[arg(long)]
    core_bin: Option<PathBuf>,
    #[arg(long)]
    socket: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(unix))]
    {
        return Err("unix only".into());
    }
    #[cfg(unix)]
    {
        let cli = Cli::parse();
        let text = cli.text;

        let (socket, child) = if let Some(sock) = cli.socket {
            (sock, None)
        } else {
            let core = resolve_core(cli.core_bin.as_deref())?;
            let socket = temp_ipc_path("nrpc-client");
            remove_ipc_path(&socket);
            let child = Command::new(&core)
                .args(["serve", "--socket", socket.to_str().unwrap(), "--once"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::inherit())
                .spawn()?;
            (socket, Some(child))
        };

        let mut stream = nvide_platform::unix_socket::connect_with_retry(
            &socket,
            50,
            Duration::from_millis(20),
        )?;
        let result = client_edit_roundtrip(
            &mut stream,
            "nvide-nrpc-client",
            ApplyEdit {
                buffer_id: 1,
                pos: 0,
                delete_len: 0,
                insert_text: text.clone(),
            },
        )?;
        drop(stream);

        if let Some(mut child) = child {
            let _ = child.wait();
            remove_ipc_path(&socket);
        }

        println!("roundtrip_text={}", result.text);
        println!("roundtrip_version={}", result.version);
        if result.text != text {
            std::process::exit(1);
        }
        Ok(())
    }
}

fn resolve_core(explicit: Option<&std::path::Path>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let c = dir.join("nvide-core");
            if c.is_file() {
                return Ok(c);
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().and_then(|p| p.parent()).unwrap();
    for profile in ["debug", "release"] {
        let c = root.join("target").join(profile).join("nvide-core");
        if c.is_file() {
            return Ok(c);
        }
    }
    Ok(PathBuf::from("nvide-core"))
}
