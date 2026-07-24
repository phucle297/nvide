//! nvide-core process: listen on a local IPC path and serve NRPC edits.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use nvide_core::{serve_connection, CoreState};
use nvide_platform::{remove_ipc_path, temp_ipc_path};

#[derive(Parser, Debug)]
#[command(name = "nvide-core", about = "NVide core process (NRPC server)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Listen on a Unix domain socket and serve one or more clients.
    Serve {
        /// Socket path. Defaults to a unique path under the system temp dir.
        #[arg(long)]
        socket: Option<PathBuf>,
        /// Print the chosen socket path to stdout (first line) for orchestrators.
        #[arg(long, default_value_t = true)]
        print_socket: bool,
        /// Serve a single client then exit (useful for integration tests).
        #[arg(long, default_value_t = false)]
        once: bool,
    },
    /// Self-check: in-process hello + edit (no socket). Prints the resulting text.
    SmokeEdit {
        #[arg(long, default_value = "smoke")]
        text: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Serve {
            socket,
            print_socket,
            once,
        } => {
            #[cfg(unix)]
            {
                let path = socket.unwrap_or_else(|| temp_ipc_path("core"));
                let listener = nvide_platform::unix_socket::bind(&path)?;
                if print_socket {
                    println!("{}", path.display());
                }
                eprintln!("nvide-core: listening on {}", path.display());
                let mut state = CoreState::new();
                loop {
                    let (mut stream, _) = listener.accept()?;
                    if let Err(e) = serve_connection(&mut stream, &mut state) {
                        eprintln!("nvide-core: connection error: {e}");
                    }
                    if once {
                        break;
                    }
                }
                remove_ipc_path(&path);
            }
            #[cfg(not(unix))]
            {
                let _ = (socket, print_socket, once);
                return Err("nvide-core serve requires Unix domain sockets in Phase 0".into());
            }
        }
        Commands::SmokeEdit { text } => {
            let mut state = CoreState::new();
            let result = state.apply_edit(&nvide_rpc_schema::ApplyEdit {
                buffer_id: 1,
                pos: 0,
                delete_len: 0,
                insert_text: text.clone(),
            })?;
            println!("{}", result.text);
            if result.text != text {
                return Err("smoke edit mismatch".into());
            }
        }
    }
    Ok(())
}
