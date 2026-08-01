# Phase 0 implementation re-review — reviewer slot 1

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Independent implementation reviewer, slot 1
- Author and implementer principal: `agent:/root`
- Reviewed commit: `4a34c83eee36e6d5cfa1bd61d85f26da669ee065`
- UTC date: 2026-08-01
- Overall verdict: **CHANGES REQUIRED**
- Scope: Phase 0 requirements `P0-R1`…`P0-R7`, acceptance `P0-A1`…`P0-A5`, ADR-0002/0003/0005/0020/0021/0022/0023, the findings from both reviews of `745b098`, and evidence `P0-E1`…`P0-E6`.

The source was inspected and tested in a detached worktree at the exact reviewed commit. This reviewer did not author or implement the reviewed change and fills only implementation-reviewer slot 1; this is not a P0.2 exit-binding or Phase 0 exit approval.

## Prior findings re-evaluated

The following prior defects are fixed: the `nvide` executable and allowed crate DAG are thin; the idle event loop schedules heartbeats; the real supervisor test exercises child failure, restart/rebind, post-restart traffic, budget exhaustion, and pre-connect exit; Windows named pipes use bounded nonblocking I/O and reject remote clients; `Buffer::slice`, sequential generated cases, and expanded buffer fuzzing exist; duplicate terminal NRPC responses are rejected; CI checks MSRV on all four targets; the runtime trace fields, raw frame readback, typed device-loss path, partial artifact machinery, and exact Cap'n Proto version check are present. The amended P0.2 release command has two independent approval artifacts.

## Remaining findings

### F1 — HIGH — incomplete NRPC frames have per-read, not aggregate, deadlines

ADR-0020 requires a five-second incomplete-frame timeout. `read_frame` performs separate reads for the first byte, remaining header, and payload (`crates/nvide-ipc/src/lib.rs:148-176`). Unix applies five seconds to each socket read (`crates/nvide-platform/src/lib.rs:70-80`), while Windows creates a fresh five-second deadline for every `Read::read` invocation (`crates/nvide-platform/src/lib.rs:272-293`). A peer that supplies another byte before each individual timeout can keep one incomplete frame alive indefinitely. Enforce one deadline from the first byte through the complete frame and add a slow-drip transport test. This blocks `P0-R5`, `P0-A4`, and `P0-E4`.

### F2 — HIGH — the edit workload advances before compositor display

P0.2 requires each measured edit to reach an actually displayed compositor event before the next edit is dispatched. The renderer returns after `SurfaceTexture::present` and readback (`crates/nvide-render/src/lib.rs:434-447`); `Benchmark::presented` immediately selects `DispatchEdit` (`crates/nvide-ui/src/lib.rs:558-607`), and `App::after_present` dispatches it (`crates/nvide-ui/src/lib.rs:246-260`). There is no displayed-event acknowledgement in this state transition, so later joining compositor logs cannot restore the required workload ordering.

The sentinel proof is also incomplete: `sentinel_shaped` combines total glyph count with source-text containment (`crates/nvide-ui/src/lib.rs:683-688`), and the artifact writer saves raw bytes without proving the sentinel pixels or embedding a verifiable frame-sequence marker (`crates/nvide-ui/src/lib.rs:761-797`). These diagnostics do not yet satisfy the approved per-sentinel shaped/readback requirement.

Finally, the five-second watchdog runs only from `about_to_wait`, while readback calls blocking `device.poll(wgpu::Maintain::Wait)` and `recv()` (`crates/nvide-render/src/lib.rs:507-517`). A stuck mapping/device can prevent the event loop from running the watchdog and can hang instead of writing the required partial bundle. These issues block implementation agreement for `P0-R7`, `P0-A5`, and `P0-E6`.

### F3 — MEDIUM — forced-failure evidence does not prove the degraded UI state

The application does render `Core degraded: ...` for unhealthy/restart-required states (`crates/nvide-ui/src/lib.rs:205-244`), but the subprocess test ages `last_healthy`, checks `CoreSupervisor` directly, and calls `restart` directly (`crates/nvide-ui/src/lib.rs:1155-1192`). It never drives the application path through the user-visible degraded state before restart. Add the smallest executable application-state test or immutable artifact proving that transition. This leaves `P0-E5` incomplete.

## Evidence verdicts

| Evidence | Verdict | Reason |
| --- | --- | --- |
| `P0-E1` | **AGREE — LOCAL IMPLEMENTATION** | Exact eight-crate/DAG enforcement and all four local Rust 1.82 target checks pass; hosted native job artifacts remain an exit-ledger concern. |
| `P0-E2` | **AGREE — LOCAL IMPLEMENTATION** | Clean bootstrap builds exact Cap'n Proto 1.5.0, regenerates byte-identical output, and leaves the tracked tree clean. |
| `P0-E3` | **AGREE — LOCAL IMPLEMENTATION** | Slice/UTF-8/line/version/atomic sequential edits, branching undo/redo, generated checks, and expanded fuzzing pass. |
| `P0-E4` | **CHANGES REQUIRED** | Framing/handshake/lifecycle/subprocess/fuzz and platform-specific fixes pass, but an incomplete frame has no aggregate five-second deadline. |
| `P0-E5` | **CHANGES REQUIRED** | Scheduling, real restart/rebind, pre-connect failure, and budget exhaustion pass; executable user-visible degraded-state evidence is absent. |
| `P0-E6` | **BLOCKED / CHANGES REQUIRED** | Runtime hooks are improved, but measured-edit display ordering, verifiable sentinel readback, and non-hanging readback timeout are incomplete. The eligible native 120 Hz binding and its two approvals are also pending. |

## Commands run

All commands below targeted the detached reviewed commit.

```text
cargo fmt --all -- --check
cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.82.0 test --workspace --all-targets --all-features --locked
cargo xtask evidence check --phase 0
# PASS; 30 tests passed and one subprocess fixture remained intentionally ignored as a direct test

cargo xtask schema check
git diff --exit-code
# PASS; clean bootstrap fetched, hash-checked, built, and verified exact Cap'n Proto 1.5.0

cargo +1.82.0 check --workspace --all-targets --all-features --locked --target <target>
# PASS for Linux x64, Windows x64, macOS x64, and macOS arm64

# Exact package/allowed-edge script embedded in .github/workflows/ci.yml
# PASS

cargo +nightly-2026-07-01 fuzz run buffer -- -runs=1000
cargo +nightly-2026-07-01 fuzz run nrpc -- -runs=1000
# PASS

git diff --check
# PASS before adding this review artifact
```

No eligible P0.2 display run was performed or inferred. `P0-E6` and Phase 0 exit remain **BLOCKED** regardless of the local diagnostic results.
