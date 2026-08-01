# Phase 0 implementation re-review — reviewer slot 1

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Independent Phase 0 implementation reviewer, slot 1 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `474c458a7c4027826de3703f7a8c04819ebca6ac`
- UTC date: 2026-08-01
- Overall verdict: **CHANGES REQUIRED**
- Scope: re-review of findings F1/F2 from `docs/reviews/10f98da-phase0-implementation-reviewer1.md`, their tests and `P0-E4`/`P0-E5` ledger wording, plus regression status for implementation support of `P0-E1`…`P0-E6`.

This review used a clean detached worktree at the exact commit above. The reviewer did not author or implement the reviewed change, fills no other reviewer slot, and does not approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Re-review summary

The Windows flush defect is resolved. Windows `LocalStream::flush` is now explicitly non-draining and returns immediately; writes still go directly through the unbuffered pipe handle. This removes `FlushFileBuffers` and its peer-drain wait from the NRPC deadline path. The Windows stalled-peer regression test is present and compiles for `x86_64-pc-windows-msvc`; it cannot be executed on this Linux review host.

The supervisor now calls `Client::heartbeat_before` with one absolute `HEARTBEAT_INTERVAL` deadline. A live subprocess that completes the handshake and then stops replying produces real one-second timeout failures, so the implementation no longer jumps from its first miss directly to restart. The focused subprocess test observed the first two `Missed` results and third `Unhealthy` result as intended.

One acceptance-evidence defect remains.

## Finding

### F1 — MEDIUM — the hung-core test simulates, rather than observes, the five-second restart boundary

`hung_core_misses_three_heartbeats_before_restart` performs three real one-second heartbeat timeouts and reaches `Unhealthy`, but then directly subtracts two seconds from the supervisor's private `last_healthy` field before heartbeat 4 (`crates/nvide-ui/src/lib.rs:1584-1595`). The focused test completes its four real heartbeat waits in approximately 4.02 seconds. `RestartRequired` therefore occurs only because state was backdated; the live hung subprocess does not itself cross the Architecture's five-second liveness boundary.

The pure `heartbeat_policy_degrades_then_restarts` unit test correctly checks the five-second threshold, and the production implementation appears consistent with the policy. However, the Phase 0 acceptance and ledger claim executable live-child evidence for the combined one-second miss, three-miss degradation, and five-second restart path. Issue real heartbeat calls through the five-second boundary (or otherwise assert genuine elapsed time without mutating `last_healthy`) so the subprocess test demonstrates that final transition. This blocks implementation agreement for `P0-E5`; it is a test/evidence blocker, not a newly found production-code defect.

## Evidence verdicts

| Evidence | Implementation verdict | Reason |
| --- | --- | --- |
| `P0-E1` | **AGREE — LOCAL IMPLEMENTATION** | The exact workspace/DAG/CI implementation is unchanged from the prior review. Hosted immutable results remain a formal exit concern. |
| `P0-E2` | **AGREE — LOCAL IMPLEMENTATION** | The approved deterministic schema pipeline is unchanged from the prior review. |
| `P0-E3` | **AGREE — LOCAL IMPLEMENTATION** | Buffer semantics and unit/generated/fuzz implementation are unchanged from the prior review. |
| `P0-E4` | **AGREE — LOCAL IMPLEMENTATION** | Windows flush no longer waits for peer drain, the Windows regression test compiles, and focused IPC/core/platform tests pass. A native Windows execution artifact remains pending in the formal ledger. |
| `P0-E5` | **CHANGES REQUIRED** | The one-second deadline and live hung-child degradation are implemented, but the real subprocess test backdates `last_healthy` instead of observing the five-second restart boundary. |
| `P0-E6` | **AGREE — IMPLEMENTATION ONLY; FORMAL EVIDENCE BLOCKED** | The implementation is unchanged from the prior review and the ledger now correctly records the approved display-ACK amendment. No eligible native 120 Hz host/tool/calibration run or immutable exit evidence was reviewed. |

## Checks performed

All commands below targeted the detached reviewed commit.

```text
git rev-parse HEAD
# 474c458a7c4027826de3703f7a8c04819ebca6ac

cargo +1.82.0 fmt --all -- --check
# PASS

cargo +1.82.0 check -p nvide-platform --tests --locked --target x86_64-pc-windows-msvc
# PASS; compile-only, not native Windows execution

cargo +1.82.0 test -p nvide-ui --locked hung_core_misses_three_heartbeats_before_restart -- --nocapture
# PASS; test body 4.02 s, demonstrating four real one-second waits before the synthetic time adjustment

cargo +1.82.0 test -p nvide-ipc -p nvide-platform -p nvide-ui -p nvide-core --all-targets --locked
# PASS; 23 tests passed and 2 subprocess fixtures were intentionally ignored as direct tests
```

The diff from `10f98da` was inspected in full. `docs/evidence/phase-0.md` now correctly says the P0.2 display-acknowledgement amendment is approved while implementation review and exit binding remain pending.

Overall verdict remains **CHANGES REQUIRED** solely for the live five-second supervisor evidence above. Formal `P0-E6` evidence and Phase 0 exit remain **BLOCKED** independently: this review performed no native benchmark, creates no P0-E6 formal approval, and makes no Phase 0 exit claim.
