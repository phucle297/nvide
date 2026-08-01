# Phase 0 final re-review — reviewer slot 2

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent Phase 0 implementation and evidence reviewer, slot 2 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `a3bde4d27479b6b9f94b22cd7810213f5a880225`
- Parent commit: `f6dd768e2c12f5c64002bb19feea2b78031c46fb`
- UTC date: 2026-08-01
- Overall verdict: **AGREE — P0-E1…P0-E5 FORMAL EVIDENCE; P0-E6 IMPLEMENTATION ONLY**
- Scope: Closure of the macOS portability blocker recorded at `9e31f0017665e63d0502c647047688255a4ab5fe`, regression review of the Phase 0 implementation, and independent evidence verdicts for `P0-E1`…`P0-E6`.

This review used a clean detached worktree at the exact reviewed commit. The reviewer did not author or implement the change, fills no other reviewer slot, and does **not** approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Portability finding closure

The blocker in `docs/reviews/9e31f00-phase0-portability-reviewer2.md` is resolved through an inspectable progression:

1. [Diagnostic run 30685476814](https://github.com/phucle297/nvide/actions/runs/30685476814) at `d97986244dabcbe78e174631b03dd69bcce0deff` exposed the complete native output. Both macOS jobs identified only `nvide_ipc::tests::edit_deadline_is_shared_across_write_and_read` as failing; the lifecycle socket tests passed on both architectures.
2. [Intermediate run 30685608117](https://github.com/phucle297/nvide/actions/runs/30685608117) at `80a56011727721aac418ee68f6379ddfea57e72a` remained red. Merely widening the test while spreading sleep across four writes did not deterministically separate write-deadline expiry from the response-read phase.
3. [Diagnostic run 30685712163](https://github.com/phucle297/nvide/actions/runs/30685712163) at `f6dd768e2c12f5c64002bb19feea2b78031c46fb` completed successfully after the test isolated one 400 ms write under one 800 ms aggregate deadline, leaving a distinct read interval that must end in the typed request timeout. The same absolute deadline still covers both phases.
4. [Final direct run 30685874494](https://github.com/phucle297/nvide/actions/runs/30685874494) at the exact reviewed commit completed successfully. The temporary Python diagnostic wrapper was removed; the workflow again runs the canonical direct Cargo test command.

The final source delta from `9e31f00` is four changed test lines in `crates/nvide-ipc/src/lib.rs`. No product timeout, public/Friend API, dependency, schema, evidence hook, or production transport behavior changed. The test retains the ADR-0020 contract: one request deadline covers write plus read, expiry sends `CANCEL`, and only that request returns `RequestTimeout`. Ten consecutive local repetitions completed in 0.80 seconds each. This is a root-cause test correction rather than scheduler-specific retry logic.

## Architecture and scope

- The exact eight Phase 0 crates and CI dependency allowlist remain unchanged and satisfy ADR-0011/P0-R1/P0-R2.
- Stable plus Rust 1.82 checks remain explicit for Windows x64, macOS x64, macOS arm64, and Linux x64. Every native job also retains direct stable build, test, and Clippy commands.
- The Windows named-pipe `ReadFile` correction preserves `WouldBlock` versus EOF and is now exercised by a successful Windows native workspace test. Short, unique Unix lifecycle endpoints are exercised successfully on both hosted macOS architectures.
- ADR-0002/ADR-0020 framing, typed failure, cancellation, aggregate-deadline, and real process-boundary behavior remain intact. ADR-0017 supervisor tests remain intact.
- The intermediate diagnostic workflow is absent from the reviewed tree. No crate, dependency, abstraction, Phase 1 feature, or later-phase scaffold was added.
- The previously accepted shaped-text/readback/display-ack implementation is unchanged. Hosted unit/integration success is not a substitute for the exit-bound P0.2 native presentation evidence.

## Verification

| Check | Result |
| --- | --- |
| Exact detached commit and clean review tree | PASS — `HEAD` is `a3bde4d27479b6b9f94b22cd7810213f5a880225` |
| Scope from `9e31f00` | PASS — net delta is only four insertions/four deletions in the IPC test; diagnostic workflow additions were fully reverted |
| Whitespace and formatting | PASS — `git diff --check 9e31f00..a3bde4d`; `cargo +1.82.0 fmt --all -- --check` |
| Four-target MSRV checks | PASS — full workspace/all-target/all-feature locked checks for Linux x64, Windows x64, macOS x64, and macOS arm64 |
| Local native regression | PASS — full stable Linux workspace/all-target/all-feature locked suite |
| Local Clippy | PASS — full stable Linux workspace/all-target/all-feature locked check with `-D warnings` |
| Corrected deadline regression | PASS — ten consecutive targeted stable runs |
| Schema and evidence mapping | PASS — clean `cargo +1.82.0 xtask schema check`, clean generated diff, and `cargo +1.82.0 xtask evidence check --phase 0` |
| Hosted four-target native matrix | PASS — exact reviewed SHA; MSRV check, stable build, stable test, and stable Clippy all succeeded on every target |
| Hosted policy | PASS — format, locked check, exact crate/DAG enforcement, schema check, clean generated diff, and evidence mapping all succeeded |
| Hosted fuzz | PASS — buffer and NRPC targets each completed 1,000 runs on the pinned nightly/cargo-fuzz job |

## Evidence verdicts

| Evidence ID | Independent reviewer verdict | Basis and boundary |
| --- | --- | --- |
| `P0-E1` | **AGREE — FORMAL EVIDENCE** | Exact commit has green MSRV/build/test/Clippy jobs on all four required native targets; policy enforces the exact eight-crate DAG. |
| `P0-E2` | **AGREE — FORMAL EVIDENCE** | The exact hosted policy run verifies pinned schema generation, byte-for-byte cleanliness, and `git diff --exit-code`; the clean-checkout command also passed locally. |
| `P0-E3` | **AGREE — FORMAL EVIDENCE** | Buffer unit/generated tests pass in the four native workspace jobs and the pinned buffer fuzz target completes 1,000 runs. Prior accepted implementation coverage remains unchanged. |
| `P0-E4` | **AGREE — FORMAL EVIDENCE** | IPC/core codec, handshake, cancellation, malformed-frame, real subprocess, local transport, aggregate-deadline, and Windows named-pipe regressions pass natively; NRPC fuzz completes 1,000 runs. |
| `P0-E5` | **AGREE — FORMAL EVIDENCE** | The exact native workspace tests cover the accepted supervisor/degradation/restart/rebind/budget behavior on all four targets; prior implementation evidence is unchanged. |
| `P0-E6` | **AGREE — IMPLEMENTATION ONLY; FORMAL EVIDENCE BLOCKED** | Shaping, readback, display-ACK ordering, and shared-deadline implementation remain accepted and unchanged. No eligible native 120 Hz reference binding, approved compositor tool/version/command, calibration, immutable bound run bundle, or two exit-binding approvals were reviewed. |

The canonical ledger still needs a separate authorized update to record the exact final run and review artifacts; this reviewer was explicitly instructed not to edit it. That recording step must not broaden the verdict above.

No finding remains in this reviewer slot. Overall verdict: **AGREE** for the Phase 0 implementation and formal evidence `P0-E1`…`P0-E5`. Formal `P0-E6`, the P0.2 exit binding, and Phase 0 exit remain **BLOCKED** independently; this artifact grants none of those approvals.
