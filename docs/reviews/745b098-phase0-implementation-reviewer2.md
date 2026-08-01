# Phase 0 implementation review — reviewer slot 2

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent implementation reviewer, slot 2
- Author and implementer principal: `agent:/root`
- Reviewed commit: `745b09861a9809575fa481ec1028e3c5d909a39d`
- UTC date: 2026-08-01
- Verdict: **CHANGES REQUIRED**
- Scope: Phase 0 implementation, with emphasis on cross-platform CI/transports, NRPC trust boundaries and failure behavior, renderer lifecycle, supervisor restart evidence, reproducible schema generation, fuzz/tests, and evidence-ledger accuracy.

This review inspected the tree at the exact commit above. The reviewer principal differs from the author/implementer principal and fills only reviewer slot 2.

## Verification

| Command or inspection | Result |
| --- | --- |
| `git rev-parse 745b098` and exact-tree hash comparison | PASS — resolved to the reviewed commit; inspected source matched that tree before this artifact was added |
| `cargo +1.82.0 fmt --all -- --check` | PASS |
| `cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo +1.82.0 test --workspace --all-targets --all-features --locked` | PASS — 26 tests |
| `cargo +1.82.0 check --workspace --all-targets --all-features --locked --target <target>` for Linux x64, Windows x64, macOS x64, and macOS arm64 | PASS for all four cross-check targets |
| Exact eight-package/DAG check from `.github/workflows/ci.yml` | PASS |
| `cargo xtask schema check` | PASS — cached Cap'n Proto 1.5.0; byte comparison clean |
| `cargo xtask evidence check --phase 0` | PASS — mapping complete |
| `cargo +1.82.0 build --locked --release -p nvide` | PASS |
| `cargo +nightly-2026-07-01 fuzz run buffer -- -runs=1000` | PASS |
| `cargo +nightly-2026-07-01 fuzz run nrpc -- -runs=1000` | PASS |
| `git diff --exit-code` and `git diff --check` before adding this artifact | PASS |
| Current GitHub-hosted runner-label check | PASS — `macos-15-intel`, `macos-15`, `windows-2022`, and `ubuntu-24.04` are valid labels in the official runner-image catalog |

## Findings

### F1 — BLOCKER — idle UI never schedules the ADR-0017 heartbeat

`nvide-ui` selects `ControlFlow::Wait` for the normal application at `crates/nvide-ui/src/lib.rs:40`, while heartbeat work runs only from `about_to_wait` at line 171. Winit 0.30.13 defines `Wait` as suspending until another event arrives; the implementation never switches to `WaitUntil` for the next one-second deadline. An idle editor can therefore stop emitting heartbeats indefinitely, so three-miss degradation and five-second restart are not enforced. This blocks P0-R6/P0-A4/P0-E5.

### F2 — BLOCKER — Windows NRPC I/O can block indefinitely

The Unix wrapper applies five-second read/write timeouts at `crates/nvide-platform/src/lib.rs:54`, but the Windows wrapper at lines 114–179 uses synchronous `PIPE_WAIT` handles backed by `File` without overlapped cancellation or another I/O deadline. The `5_000` argument to `CreateNamedPipeW` is the default used by `WaitNamedPipe`; it is not a `ReadFile`/`WriteFile` timeout. Microsoft documents that `PIPE_WAIT` operations may wait indefinitely. Consequently handshake, incomplete-frame, stalled-write, and request deadlines are not enforced on Windows, contrary to ADR-0020. This blocks P0-R5/P0-A4/P0-E4.

The same Windows pipe is created without `PIPE_REJECT_REMOTE_CLIENTS`; the local-only transport should explicitly reject remote clients rather than depend on a machine's default pipe ACL.

### F3 — HIGH — the lifecycle evidence does not exercise the supervisor

`crates/nvide-core/tests/lifecycle.rs:5` kills one child and then the test harness manually calls `spawn` for another. It never constructs or drives `CoreSupervisor`, its backoff, restart budget, degraded UI state, or `restart` method. In addition, `spawn_core` blocks in `listener.accept()` at `crates/nvide-ui/src/lib.rs:742` if a spawned child exits before connecting. The ledger's P0-E5 statement that supervised forced-failure/restart/rebind checks pass is therefore stronger than its artifact. Add one real supervisor subprocess test covering idle heartbeat loss, visible degradation, successful restart/rebind, budget exhaustion, and child-fails-before-connect.

### F4 — HIGH — the approved P0.2 edit trace is not implemented

The approved profile requires UI dispatch, core receipt, version increment, viewport emit/receive, shaped-glyph inclusion, frame sequence, present call, a five-second partial-bundle timeout, and a compositor join. The runtime CSV at `crates/nvide-ui/src/lib.rs:562` records only dispatch, a combined viewport timestamp, and present-call time. There is no core-receipt/viewport-emit event, glyph-inclusion proof, frame sequence, five-second watchdog, partial failure bundle, or frame readback. P0-E6 is correctly blocked on the native 120 Hz binding, but `IMPLEMENTED LOCALLY` in `docs/evidence/phase-0.md` still overstates the current P0-R7/P0-A5 hook coverage.

ADR-0021 also requires unrecoverable device loss to become a typed fatal/degraded result. `RenderError::DeviceLost` is declared at `crates/nvide-render/src/lib.rs:47` but is never constructed or tested, so that lifecycle path remains unproved.

### F5 — HIGH — buffer generation/fuzz does not cover the accepted sequential-edit matrix

ADR-0022 requires generated sequential batches covering atomic rejection, line terminators, strict UTF-8 boundaries, monotonic versions, and undo/redo branches. `generated_edit_roundtrips` at `crates/nvide-buffer/src/lib.rs:543` generates one insert followed by undo/redo, and `fuzz/fuzz_targets/buffer.rs` fuzzes one insert batch only. Handwritten tests cover individual cases, but the required generated edit-sequence/property coverage is absent. This blocks P0-R4/P0-A3/P0-E3.

### F6 — MEDIUM — CI does not enforce MSRV on all four targets

The four-target native matrix uses the moving `stable` toolchain at `.github/workflows/ci.yml:28`; Rust 1.82 runs only in the Linux policy job at line 40. P0-R2 requires stable/MSRV checks for Windows x64, macOS x64, macOS arm64, and Linux x64. The local four-target 1.82 cross-check passes, but CI does not keep that guarantee from regressing on the other three platforms. Add the pinned MSRV check to each target or an equivalent four-target MSRV matrix.

## Evidence verdicts

| Evidence | Verdict | Reason |
| --- | --- | --- |
| P0-E1 | **CHANGES REQUIRED** | Exact crate/DAG and local four-target checks pass, but CI does not enforce MSRV on all four targets and hosted results remain pending. |
| P0-E2 | **AGREE** | Approved command surface, pinned source/hash/compiler, committed generated output, byte-for-byte check, and evidence mapping are implemented without a blocker found. |
| P0-E3 | **CHANGES REQUIRED** | Core buffer behavior passes current tests, but accepted sequential generated/property/fuzz coverage is missing. |
| P0-E4 | **CHANGES REQUIRED** | Unix tests and codec/fuzz coverage pass; Windows cannot enforce ADR-0020 I/O deadlines and the local pipe does not explicitly reject remote clients. |
| P0-E5 | **CHANGES REQUIRED** | The idle heartbeat is not scheduled, and the subprocess artifact manually restarts rather than testing `CoreSupervisor`. |
| P0-E6 | **BLOCKED / CHANGES REQUIRED** | Native 120 Hz host/tool/calibration and approvals are absent as correctly recorded; the approved trace/device-loss hooks are also incomplete. |

Overall verdict remains **CHANGES REQUIRED**. P0-E6 must remain blocked until the approved native reference binding and immutable display artifacts exist, even after implementation findings are fixed.
