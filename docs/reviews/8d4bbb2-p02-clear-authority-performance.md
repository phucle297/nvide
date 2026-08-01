# P0.2 clear authority-completion performance review

- Reviewer principal: `agent:/root/p02_drain_amendment_performance`
- Role: Independent P0.2 performance reviewer, revalidating the same harness-amendment slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `8d4bbb27306adbbc411759de296b0f635105379b`
- Review scope: only natural authority completion for a successful clear workload plus regression of successful-edit and nonzero-exit behavior. This review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Clear authority lifetime, unchanged measurement window/formulas/thresholds, failure behavior, rejected tail evidence, and native clear proof | `AGREE` |

## Independent verification

| Check | Result |
| --- | --- |
| Completion policy | PASS — a successful clear workload no longer triggers the one-second quiet or five-second stop heuristic; it waits for PresentMon's configured timed completion. A nonzero clear/edit exit still requests immediate authority stop, and a successful edit retains the prior quiet-drain/cap after its required acknowledgements. Boundary self-tests cover all three branches. |
| Exact candidate and proof | PASS — `capture-manifest.txt` binds commit `8d4bbb27306adbbc411759de296b0f635105379b`, harness SHA-256 `fe44e3728e65c2ee76e6d62df4a54203f8925845d0d54c870920a0c423bda490`, NVide SHA-256 `d549e8a3b63e179f4282c208e3de44e413fbf34da12a75e73d70d87f5ba257cc`, PresentMon 2.5.1, PID `3228`, one swapchain, Vulkan/AMD Radeon 860M, and 120 Hz. The harness hash equals the exact committed Windows-CRLF file. |
| Natural authority exit | PASS — PresentMon uses the retained `--timed 50 --terminate_after_timed` command, exits naturally with code `0`, records `presentmon_stopped_after_application_exit=False`, and retains `6,727 ms` after observing NVide's successful exit. All 4,791 authority rows are preserved; the last displayed timestamp is `26.9084 ms` after the runtime measurement end. |
| Bound authority stream | PASS — every row uses the exact PID/swapchain, `Runtime=DXGI`, `SyncInterval=1`, and `AllowsTearing=0`; the complete case-sensitive v1 schema is retained. There are 4,784 displayed and 7 dropped events overall, with no dropped event inside the measurement window. |
| Independent clear calculation | PASS — with `s=68,829,499,077,200 ns`, `e=68,859,499,077,200 ns`, and approved `R=8,332,000 ns`, the half-open window contains `N=3,600` displayed events and `expected_slots=3,600`. Displayed FPS is `119.996726647`, missed rate is `0`, and nearest-rank p99 is `9.2157 ms`. |
| Measurement-window edges | PASS — the first displayed event is `1.1001 ms` after `s` and the last is `6.4151 ms` before `e`; both are below `R=8.332 ms`. The runtime binary hash is unchanged, the runtime manifest still records exactly a 30-second measurement interval, and the new authority wait occurs only after that interval. |
| Rejected `568ccfe` run | PASS — independent recomputation gives `N=3,542/expected=3,600`, FPS `119.998593905`, missed rate `0.016111111`, p99 `9.2383 ms`, and an end gap of `490.6183 ms`. It fails both end-edge and missed-rate rules despite its capture manifest saying `PASS`; it remains explicitly rejected and contributes no accepted sample. |
| Regression checks | PASS — Windows PowerShell `-SelfTest`, `cargo test --locked -p nvide-ui --all-targets` (8 passed, 2 intentional subprocess fixtures ignored), targeted Clippy with `-D warnings`, and `git diff --check` pass. Exact GitHub Actions run `30696809503` completed successfully with all six jobs green: four target jobs, policy, and fuzz. |

No actionable finding remains in this amendment-review scope.

## Explicit exclusion

This `AGREE` verdict fills only the independent performance-reviewer slot for the P0.2 clear authority-completion amendment at the exact reviewed commit. The native run is amendment proof, not replacement calibration or formal P0-E6 evidence. Exit-binding revalidation, the five formal clear runs, edit evidence, P0-E6, Phase 0 exit, and the phase-lead slot remain separate and unapproved by this artifact.
