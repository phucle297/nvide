# Phase 0 implementation final re-review — reviewer slot 2

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent Phase 0 implementation reviewer, slot 2 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `10f98da6040da2ad789d1ae2667b56f3493f76b2`
- UTC date: 2026-08-01
- Overall verdict: **CHANGES REQUIRED**
- Scope: `P0-R1`…`P0-R7`, `P0-A1`…`P0-A5`, and implementation support for `P0-E1`…`P0-E6`, including the exact crate/DAG/CI policy, schema pipeline, rope/undo/fuzz behavior, local NRPC transport and lifecycle deadlines, supervisor degradation/restart evidence, renderer/readback/display-ack trace, and blocked-exit ledger accuracy.

The review used a detached worktree at the exact commit above. This reviewer did not author or implement the change, fills no other reviewer role, and does not approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Authority and verification

The review re-read `AGENTS.md`, Architecture v0.2.1 sections governing Phase 0 topology, ownership, NRPC, buffers, rendering, supervision, performance, CI, and testing; every accepted ADR present in `docs/adr`; the canonical Phase 0 roadmap, P0.2/P0.3 artifacts, evidence ledger, both `745b098` and `4a34c83` implementation reviews, and all approved P0.2 build-command/display-ack amendment artifacts.

| Check | Result |
| --- | --- |
| Exact detached commit and pre-review tree | PASS — `HEAD` resolved to `10f98da6040da2ad789d1ae2667b56f3493f76b2`; the detached tree was clean |
| Diff from approved display-ack candidate `5b2e972` | PASS — `10f98da` changes only the P0.2 status/review record and adds the two independent `5b2e972` approval artifacts |
| `cargo +1.82.0 fmt --all -- --check` | PASS |
| `cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| Exact crate list and allowed local dependency edges in the manifest/CI policy | PASS — exactly the eight Phase 0 crates and no forbidden edge |
| Static trace of aggregate NRPC frame/edit deadlines | PASS — one absolute deadline covers frame read/write and the benchmark core transaction; cancellation does not start another polling window |
| Static trace of supervisor failure → degraded state → restart/rebind → post-restart edit → budget exhaustion | PASS; focused executable coverage is present |
| Static trace of shaping/readback/present → matching compositor ACK → next-edit dispatch | PASS — the bound path cannot advance on present/readback alone, a late ACK loses to the shared deadline, and `--unbound-diagnostic` remains excluded from P0-E6 |
| Windows local transport inspection | PASS — pipe handles are nonblocking and remote clients are rejected; Rust 1.82's `std::fs::File::flush` is a no-op, so the wrapper does not introduce an unbounded `FlushFileBuffers` call |
| Full workspace test command | NOT CLAIMED — compilation was stopped at the parent request after the checks above; the exact implementation is the independently approved `5b2e972` candidate, whose focused IPC/render/UI checks are recorded in both amendment artifacts |

## Prior findings

All implementation findings from the `745b098` and `4a34c83` reviews are resolved at the reviewed commit:

- `nvide` is a thin UI binary; the exact crate DAG and four-target stable/MSRV CI policy are enforced.
- Schema generation is pinned, byte-reproducible, and checked through the approved command surface.
- Buffer slice/conversion, atomic sequential batches, versioning, branching undo/redo, generated cases, and edit fuzzing are present.
- NRPC validates framing, flags, handshake versions/roles, stream parity/lifecycle, cancellation, terminal responses, limits, and malformed input. Local reads and writes are nonblocking, and the aggregate frame deadline is shared across every partial operation.
- The application schedules idle heartbeats; a real subprocess test reaches the degraded state, restart/rebind, successful post-restart traffic, pre-connect failure, and restart-budget exhaustion.
- The renderer returns typed surface/device failures, uses bounded polling for readback, and correlates shaped sentinel pixels plus the frame marker. The approved bound edit path waits for a strictly parsed matching compositor acknowledgement before advancing and shares one absolute deadline through core, render/readback, and ACK receipt.

No remaining implementation defect was found in `P0-R1`…`P0-R7` or `P0-A1`…`P0-A5`.

## Finding

### F1 — MEDIUM — the P0-E6 ledger contradicts the approval record in the same commit

`docs/phase-0/P0.2-benchmark-profile.md:91-98` records distinct phase-lead and performance-reviewer `AGREE` verdicts for the display-acknowledgement amendment at `5b2e972`. However, `docs/evidence/phase-0.md:12` still says the display-ack amendment is pending and that no P0-E6 claim may occur until amendment review completes. That condition is already complete at the exact reviewed commit.

The row must remove only those stale amendment-pending statements and accurately distinguish:

- display-ack amendment: **APPROVED**;
- Phase 0 implementation review: pending until the required implementation review records are incorporated;
- native reference host, compositor tool/version/command, calibration, immutable formal P0-E6 evidence, and two exit-binding approvals: **PENDING / EXIT BLOCKED**.

The current wording is conservative and does not fabricate an exit, but evidence-ledger accuracy is explicitly required. This documentation inconsistency is the sole reason the overall verdict is `CHANGES REQUIRED`.

## Evidence verdicts

| Evidence | Implementation verdict | Formal exit status | Reason |
| --- | --- | --- | --- |
| `P0-E1` | **AGREE** | PENDING | Exact crates/DAG and four-target stable/MSRV CI implementation are present; hosted immutable results remain ledger work. |
| `P0-E2` | **AGREE** | PENDING | Approved deterministic schema ownership, pinning, generation, and byte-check path are present. |
| `P0-E3` | **AGREE** | PENDING | Required buffer semantics and unit/generated/fuzz coverage are present. |
| `P0-E4` | **AGREE** | PENDING | NRPC framing, compatibility, lifecycle/failure behavior, local transports, and absolute deadlines are implemented and covered. |
| `P0-E5` | **AGREE** | PENDING | Idle heartbeat, real degraded-state transition, restart/rebind, post-restart edit, pre-connect failure, and budget exhaustion are implemented and covered. |
| `P0-E6` | **AGREE — IMPLEMENTATION ONLY** | **BLOCKED** | The shaped-text/readback/display-ACK path now implements the approved protocol. Formal evidence cannot agree until the eligible native 120 Hz binding, calibrated compositor authority, immutable run bundle, and two exit-binding approvals exist. The ledger wording itself still needs F1 corrected. |

Overall verdict is **CHANGES REQUIRED** solely for F1. After that wording is corrected without weakening the pending native binding, this reviewer found no implementation blocker requiring another code change. P0-E6 formal evidence and Phase 0 exit remain **BLOCKED** regardless of this implementation verdict.
