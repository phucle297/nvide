# NVide Coding Standards

## Rust toolchain

| Item | Value |
|------|--------|
| **Target** | Rust **1.85+** stable |
| **MSRV** | **1.82** (`workspace.package.rust-version` and `clippy.toml`) |
| **Nightly** | Not used without a new ADR and sunset date |

Pin the developer toolchain with `rust-toolchain.toml` (channel 1.85.0). CI should install a matching stable toolchain on each OS runner.

## Formatting and lints

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- `rustfmt.toml` and `clippy.toml` live at the workspace root.
- Warnings are errors in CI (`-D warnings`).

## Crate boundaries (Phase 0)

- `nvide-buffer` must not depend on UI, IPC, or IDE-specific crates.
- `nvide-ipc` is pure framing/codec (+ light transport helpers); no GPU.
- UI binary (`nvide`) stays thin: winit event loop, wgpu present, NRPC client.
- Core process (`nvide-core`) owns buffers/undo and serves NRPC.

## ADRs

Material architecture choices are recorded under `docs/adr/ADR-NNNN-slug.md`.
Required sections: Context · Decision · Alternatives · Pros/Cons · Consequences · Migration · Scalability.

## Orchestration

Use `cargo xtask` for developer and CI helper tasks (schema generation, etc.).
See `cargo xtask --help`.
