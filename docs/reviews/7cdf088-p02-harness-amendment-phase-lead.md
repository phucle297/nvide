# P0.2 harness amendment — corrected phase-lead re-review

- Reviewer principal: `agent:/root/p02_drain_amendment_lead`
- Role: Phase lead, revalidating the same harness-amendment slot
- Author/implementer principal: `agent:/root`
- Verdict: **AGREE**
- UTC date: 2026-08-01
- Reviewed commit: `7cdf0881c8c5c2d340512616a2528ce24b58fb33`
- Scope: Closure of performance finding F1 from `9ed5ae9-p02-harness-amendment-performance.md` plus regression of the same cumulative P0.2 harness amendment. This review does not approve exit binding, formal P0-E6 evidence, or Phase 0 exit.

## F1 closure

- `Should-AttemptAtomicReplace` is now the loop guard before every `MoveFileExW` call. The first attempt remains unconditional, while every post-failure attempt requires elapsed time `<250 ms`; a success first available at or after 250 ms can no longer be called or accepted.
- A retryable error still must be exactly Windows error 5 or 32. Non-retryable errors fail immediately, and reaching the deadline with the previous attempt failed exits the loop and throws without marking the sequence acknowledged.
- Retry sleep remains 5 ms but is capped to the remaining deadline. Boundary self-tests prove that a first attempt at 250 ms is allowed, a post-failure attempt at 249 ms is allowed, and a post-failure attempt at 250 ms is rejected. Existing 5/32, non-retryable-error, and 249/250 ms retry-policy tests remain intact.

Performance F1 is closed.

## Regression checks

- Inspected `AGENTS.md`, ADR-0003, ADR-0021, P0.2, the cumulative amendment, the prior phase-lead artifact, the performance `CHANGES REQUIRED` artifact, and the exact corrective diff from `9ed5ae90a6d216a8b8532ccc5f9a0d9022a25957`.
- The correction changes only `tools/phase0-presentmon.ps1` beyond the two historical review artifacts. It does not change dependencies, public runtime API, workloads, warmup/measurement counts, formulas, thresholds, join tolerance, target/finalizer state, aggregate authority cap, or the single five-second per-trace deadline.
- Ran the committed Windows PowerShell harness self-test, `cargo +1.82.0 fmt --all -- --check`, targeted `nvide-ui` tests (8 pass, 2 intentional subprocess fixtures ignored), targeted Clippy with `-D warnings`, and `git diff --check` (all pass).
- Fresh native proof `C:\Users\permees\AppData\Local\nvide-phase0\ack-finalizer-proof-7cdf088` is `PASS` and binds exact commit `7cdf0881c8c5c2d340512616a2528ce24b58fb33`, Windows-CRLF harness SHA-256 `3F033BBB...11E6E54`, PresentMon 2.5.1, one PID/swapchain, 120 Hz, a 210-second aggregate cap, 190 authority rows, and exactly 40 requests/acknowledgements.
- Request IDs are exactly 1–40 and target sequences are `112,114,…,190`. Each target has one unique displayed authority match within 0.879 ms, is immediately followed by its sole finalizer, and is displayed before that finalizer. Target→finalizer delay is 53.519–95.962 ms; maximum target display wait is 25.526 ms.
- `runtime.csv` contains exactly 30 measured traces with IDs/versions 11–40. All timestamps are ordered; request identity, expected/actual frame, shaping, sentinel pixels, compositor displayed timestamp, and 1920×64 RGBA readback match. All 30 readbacks are non-uniform and consecutive measured readbacks differ.
- Recalculated diagnostic UI-dispatch→version median/p95 as 0.661/1.431 ms and viewport-receive→present median/p95 as 4.058/5.343 ms. These remain diagnostic only.
- Exact-commit CI [run 30696141131](https://github.com/phucle297/nvide/actions/runs/30696141131) completed successfully across policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64.

No blocking finding remains in this review scope.

## Verdict

**AGREE.** Commit `7cdf0881c8c5c2d340512616a2528ce24b58fb33` is acceptable for the corrected phase-lead P0.2 harness-amendment slot. This fills no performance-reviewer or exit-binding slot and does not approve formal P0-E6 evidence or Phase 0 exit.
