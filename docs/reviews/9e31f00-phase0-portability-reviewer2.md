# Phase 0 portability re-review — reviewer slot 2

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent Phase 0 implementation reviewer, slot 2 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `9e31f0017665e63d0502c647047688255a4ab5fe`
- Parent commit: `7bc5fed4d60d28e015d56bd342762026aea58521`
- UTC date: 2026-08-01
- Verdict: **CHANGES REQUIRED**
- Scope: CI policy metadata deletion, macOS-safe short unique lifecycle endpoints, hosted-scheduler deadline-test margins, and the Windows `ReadFile` transport correction.

This review used a clean detached worktree at the exact commit above. The reviewer did not author or implement the change, fills no other reviewer slot, and does **not** approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Finding

### F1 — BLOCKER — both hosted macOS native test jobs remain red

[Hosted run 30685372716](https://github.com/phucle297/nvide/actions/runs/30685372716) completed against the exact reviewed SHA with Linux x64, Windows x64, policy, and fuzz green, but both `aarch64-apple-darwin` and `x86_64-apple-darwin` failed the exact stable test step:

```text
cargo +stable test --workspace --all-targets --all-features --locked --target <macOS target>
```

The MSRV check and stable build completed successfully before each failing test step. The public job metadata exposes exit code 101 but not the underlying test output; downloading the full job log requires repository administration rights. Consequently, this reviewer cannot identify the failing executable or test from the available hosted artifact and does not speculate about its cause.

`P0-R2` and `P0-A1` require the recorded four-target build/test commands to be green. The shorter lifecycle socket endpoint is statically unique and well within the macOS Unix-domain-socket path limit, but the exact native run disproves completion of the portability gate. Capture the full failed-test output (or add narrowly scoped diagnostics that identify the failing package/test), correct the native macOS failure, and rerun the exact four-target workflow before requesting another review.

## Delta assessment

- Removing the redundant standalone `cargo metadata --locked --format-version 1` call does not weaken the policy gate. The immediately following exact crate/DAG check still consumes `cargo metadata --locked --no-deps --format-version 1`, and the hosted policy job is green.
- The lifecycle endpoint change adds a process-wide atomic suffix and shortens Unix paths to `nvc-<pid>-<id>.sock`. It preserves Windows behavior and introduces no product API or Phase 1 scope. Its intended macOS portability result is nevertheless unproven because both native macOS jobs failed.
- The deadline tests preserve the 20/30 ms semantic request/edit deadlines while widening only scheduler-facing elapsed-time assertions to 500 ms; frame and stalled-writer deadlines move from 20 ms to 100 ms. The tests still verify aggregate deadline and typed timeout behavior.
- The Windows transport now calls `ReadFile` directly, preserving `ERROR_NO_DATA` as `WouldBlock`, treating `ERROR_BROKEN_PIPE` as EOF, and returning typed OS errors otherwise. This avoids `std::fs::File::read` collapsing the named-pipe nonblocking condition into EOF, caps the requested length to `u32`, handles an empty buffer, and retains a local `SAFETY` explanation. The hosted Windows job is green.
- The direct delta changes only `.github/workflows/ci.yml`, `crates/nvide-core/tests/lifecycle.rs`, `crates/nvide-ipc/src/lib.rs`, and `crates/nvide-platform/src/lib.rs`. No dependency, schema, evidence-ledger, later-phase crate, or Phase 1 feature is added.

## Verification

| Check | Result |
| --- | --- |
| Exact detached commit and clean review tree | PASS — `HEAD` resolved to `9e31f0017665e63d0502c647047688255a4ab5fe` |
| Parent and direct scope | PASS — parent is `7bc5fed4d60d28e015d56bd342762026aea58521`; four expected files only, 40 insertions and 16 deletions |
| Whitespace validation | PASS — `git diff --check 7bc5fed4..9e31f00` |
| Formatting | PASS — `cargo +1.82.0 fmt --all -- --check` |
| MSRV cross-target checks | PASS — full workspace/all-target/all-feature locked checks for Linux x64, Windows x64, macOS x64, and macOS arm64 |
| Linux native tests | PASS — full stable workspace/all-target/all-feature locked suite |
| Linux Clippy | PASS — full workspace/all-target/all-feature locked check with `-D warnings` |
| Windows cross-target Clippy | PASS — full stable workspace/all-target/all-feature locked check with `-D warnings` |
| Schema and evidence mapping | PASS — `cargo +1.82.0 xtask schema check`, clean generated diff, and `cargo +1.82.0 xtask evidence check --phase 0` |
| Hosted Linux x64 / Windows x64 / policy / fuzz | PASS — exact reviewed SHA in run 30685372716 |
| Hosted macOS x64 / macOS arm64 | **FAIL — both exact stable test steps exited 101** |

Overall verdict: **CHANGES REQUIRED** for F1. The static portability changes are narrow and the hosted Windows/policy repairs are successful, but an implementation/CI portability approval cannot be issued while both required macOS jobs fail. This verdict neither grants nor changes the independent formal-evidence state: P0-E6 and Phase 0 exit remain **BLOCKED** pending the approved native benchmark binding, immutable exit evidence, and their required independent approvals.
