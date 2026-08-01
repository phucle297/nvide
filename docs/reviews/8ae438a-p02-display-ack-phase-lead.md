# P0.2 display-acknowledgement amendment — phase-lead review

- Reviewer principal: `agent:/root/p02_display_ack_lead`
- Role: Phase lead
- Verdict: **CHANGES REQUIRED**
- UTC date: 2026-08-01
- Reviewed commit: `8ae438acf92db0d564f3a5e71a7bb2568aaac061`
- Scope: Only the P0.2 display-acknowledgement amendment and its supporting Phase 0 UI/render implementation. This is not an exit-binding review and does not approve P0-E6 or Phase 0 exit.

## Authority checked

- Architecture v0.2.1 ADR process, rendering pipeline, performance profile, and Phase 0 milestones.
- Accepted ADR-0003, ADR-0021, and ADR-0023.
- `docs/phase-0/P0.2-benchmark-profile.md` at the reviewed commit.

## Checks

- Inspected the exact commit in a detached worktree.
- Traced edit dispatch through core response, shaping, frame capture/present, displayed-ack parsing, and next-edit dispatch.
- Checked atomic-file ingress semantics and handling for stale, future/out-of-order, duplicate, malformed, wrong-PID, and pre-present acknowledgements.
- Checked bound versus `--unbound-diagnostic` behavior, sentinel/readback correlation, artifact status, and the P0-E6 exclusion.
- Ran `cargo test -p nvide-ui --lib displayed_ack_gates_the_next_edit_and_readback_must_change --locked` (pass: 1 test).

## Findings

1. **The synchronous core edit does not share the trace deadline.** `CoreSupervisor::edit` forwards to `Client::edit` without a deadline. That call gives request-frame writing a fresh five-second window and then gives response-frame reading another fresh five-second window; its timeout cancellation may start yet another write window. Because this work blocks the UI event loop, the trace timeout cannot run while it waits. A response can therefore arrive well after the trace's five seconds and still proceed to shaping/rendering. The edit transaction must consume the trace's one absolute deadline across request write, response read, and cancellation, with a regression test that stalls the peer across operation boundaries.

2. **The readback deadline is not aggregate across renderer work.** `Benchmark::readback_timeout` computes the trace's remaining duration before `Renderer::render`, but `Renderer::render` performs surface acquisition, command encoding, queue submission, and present before passing that unchanged duration to `finish_readback`. `finish_readback` then starts a new deadline at `Instant::now() + timeout`. Time consumed before map polling is therefore added back, so core response plus render plus readback may exceed the amendment's single five-second trace deadline. The deadline must remain absolute (or the remaining duration must be recomputed after pre-map work), and expiry must win even if map completion becomes observable after the deadline.

3. **A matching ACK arriving after five seconds currently wins over timeout.** `App::about_to_wait` calls `display_acknowledged` and finishes/dispatches the next edit before it checks `pending_timed_out`. A matching row visible after the trace deadline can therefore pass. The aggregate deadline must be checked before accepting the ACK, with a regression test covering an expired pending trace plus a matching row.

The proposed CSV contract, atomic replacement responsibility, PID/frame correlation, strict row parsing, stale-row ignore behavior, future-frame rejection, sentinel/frame readback support, and `UNBOUND_DIAGNOSTIC` exclusion are otherwise suitable for this Phase 0 prerequisite.

## Verdict

**CHANGES REQUIRED.** Resolve all three aggregate-deadline defects and add focused regression coverage before requesting this same phase-lead slot to re-review a new exact commit. This verdict neither fills nor implies either exit-binding reviewer slot and does not approve P0-E6 or Phase 0 exit.
