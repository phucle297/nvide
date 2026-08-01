# P0.2 display-acknowledgement amendment independent performance review

- Reviewer principal: `agent:/root/p02_display_ack_perf`
- Role: Independent performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `8ae438acf92db0d564f3a5e71a7bb2568aaac061`
- Review scope: exactly the independent performance-reviewer slot for the P0.2 display-acknowledgement amendment; no phase-lead, implementation-reviewer, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Display-authority ingress, trace ordering, aggregate deadline, readback correlation, and diagnostic/exit boundary | `CHANGES REQUIRED` |

## Verification

| Check | Result |
| --- | --- |
| Exact commit identity | PASS — `HEAD` and the inspected tree resolved to `8ae438acf92db0d564f3a5e71a7bb2568aaac061` before this artifact was added. |
| Architecture, ADR-0003, ADR-0021, ADR-0023, Phase 0 roadmap, and P0.2 profile inspection | PASS — the amendment remains inside the accepted Phase 0 render path and uses a distinct review principal. |
| `cargo +1.82.0 test --locked -p nvide-ui displayed_ack_gates_the_next_edit_and_readback_must_change` | PASS — the current unit test proves a missing ACK gates advancement and exercises stale/future frame parsing plus synthetic readback change detection. |
| `cargo +1.82.0 test --locked -p nvide-render` | PASS — 3 tests passed. |
| `cargo +1.82.0 test --locked -p nvide-ipc request_timeout` | PASS — the request timeout/cancellation test passed in 5.00 seconds. |
| `git diff --check` | PASS. |

## Confirmed measurement properties

- The bound path obtains displayed-event authority only through the future compositor harness. Runtime readback remains supporting correlation data and is not labelled as compositor display proof.
- The CSV parser requires the exact header and one row, joins decimal PID plus exact frame sequence, ignores a preceding frame, and rejects malformed, duplicate, wrong-PID, future-frame, and pre-present acknowledgements.
- A matching ACK is required before `finish_edit` can dispatch the next bound edit. `--unbound-diagnostic` bypasses that wait only while marking the manifest `UNBOUND_DIAGNOSTIC`, and P0.2 explicitly prohibits that flag in P0-E6.
- The renderer shapes the sentinel first line and `frame:<sequence>` second line, captures a 64-row strip, correlates expected/actual sequence, and requires the first 22 rows to be non-uniform and changed from the preceding readback.
- GPU mapping uses `Maintain::Poll` and a polling loop rather than an unbounded blocking wait.
- The native 120-class host, approved compositor tool/version/command, surface filtering, calibration, immutable evidence, and exit-binding approvals remain explicitly pending.

## Required changes

### F1 — HIGH — the five-second trace deadline is restarted between core, render, and readback stages

P0.2 requires the core response, readback polling, and display-ACK wait to share one aggregate five-second trace deadline. The UI records `Trace::started` before `CoreSupervisor::edit`, but the synchronous NRPC client does not receive that deadline. `Client::edit` performs a frame write, response read, and timeout cancellation write through operations that each create their own five-second deadline. A stalled request can therefore consume the trace budget and then consume another independent deadline while cancelling.

The UI later computes a remaining `Duration` before `Renderer::render`, but `Renderer::finish_readback` creates `Instant::now() + timeout` only after surface acquisition, command encoding, queue submission, and `present`. That adds all preceding render work outside the aggregate deadline. `Maintain::Poll` makes the mapping loop nonblocking, but it bounds only this restarted local timer.

Pass one absolute trace deadline through the synchronous core request and renderer/readback path, check it before each stage, and add one runnable test that proves the entire core→render/readback→ACK sequence cannot exceed the original trace deadline.

### F2 — HIGH — an ACK arriving after the deadline can advance the workload

`App::about_to_wait` calls `Benchmark::display_acknowledged` before `Benchmark::pending_timed_out`. Consequently, a matching row observed after five seconds is accepted and can dispatch the next edit before the timeout branch runs. `display_acknowledged` itself does not reject an expired trace, and the current ACK test covers only an unexpired trace.

Evaluate the absolute deadline before accepting an ACK and add a regression test with an expired pending trace plus a matching row. A late ACK must write a failed partial bundle and must never advance the workload.

### F3 — MEDIUM — the displayed timestamp is not checked against receipt time

The parser rejects `displayed_ns < present_ns`, but accepts any larger value, including a timestamp in the future relative to NVide's same OS-monotonic clock. The amendment promises strict rejection of future acknowledgements and uses the monotonic timestamp as part of the join. Capture the runtime monotonic time at receipt, require `present_ns <= displayed_ns <= received_ns`, and cover the future-timestamp case in the parser test.

## Explicit exclusion

This review does **not** approve the P0.2 exit binding, a native reference host, presentation tool/version/command, calibration artifact, P0-E6 evidence, Phase 0 exit, the phase-lead slot, or any Phase 0 implementation-reviewer slot. Those remain pending even after the findings above are fixed.
