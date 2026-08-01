# P0.2 harness amendment performance revalidation

- Reviewer principal: `agent:/root/p02_drain_amendment_performance`
- Role: Independent P0.2 performance reviewer, revalidating the same harness-amendment slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `9ed5ae90a6d216a8b8532ccc5f9a0d9022a25957`
- Review scope: only the atomic acknowledgement replacement, delayed single-finalizer, target-preservation, and aggregate capture-cap amendments through the exact reviewed commit. This review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Atomic ACK retry policy, target/finalizer timing and identity, unchanged five-second trace deadline, rejected diagnostic runs, and native edit proof | `CHANGES REQUIRED` |

## Independent verification

| Check | Result |
| --- | --- |
| Exact candidate and proof | PASS — the capture manifest binds commit `9ed5ae90a6d216a8b8532ccc5f9a0d9022a25957`, harness SHA-256 `c5e0461d1dd14290cafa3c30ed154043aaee5216878a6bcb69c111dccb89facd`, NVide SHA-256 `d549e8a3b63e179f4282c208e3de44e413fbf34da12a75e73d70d87f5ba257cc`, PresentMon 2.5.1, PID `11164`, one swapchain, Vulkan/AMD Radeon 860M, 120 Hz, and a successful bound edit run. The harness hash equals the exact committed Windows-CRLF file. |
| Authority stream | PASS — all 192 rows use the bound PID/swapchain, `Runtime=DXGI`, `SyncInterval=1`, and `AllowsTearing=0`; the complete exact v1 header is retained. The capture reports 40 requests and 40 acknowledgements and exits successfully after an 890 ms authority drain. |
| Request/ACK completeness | PASS — there are exactly 40 immutable requests with trace IDs `1…40` and target frame sequences `114…192` in steps of two. Every request has exactly one PresentMon match inside 2 ms, every matched target is displayed, and the 30 retained measured rows reproduce the same request identity and displayed timestamp. |
| Single-finalizer timing | PASS — each of the 40 target rows is followed immediately by one distinct finalizer row, and consecutive target indices differ by exactly two. Raw QPC target→finalizer intervals are `52.2398–96.8736 ms`; the minimum is `6.277` times the approved `R=8.32205 ms`. All targets and all finalizers are displayed. The state machine preserves the target sequence/timestamp when recording the finalizer. |
| Measured trace integrity | PASS — `runtime.csv` contains exactly 30 measured traces with IDs/versions `11…40`, unique expected sentinels `K…n`, ordered dispatch→core→version→viewport→present→display timestamps, matching expected/actual frame sequences, shaped sentinels, and positive sentinel-pixel checks. All 30 `1920×64` RGBA readbacks exist, are distinct, and have non-uniform sentinel rows. |
| Diagnostic timing | PASS — independently calculated UI-dispatch→version-increment median/p95 is `0.9361/1.5714 ms`; viewport-receipt→present median/p95 is `4.07425/6.5445 ms`. Dispatch→display is at most `30.8007 ms`. These remain diagnostic under P0.2. |
| Per-trace deadline and aggregate cap | PASS — each trace retains one deadline at `trace.started + 5 s`; ACK polling checks it before publishing/reading and again before accepting parsed content, so a late ACK cannot advance a trace. The PresentMon cap is now `5 s × 40 + 10 s = 210 s`; it does not extend the runtime deadline. |
| Rejected attempts | PASS — both `44b0f54` runs failed closed on Windows error 5 and remain rejected. The `4ad7bcf` proof stopped after four ACKs with a dropped target only `5.9409 ms` before its finalizer; the `cdeaf18` proof stopped after eight ACKs with a dropped target `15.1754 ms` before its finalizer. Neither contributes accepted evidence. |
| Local regression checks | PASS — Windows PowerShell `-SelfTest`, `cargo test --locked -p nvide-ui --all-targets` (8 passed, 2 intentional subprocess fixtures ignored), targeted Clippy with `-D warnings`, and `git diff --check` pass. GitHub Actions run `30695811534` was still in progress when this blocking verdict was recorded. |
| Atomic retry upper bound | **FAIL** — see F1. |

## Required change

### F1 — MEDIUM — a successful replacement can be accepted after the normative 250 ms cap

`Should-RetryAtomicReplace` correctly returns false at elapsed `250 ms`, but the loop checks elapsed only **after** a failed `MoveFileExW` call. For example, a transient failure at `245–249 ms` is approved for retry, the harness sleeps 5 ms, and then calls `MoveFileExW` again at or after `250 ms` before checking the clock. If that out-of-cap call succeeds, the acknowledgement is accepted even though P0.2 says replacement is retried for at most 250 ms. The boundary self-test exercises only the predicate and therefore does not catch the loop-order error.

Check the deadline before every post-sleep replacement attempt, or cap the sleep to the remaining time, so no `MoveFileExW` attempt begins at elapsed `>=250 ms`. Add the smallest runnable loop-level check proving a success available only after the cap is rejected; retain immediate failure for non-5/non-32 errors and the unchanged per-trace deadline.

No other actionable finding remains in this amendment-review scope.

## Explicit exclusion

This `CHANGES REQUIRED` verdict applies only to the independent performance-reviewer slot for the P0.2 harness amendment at the exact reviewed commit. The native run is amendment proof, not a replacement calibration or formal P0-E6 evidence. Exit-binding revalidation, formal workload evidence, P0-E6, Phase 0 exit, and the phase-lead slot remain separate and unapproved by this artifact.
