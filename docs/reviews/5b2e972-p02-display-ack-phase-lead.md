# P0.2 display-acknowledgement amendment — phase-lead re-review

- Reviewer principal: `agent:/root/p02_display_ack_lead`
- Role: Phase lead
- Verdict: **AGREE**
- UTC date: 2026-08-01
- Reviewed commit: `5b2e972f188f52a8f4f37b127737262224e7a2d9`
- Scope: Only the P0.2 display-acknowledgement amendment and its supporting Phase 0 UI/render/IPC implementation. This is not an exit-binding review and does not approve P0-E6 or Phase 0 exit.

## Authority checked

- Architecture v0.2.1 ADR process, rendering pipeline, performance profile, and Phase 0 milestones.
- Accepted ADR-0003, ADR-0021, and ADR-0023.
- `docs/phase-0/P0.2-benchmark-profile.md` at the reviewed commit.
- Prior phase-lead findings in `docs/reviews/8ae438a-p02-display-ack-phase-lead.md`.

## Checks

- Inspected the exact commit in a detached worktree and traced the amendment from edit dispatch through NRPC, renderer/readback, ACK receipt, and next-edit dispatch.
- Verified that the trace creates one absolute `Instant`; `CoreSupervisor::edit_before` forwards it unchanged; and `Client::edit_before` uses it for request write, flush, response read, final expiry, and a one-shot best-effort cancellation without starting another transport wait.
- Verified that the same absolute `Instant` reaches `Renderer::render` and `finish_readback`. Renderer work does not create a replacement duration, and expiry is checked before accepting a map callback and after mapped-data processing.
- Verified that ACK handling checks trace expiry before and after file receipt, so a late matching row cannot advance the workload even though `about_to_wait` invokes ACK handling first.
- Verified `present_ns <= displayed_ns <= acknowledgement receipt_ns`, stale-row ignore, future-frame/timestamp rejection, duplicate/malformed/wrong-PID rejection, sentinel/frame correlation, and the `UNBOUND_DIAGNOSTIC` exclusion from P0-E6.
- Ran `cargo test -p nvide-ipc -p nvide-render -p nvide-ui --lib --locked` (pass: IPC 11, render 3, UI 6; one UI subprocess fixture intentionally ignored).

## Findings

The three blocking findings from the `8ae438a` review are resolved at the reviewed commit. The focused tests cover a deadline shared across write/read, best-effort cancellation, an expired matching ACK, future receipt timestamp rejection, and identity of the pending/readback deadline.

No blocking finding remains in this review scope.

## Verdict

**AGREE.** Commit `5b2e972f188f52a8f4f37b127737262224e7a2d9` is acceptable for the phase-lead slot of the P0.2 display-acknowledgement amendment. This approval fills no other reviewer slot, does not approve the exit binding, and does not approve P0-E6 or Phase 0 exit.
