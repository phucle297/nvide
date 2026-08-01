# Phase 0 implementation re-review — reviewer slot 2

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent Phase 0 implementation reviewer, slot 2 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `474c458a7c4027826de3703f7a8c04819ebca6ac`
- UTC date: 2026-08-01
- Overall verdict: **CHANGES REQUIRED**
- Scope: Re-review of the slot-2 finding at `10f98da` plus the Windows non-draining flush and live-hung-core heartbeat evidence added for the slot-1 findings; evidence verdicts cover implementation support for `P0-E1`…`P0-E6`.

This review used a detached worktree at the exact commit above. The reviewer did not author or implement the change, fills no other reviewer role, and does **not** approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Verification

| Check | Result |
| --- | --- |
| Exact detached commit and clean pre-review tree | PASS — `HEAD` resolved to `474c458a7c4027826de3703f7a8c04819ebca6ac` |
| Targeted diff from `10f98da` | PASS — changes are limited to the three reviewed fixes, their ledger wording, and incorporation of the two `10f98da` implementation-review artifacts |
| `cargo +1.82.0 fmt --all -- --check` | PASS |
| `cargo +1.82.0 clippy -p nvide-platform -p nvide-ipc -p nvide-ui --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo +1.82.0 test -p nvide-ipc -p nvide-platform -p nvide-ui --all-targets --all-features --locked` | PASS — IPC 11, platform 1 on Unix, UI 7; two UI subprocess fixtures intentionally ignored as direct tests and exercised by their parent tests |
| `cargo +1.82.0 check -p nvide-platform -p nvide-ipc -p nvide-ui --all-targets --all-features --locked --target x86_64-pc-windows-msvc` | PASS |
| Static inspection of Windows server-end non-draining flush | PASS — `LocalStream::flush` is explicitly a no-op and the Windows-only test writes while the client deliberately does not read, then requires flush to return before the peer is dropped |
| Static and executable inspection of live hung-core heartbeat | PARTIAL — three real one-second timeouts reach `Unhealthy`, but the test advances the health clock manually before its restart assertion |

## Closed findings

### Slot-2 P0-E6 ledger finding — resolved

`docs/evidence/phase-0.md:12` now says the P0.2 display-ack amendment is approved while the exit binding remains pending. It removes the obsolete amendment-review condition and still states that the eligible native 120 Hz host/tool/calibration, implementation-review incorporation, formal P0-E6 evidence, and exit binding remain incomplete. The correction does not weaken the block.

### Slot-1 Windows flush finding — resolved

The Windows `LocalStream` now makes protocol `flush` an explicit no-op (`crates/nvide-platform/src/lib.rs:303-305`), so no server-side peer-drain syscall sits outside the NRPC absolute deadline. The Windows-only test at `crates/nvide-platform/src/lib.rs:377-396` holds a connected client without reading, verifies a write succeeds, and requires flush to return immediately. Windows cross-compilation passes.

### Slot-1 heartbeat implementation — code path resolved

`Client::heartbeat_before` accepts the caller's absolute deadline, and `CoreSupervisor::heartbeat` supplies `Instant::now() + HEARTBEAT_INTERVAL` (`crates/nvide-ui/src/lib.rs:1133-1155`). The live hung subprocess therefore produces one bounded miss per call instead of spending the entire five-second restart window in its first heartbeat. The focused test proves three real misses transition through `Missed`, `Missed`, then `Unhealthy`.

## Remaining finding

### F1 — MEDIUM — the live-hung-core test does not prove the claimed five-second restart

After three real one-second heartbeat timeouts, `hung_core_misses_three_heartbeats_before_restart` subtracts two seconds directly from `supervisor.last_healthy` and expects the fourth miss to return `RestartRequired` (`crates/nvide-ui/src/lib.rs:1584-1595`). This bypasses the live fourth-second state and fifth-second restart that `docs/evidence/phase-0.md:11` claims as live hung-child evidence.

The implementation appears capable of satisfying the rule without clock manipulation: leave `last_healthy` untouched, assert miss 4 remains `Unhealthy`, and assert miss 5 returns `RestartRequired`. That smallest test change would demonstrate the Architecture's real `3 missed → unhealthy; 5 s → restart` path against the hung-but-live subprocess. Until then, `P0-E5` overstates this focused evidence.

## Evidence verdicts

| Evidence | Implementation verdict | Formal exit status | Reason |
| --- | --- | --- | --- |
| `P0-E1` | **AGREE** | PENDING | Exact crate/DAG and four-target stable/MSRV implementation are unchanged from the prior accepted review scope. |
| `P0-E2` | **AGREE** | PENDING | Reproducible schema implementation is unchanged from the prior accepted review scope. |
| `P0-E3` | **AGREE** | PENDING | Buffer semantics and generated/fuzz coverage are unchanged from the prior accepted review scope. |
| `P0-E4` | **AGREE** | PENDING | Aggregate NRPC deadlines remain intact; Windows protocol flush is explicitly non-draining and has focused platform evidence. |
| `P0-E5` | **CHANGES REQUIRED** | PENDING | The one-second deadline and real three-miss degradation path pass, but the live fifth-second restart assertion is replaced by direct clock manipulation while the ledger claims the complete live policy. |
| `P0-E6` | **AGREE — IMPLEMENTATION ONLY** | **BLOCKED** | The prior display-ACK implementation agreement remains valid and the ledger wording is corrected. Formal evidence still lacks the eligible native host, compositor tool/version/command, calibration, immutable run bundle, and two exit-binding approvals. |

Overall verdict is **CHANGES REQUIRED** solely for F1. This verdict does not reopen the Windows flush or P0-E6 implementation findings. Formal P0-E6 evidence and Phase 0 exit remain **BLOCKED** independently of the focused P0-E5 test correction.
