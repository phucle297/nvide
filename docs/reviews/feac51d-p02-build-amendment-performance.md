# P0.2 build-command amendment independent performance review

- Reviewer principal: `agent:/root/p02_build_amendment_perf`
- Role: Independent performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `feac51dc597b54c1289c68fb2f17dcf991622dd8`
- Review scope: exactly the independent performance-reviewer slot for the P0.2 build-command amendment; no phase-lead, implementation-reviewer, exit-binding, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Change the release build command from `cargo build --locked --release -p nvide` to `cargo build --locked --release -p nvide -p nvide-core` | `AGREE` |

## Verification

| Check | Result |
| --- | --- |
| Exact commit identity | PASS — `HEAD` and the inspected tree resolved to `feac51dc597b54c1289c68fb2f17dcf991622dd8` before this artifact was added. |
| P0.2 amendment diff | PASS — only the status, release build command, and pending amendment-review record changed in the profile; the workload, timing semantics, formulas, pass rules, artifact requirements, and exit-binding record remain unchanged. |
| `cargo build --locked --release -p nvide -p nvide-core` | PASS — produced executable sibling binaries `target/release/nvide` and `target/release/nvide-core`. |
| One-edit diagnostic smoke with zero warmup and one measured edit | PASS — `nvide` found the sibling core, completed the NRPC edit path, and wrote a versioned `UNBOUND_DIAGNOSTIC` manifest, complete runtime trace, and frame readback. |
| Exit-gate boundary | PASS — the reference host, presentation tool/version/command, calibration, P0-E6 claim, and exit-binding approvals remain explicitly pending. |

## Findings

- Building both existing Phase 0 process packages is required by the accepted thin-UI/multi-process topology and removes no measured work from either benchmark launch.
- The clear launch commands, 1920×1080 FIFO workload, five-run sampling, warmup/measurement windows, compositor authority, formulas, per-run thresholds, and artifact semantics are byte-for-byte unchanged.
- The edit workload retains the same dispatch-to-displayed-event stages, timeout, ordering/version requirements, diagnostic timing definitions, and immutable evidence requirements.
- The additional package selector changes build coverage only. It does not relax measurement eligibility or turn the local smoke result into display evidence.

No actionable finding remains in this review scope.

## Explicit exclusion

This review does **not** approve the P0.2 exit binding, a native reference host, presentation tool/version/command, calibration artifact, P0-E6 evidence, Phase 0 exit, the phase-lead slot, or any Phase 0 implementation-reviewer slot.
