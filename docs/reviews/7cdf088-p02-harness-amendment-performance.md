# P0.2 harness amendment performance final re-review

- Reviewer principal: `agent:/root/p02_drain_amendment_performance`
- Role: Independent P0.2 performance reviewer, revalidating the same harness-amendment slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `7cdf0881c8c5c2d340512616a2528ce24b58fb33`
- Review scope: closure of F1 from `docs/reviews/9ed5ae9-p02-harness-amendment-performance.md` plus regression review of the cumulative atomic-ACK, delayed single-finalizer, target-preservation, and aggregate capture-cap amendment. This review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Atomic ACK retry deadline, target/finalizer timing and identity, unchanged five-second trace deadline, rejected diagnostics, and native edit proof | `AGREE` |

## Independent verification

| Check | Result |
| --- | --- |
| F1 closure | PASS — the first replacement attempt is always allowed. After any failed attempt, `Should-AttemptAtomicReplace` gates the loop before the next `MoveFileExW` call and permits it only while elapsed time is `<250 ms`; sleep is capped to the remaining interval. At elapsed `>=250 ms` the loop exits and the retained failure is thrown, so a success cannot be accepted after the cap. Boundary checks cover first attempt at 250 ms, retry at 249 ms, and rejection at 250 ms. Non-5/non-32 errors still fail immediately. |
| Exact candidate and proof | PASS — `capture-manifest.txt` binds commit `7cdf0881c8c5c2d340512616a2528ce24b58fb33`, harness SHA-256 `3f033bbb9beeb1e985fc7e32829a9603ce9855599fa01a0c61dc36dd211e6e54`, NVide SHA-256 `d549e8a3b63e179f4282c208e3de44e413fbf34da12a75e73d70d87f5ba257cc`, PresentMon 2.5.1, PID `10012`, one swapchain, Vulkan/AMD Radeon 860M, 120 Hz, and a successful bound edit run. The harness hash equals the exact committed Windows-CRLF file. |
| Authority stream | PASS — all 190 retained rows use the bound PID/swapchain, `Runtime=DXGI`, `SyncInterval=1`, and `AllowsTearing=0`; the complete v1 schema is retained. There are 183 displayed and 7 dropped rows overall, while every correlation target and finalizer is displayed. The capture reports exactly 40 acknowledgements and exits successfully after an 890 ms authority drain. |
| Request/ACK completeness | PASS — exactly 40 immutable requests have trace IDs `1…40` and target sequences `112…190` in steps of two. Every request has exactly one displayed PresentMon match within 2 ms; maximum join distance is `0.8791 ms`. The 30 measured rows reproduce the same request identity and displayed timestamp under the harness's double-to-decimal conversion. |
| Single-finalizer evidence | PASS — every target row is followed immediately by one distinct finalizer row, and consecutive target indices differ by exactly two. Raw QPC target→finalizer intervals are `53.5193–95.9618 ms`; the minimum is `6.431` times the approved `R=8.32205 ms`. The state machine preserves the target sequence, present timestamp, and readback when recording the finalizer. |
| Measured trace integrity | PASS — `runtime.csv` contains exactly 30 measured traces with IDs/versions `11…40`, unique expected sentinels `K…n`, ordered dispatch→core→version→viewport→present→display timestamps, matching expected/actual frame sequences, shaped sentinels, and positive sentinel-pixel checks. All 30 `1920×64` RGBA readbacks exist, are distinct, and have non-uniform sentinel rows. |
| Diagnostic timing | PASS — independently calculated UI-dispatch→version-increment median/p95 is `0.66425/1.4314 ms`; viewport-receipt→present median/p95 is `4.0959/5.3432 ms`. Dispatch→display median/p95/max is `18.93555/30.3821/31.6599 ms`. These remain diagnostic under P0.2. |
| Deadline and historical failures | PASS — the unchanged five-second runtime deadline is checked before and after ACK file reading, so neither the 250 ms retry allowance nor the 210-second aggregate authority cap can authorize a late trace. The failed `44b0f54`, `4ad7bcf`, and `cdeaf18` runs remain explicitly rejected and supply no accepted samples. |
| Regression checks | PASS — Windows PowerShell `-SelfTest`, `cargo test --locked -p nvide-ui --all-targets` (8 passed, 2 intentional subprocess fixtures ignored), targeted Clippy with `-D warnings`, and `git diff --check` pass. Exact GitHub Actions run `30696141131` completed successfully with all six jobs green: four target jobs, policy, and fuzz. |

No actionable finding remains in this amendment-review scope.

## Explicit exclusion

This `AGREE` verdict fills only the independent performance-reviewer slot for the P0.2 harness amendment at the exact reviewed commit. The native run is amendment proof, not replacement calibration or formal P0-E6 evidence. Exit-binding revalidation, formal workload evidence, P0-E6, Phase 0 exit, and the phase-lead slot remain separate and unapproved by this artifact.
