# P0.2 display-acknowledgement amendment independent performance re-review

- Reviewer principal: `agent:/root/p02_display_ack_perf`
- Role: Independent performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `5b2e972f188f52a8f4f37b127737262224e7a2d9`
- Review scope: exactly the independent performance-reviewer slot for the P0.2 display-acknowledgement amendment; no phase-lead, implementation-reviewer, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Display-authority ingress, trace ordering, aggregate deadline, readback correlation, and diagnostic/exit boundary | `AGREE` |

## Verification

| Check | Result |
| --- | --- |
| Exact commit identity | PASS — the review used a detached worktree at `5b2e972f188f52a8f4f37b127737262224e7a2d9`. |
| Architecture, ADR-0003, ADR-0021, ADR-0023, Phase 0 roadmap, and P0.2 profile inspection | PASS — the amendment remains within the accepted Phase 0 render path and uses a distinct reviewer principal. |
| `cargo +1.82.0 test --locked -p nvide-ipc frame_deadlines_cover_the_aggregate_operation` | PASS. |
| `cargo +1.82.0 test --locked -p nvide-ipc request_timeout_sends_cancel` | PASS — timeout and best-effort cancellation completed in 0.02 seconds. |
| `cargo +1.82.0 test --locked -p nvide-ipc edit_deadline_is_shared_across_write_and_read` | PASS — the stalled transaction consumed one supplied deadline across request writes and response read. |
| `cargo +1.82.0 test --locked -p nvide-ui displayed_ack_gates_the_next_edit_and_readback_must_change` | PASS — a ready matching row cannot advance an expired trace, and future timestamps are rejected. |
| `cargo +1.82.0 test --locked -p nvide-render` | PASS — 3 tests passed. |
| `cargo +1.82.0 clippy --locked -p nvide-ipc -p nvide-render -p nvide-ui --all-targets --all-features -- -D warnings` | PASS. |
| `git diff --check` in the detached worktree | PASS. |

## Closed findings

1. **Aggregate deadline:** `Trace::started + 5 s` is now passed unchanged from the UI to `CoreSupervisor::edit_before`, through NRPC request write and response read, and into renderer readback. The timeout cancellation is a single best-effort write on the already nonblocking local transport, so it cannot open another polling window. Renderer map polling accepts neither a ready callback nor copied readback after the absolute deadline.
2. **Late ACK ordering:** `display_acknowledged` checks expiry before reading the atomic CSV and again after the read. The regression test supplies a matching row to an expired trace and receives an error without calling `finish_edit`, so no next edit is dispatched.
3. **Monotonic timestamp join:** the parser now requires `present_ns <= displayed_ns <= acknowledgement receipt_ns`; the focused test rejects a displayed timestamp later than receipt.

## Retained measurement boundaries

- The strict one-row CSV contract still joins PID, exact frame sequence, and OS-monotonic timestamps; stale rows are ignored while malformed, duplicate, wrong-PID, future-frame, pre-present, and future-timestamp acknowledgements fail.
- The bound path still requires the matching compositor ACK before advancement. Readback, first-line sentinel pixels, and the second-line frame marker remain supporting correlation evidence only.
- `--unbound-diagnostic` still produces `UNBOUND_DIAGNOSTIC` and remains prohibited in P0-E6.
- The native 120-class reference host, compositor tool/version/command, PID/surface calibration, immutable evidence, and two exit-binding approvals remain pending.

No actionable finding remains in this amendment-review scope.

## Explicit exclusion

This review does **not** approve the P0.2 exit binding, a native reference host, presentation tool/version/command, calibration artifact, P0-E6 evidence, Phase 0 exit, the phase-lead slot, or any Phase 0 implementation-reviewer slot.
