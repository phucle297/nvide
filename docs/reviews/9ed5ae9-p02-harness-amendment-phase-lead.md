# P0.2 harness amendment — phase-lead revalidation

- Reviewer principal: `agent:/root/p02_drain_amendment_lead`
- Role: Phase lead
- Author/implementer principal: `agent:/root`
- Verdict: **AGREE**
- UTC date: 2026-08-01
- Reviewed commit: `9ed5ae90a6d216a8b8532ccc5f9a0d9022a25957`
- Scope: Revalidation of the same P0.2 harness-amendment slot for atomic acknowledgement retry, delayed exact finalizer, target preservation, and aggregate authority cap only. This review does not approve the replacement exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Checks

- Inspected `AGENTS.md`, the Architecture Phase 0 rendering milestone, ADR-0003, ADR-0021, P0.2, and the exact cumulative changes through commits `4ad7bcf`, `cdeaf18`, `20c6b64`, and `9ed5ae9`.
- Confirmed no dependency, public runtime API, clear/edit workload, warmup/measurement count, formula, threshold, join tolerance, or five-second per-trace deadline changed. The changes are confined to private benchmark state, the Windows evidence harness, and the records that keep exit binding and P0-E6 blocked.
- Verified atomic acknowledgement replacement retries only Windows errors 5 and 32, sleeps 5 ms between attempts, and stops retry eligibility at 250 ms. Any other error or an exhausted cap throws; the sequence is marked acknowledged only after `MoveFileExW` succeeds. The Windows self-test covers errors 5/32, the 249/250 ms boundary, and rejection of error 2 (pass).
- Verified the target frame sequence, present timestamp, readback, and request identity are recorded once. The next render only marks `finalizer_presented` and does not overwrite target fields. The event loop waits 50 ms before requesting that one unchanged finalizer and dispatches no next edit until the matching displayed acknowledgement. The unit test preserves target `(sequence=2, present_ns=100)` after finalizer sequence 3 and rejects an acknowledgement after the original five-second deadline.
- Verified PresentMon's edit safety cap is `5 seconds × requested edits + 10 seconds`; the default proof command therefore records 210 seconds. Rust still derives core, render/readback, acknowledgement, and timeout checks from the single `trace.started + 5 seconds` deadline, so the aggregate cap cannot extend a trace.
- Inspected the rejected native runs. Both `formal-44b0f54-edit-*` runs failed closed on Windows error 5 after two acknowledgements; `ack-retry-proof-4ad7bcf` failed closed after four acknowledgements; and `ack-finalizer-proof-cdeaf18` failed closed after eight. P0.2 identifies all of them as rejected and non-reusable.
- Independently checked native proof `C:\Users\permees\AppData\Local\nvide-phase0\ack-finalizer-proof-9ed5ae9`. Its manifest is `PASS`, binds the exact reviewed commit and PresentMon 2.5.1, records one PID/swapchain, 192 authority rows, exactly 40 requests and 40 acknowledgements, a 210-second aggregate cap, and bounded post-exit cleanup.
- Request IDs are exactly 1–40 and target sequences are exactly `114,116,…,192`. Every target has one unique displayed authority match within 0.741 ms; the immediately following authority row is its sole finalizer, the next target follows immediately after that finalizer, and every target is displayed before its finalizer. Target-to-finalizer delay is 52.240–96.874 ms; maximum target display wait is 24.106 ms.
- `runtime.csv` contains exactly the 30 measured traces, IDs/versions 11–40. Every row has ordered dispatch→core→version→viewport→present→display timestamps, matching expected/actual target sequence and immutable request timestamp, shaped sentinel/glyph count, compositor displayed timestamp, sentinel pixels, and a 1920×64 RGBA readback. All 30 readbacks are non-uniform and consecutive measured readbacks differ. No trace is incomplete or stale.
- Recalculated diagnostic UI-dispatch→version median/p95 as 0.911/1.571 ms and viewport-receive→present median/p95 as 4.067/6.545 ms. These remain diagnostic, not newly introduced blockers.
- Ran `cargo +1.82.0 fmt --all -- --check`, targeted `nvide-ui` tests (8 pass, 2 intentional subprocess fixtures ignored), targeted Clippy with `-D warnings`, the committed Windows PowerShell harness self-test, and `git diff --check` (all pass).
- Exact-commit CI [run 30695811534](https://github.com/phucle297/nvide/actions/runs/30695811534) completed successfully across policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64; the Windows job succeeded.

No blocking finding remains in this review scope.

## Verdict

**AGREE.** Commit `9ed5ae90a6d216a8b8532ccc5f9a0d9022a25957` is acceptable for revalidation of the phase-lead P0.2 harness-amendment slot. This fills no performance-reviewer or exit-binding slot and does not approve formal P0-E6 evidence or Phase 0 exit.
