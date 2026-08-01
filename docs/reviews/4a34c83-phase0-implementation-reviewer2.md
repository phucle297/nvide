# Phase 0 implementation re-review — reviewer slot 2

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent implementation reviewer, slot 2
- Author and implementer principal: `agent:/root`
- Reviewed commit: `4a34c83eee36e6d5cfa1bd61d85f26da669ee065`
- UTC date: 2026-08-01
- Verdict: **CHANGES REQUIRED**
- Scope: Phase 0 implementation and the fixes made after the review of `745b098`, including the exact crate DAG, four-target CI/MSRV policy, buffer semantics and fuzzing, NRPC transports/lifecycle, UI/core supervision, P0.2 diagnostics, reproducible schema generation, and evidence-ledger accuracy.

This review inspected the tree at the exact commit above. The reviewer principal differs from the author/implementer principal and fills only implementation-reviewer slot 2. It does not fill an exit-binding, phase-lead, performance-reviewer, or plan-review slot.

## Verification

| Command or inspection | Result |
| --- | --- |
| Exact commit/tree and clean-worktree inspection before adding this artifact | PASS — `HEAD` was `4a34c83eee36e6d5cfa1bd61d85f26da669ee065` and inspected source matched that tree |
| `cargo +1.82.0 fmt --all -- --check` | PASS |
| `cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo +1.82.0 test --workspace --all-targets --all-features --locked` | PASS — 30 tests passed; the ignored subprocess fixture was exercised by the supervisor test |
| `cargo +stable clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo +stable test --workspace --all-targets --all-features --locked` | PASS |
| `cargo +1.82.0 check --workspace --all-targets --all-features --locked --target <target>` for Linux x64, Windows x64, macOS x64, and macOS arm64 | PASS for all four targets |
| Exact eight-package and allowed-DAG check | PASS |
| `cargo xtask schema check` | PASS — pinned Cap'n Proto 1.5.0 and byte-for-byte generated output are clean |
| `cargo xtask evidence check --phase 0` | PASS |
| `cargo +1.82.0 build --locked --release -p nvide -p nvide-core` | PASS |
| `cargo +nightly-2026-07-01 fuzz run buffer -- -runs=1000` | PASS |
| `cargo +nightly-2026-07-01 fuzz run nrpc -- -runs=1000` | PASS |
| Short release `clear` and `edit` diagnostic smoke runs | PASS as `UNBOUND_DIAGNOSTIC`; the edit row contained all runtime timestamp fields, glyph/frame fields, and a 1920×64 RGBA readback |
| `git diff --check` before adding this artifact | PASS |

## Closed findings from the prior reviews

- The `nvide` executable is now thin and the exact eight-crate dependency graph has no forbidden `nvide -> nvide-buffer` edge.
- CI installs both stable and Rust 1.82 and checks MSRV on each of the four native targets.
- `Buffer::slice`, generated sequential-batch coverage, UTF-8/line-ending/version/atomicity cases, and multi-operation fuzzing are present.
- Duplicate terminal NRPC responses are rejected and tested.
- Windows named pipes use nonblocking polling, explicit local-client rejection, and five-second read/write deadlines; Windows-only idle-read and stalled-write tests are present.
- Normal idle UI operation uses `WaitUntil`; the real supervisor test covers child failure, restart/rebind, post-restart edit traffic, budget exhaustion, and child exit before connect.
- P0.2 runtime fields, frame sequence/readback, five-second partial-failure handling, and typed renderer device-loss handling are implemented as diagnostic hooks.
- Schema generation checks the exact compiler version and remains reproducible.

## Remaining findings

### F1 — HIGH — incomplete-frame timeout is not one five-second frame deadline

ADR-0020 requires the incomplete-frame read timeout to be five seconds (`docs/adr/ADR-0020-phase-0-nrpc-wire-profile.md:47`). `read_frame` reads one byte, then calls `read_exact` for the rest of the header and payload (`crates/nvide-ipc/src/lib.rs:148-176`). The Unix transport applies a socket timeout to each individual read (`crates/nvide-platform/src/lib.rs:70-80`), and the Windows `Read` implementation creates a new deadline on every invocation (`crates/nvide-platform/src/lib.rs:272-293`). A peer can therefore send another byte before each per-read timeout and keep one incomplete frame open for substantially longer than five seconds. The timeout must cover the whole in-progress frame, with a slow-drip test proving the connection closes within the aggregate deadline.

This blocks `P0-R5`, `P0-A4`, and `P0-E4` even though the earlier Windows infinite-block and remote-client findings are fixed.

### F2 — HIGH — the edit driver advances on present-call/readback, not on compositor display

P0.2 requires the next measured edit to be dispatched only after the preceding trace reaches an actually displayed compositor event; it explicitly says a readback is not display evidence (`docs/phase-0/P0.2-benchmark-profile.md:61`). The renderer records a timestamp immediately after `SurfaceTexture::present` and completes GPU readback (`crates/nvide-render/src/lib.rs:434-447`). `Benchmark::presented` then immediately returns `DispatchEdit` (`crates/nvide-ui/src/lib.rs:558-607`), and `App::after_present` sends the next edit (`crates/nvide-ui/src/lib.rs:246-260`). There is no compositor-displayed acknowledgement in that state transition. Post-processing compositor events cannot repair the workload ordering after the edits have already been dispatched.

The diagnostic bundle also marks `sentinel_shaped` from nonzero total glyph count plus the source text containing the sentinel (`crates/nvide-ui/src/lib.rs:683-688`) and writes the raw readback without verifying sentinel pixels (`crates/nvide-ui/src/lib.rs:761-797`). These are useful diagnostics, but not yet the profile's per-sentinel shaped/readback proof. This blocks implementation agreement for `P0-R7`, `P0-A5`, and `P0-E6`, independently of the still-pending native binding.

### F3 — MEDIUM — P0-E5 still lacks executable degraded-state evidence

The runtime path does call `show_degraded` for `Unhealthy` and `RestartRequired` (`crates/nvide-ui/src/lib.rs:205-244`), and the policy unit test proves the three-miss transition. However, the real subprocess test jumps directly to `RestartRequired` by aging `last_healthy` five seconds and invokes `CoreSupervisor::restart` directly (`crates/nvide-ui/src/lib.rs:1155-1192`). It never drives the application event path through `Unhealthy` or verifies the user-visible degraded render required by `P0-E5` (`docs/plan/README.md:100`). Add the smallest executable test or immutable artifact that proves forced failure produces the degraded UI state before restart.

## Evidence verdicts

| Evidence | Verdict | Reason |
| --- | --- | --- |
| `P0-E1` | **AGREE — LOCAL IMPLEMENTATION** | Thin executable, exact crate/DAG policy, stable/MSRV CI matrix, and all four local 1.82 target checks pass. Hosted native CI evidence remains an exit-ledger concern. |
| `P0-E2` | **AGREE — LOCAL IMPLEMENTATION** | The approved generation surface, pinned compiler/source/hash, committed output, exact version check, and byte comparison pass. |
| `P0-E3` | **AGREE — LOCAL IMPLEMENTATION** | Slice semantics, atomic sequential batches, versions, undo/redo, generated cases, and the expanded buffer fuzz target pass. |
| `P0-E4` | **CHANGES REQUIRED** | Codec, lifecycle, malformed/version cases, fuzzing, and platform fixes pass, but the five-second incomplete-frame deadline resets across reads and permits slow-drip frames. |
| `P0-E5` | **CHANGES REQUIRED** | Idle scheduling, supervisor failure/restart/rebind, pre-connect failure, and real budget exhaustion pass; executable user-visible degraded-state evidence is still absent. |
| `P0-E6` | **BLOCKED / CHANGES REQUIRED** | Runtime diagnostics are substantially improved, but measured edits advance before compositor display and sentinel proof is incomplete. The eligible native 120 Hz host, tool/version/command, calibration, immutable artifacts, and two exit-binding approvals are also still pending. |

Overall verdict is **CHANGES REQUIRED**. `P0-E6` and Phase 0 exit remain **BLOCKED**. This review must not be represented as approval of the P0.2 exit binding or as a Phase 0 exit approval.
