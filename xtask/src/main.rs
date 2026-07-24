//! `cargo xtask` — developer and CI orchestration for NVide.
//!
//! Common commands:
//! - `cargo xtask schema-gen` — regenerate RPC schema artifacts under `schemas/`
//! - `cargo xtask --help` — list commands

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "NVide workspace orchestration")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate NRPC / settings schema documents into `schemas/`.
    SchemaGen {
        /// Workspace root (defaults to discovering from CARGO_MANIFEST_DIR).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print documented developer commands.
    Doctor,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::SchemaGen { root } => {
            let root = root.unwrap_or_else(workspace_root);
            schema_gen(&root)?;
        }
        Commands::Doctor => {
            doctor();
        }
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    // xtask lives at <root>/xtask
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask parent")
        .to_path_buf()
}

fn schema_gen(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let schemas_dir = root.join("schemas");
    fs::create_dir_all(&schemas_dir)?;

    let doc = nvide_rpc_schema::schema_document();
    let nrpc_path = schemas_dir.join("nrpc-v0.1.json");
    let pretty = serde_json::to_string_pretty(&doc)?;
    fs::write(&nrpc_path, pretty.as_bytes())?;

    // Index file for tooling discovery.
    let index = serde_json::json!({
        "generated_by": "cargo xtask schema-gen",
        "schemas": [
            {
                "id": "nrpc-v0.1",
                "path": "nrpc-v0.1.json",
                "kind": "nrpc",
                "version": {
                    "major": nvide_rpc_schema::NRPC_VERSION_MAJOR,
                    "minor": nvide_rpc_schema::NRPC_VERSION_MINOR
                }
            }
        ]
    });
    let index_path = schemas_dir.join("index.json");
    fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;

    println!("schema-gen: wrote {}", nrpc_path.display());
    println!("schema-gen: wrote {}", index_path.display());
    Ok(())
}

fn doctor() {
    println!("NVide xtask doctor");
    println!("  cargo xtask schema-gen   # regenerate schemas/ from nvide-rpc-schema");
    println!("  cargo xtask doctor       # this help");
    println!("  cargo fmt --all");
    println!("  cargo clippy --workspace --all-targets -- -D warnings");
    println!("  cargo test --workspace");
    println!("  cargo run -p nvide-core -- smoke-edit --text hi");
    println!("  cargo run -p nvide -- nrpc-roundtrip --text hello");
    println!("  cargo run -p nvide -- ui --max-frames 3");
    let root = workspace_root();
    println!("workspace_root={}", root.display());
    let _ = Command::new("rustc").arg("--version").status();
}
