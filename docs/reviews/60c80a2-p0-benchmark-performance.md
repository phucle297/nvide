# P0.2 protocol amendment independent performance review

- Reviewer principal: `agent:/root/p0_benchmark_reviewer`
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `60c80a2c778027d504c965ab125de5937a852977`
- Review scope: exactly the P0.2 independent performance-reviewer slot for the benchmark-protocol amendment.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Protocol workload, measurement semantics, formulas, pass rules, required environment fields, and deferred exit-binding boundary | `AGREE` |

## Evidence and findings

- The protocol fixes the eligible display class, 1920×1080 FIFO workload, build command, five fresh runs, warmup and measurement windows, compositor-observed display semantics, raw-event retention, formulas, aggregation, and independent per-run pass rules.
- The clear result cannot be inferred from submitted frames: displayed events are filtered to the process and surface, dropped and superseded presents are excluded, edge coverage is required, and the selected presentation tool must be calibrated against display rather than submission.
- The edit trace carries one identity through UI dispatch, the core version increment, viewport receipt, glyph inclusion, present, and a compositor-displayed event. All 30 measured edits must be ordered, complete, version-correct, and independently visible; a timeout yields a failing partial artifact.
- Architecture's M0.1 target remains a hard clear-workload pass. The input/edit and paint-stage timings are correctly diagnostic because Architecture does not make Phase 0 latency thresholds part of M0.2 or M0.3.
- Deferring only the concrete native host, exact tool/version/command, and calibration is safe at the implementation-entry gate: their eligibility and required fields are already constrained, while P0-E6 and Phase 0 exit remain explicitly blocked until a separate binding record is approved.
- The Markdown and HTML Phase 0 sections preserve the same protocol-entry and exit-binding split.

No finding remains in this review scope.

## Explicit exclusion

The P0.2 exit binding is **NOT APPROVED**. This review does not approve a reference host, presentation tool/version/command, calibration artifact, P0-E6 evidence, Phase 0 exit, Phase-lead slot, or full-plan reviewer slot.
