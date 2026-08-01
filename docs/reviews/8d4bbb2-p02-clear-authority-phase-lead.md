# P0.2 clear authority completion — phase-lead revalidation

- Reviewer principal: `agent:/root/p02_drain_amendment_lead`
- Role: Phase lead, revalidating the same harness-amendment slot
- Author/implementer principal: `agent:/root`
- Verdict: **AGREE**
- UTC date: 2026-08-01
- Reviewed commit: `8d4bbb27306adbbc411759de296b0f635105379b`
- Scope: Successful-clear natural presentation-authority completion and regression of the already-approved edit/failure cleanup behavior only. This review does not approve exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Verification

- Inspected `AGENTS.md`, ADR-0003, ADR-0021, P0.2, the prior harness-amendment approvals, and the exact change from the last exit-binding approval commit `568ccfe132b29e0cfefccaace14798ff0bdc828b`.
- For a successful clear workload, `Should-StopCaptureAfterApplicationExit` always returns false: the harness cannot actively terminate PresentMon and instead reads through its natural `--timed warmup+measure+10 --terminate_after_timed` exit. The ten-second authority margin is outside the unchanged runtime `[s,e)` window and cannot change its markers or accepted samples.
- Successful edit behavior is unchanged: after all required acknowledgements, it still stops after one second of stdout quiet with the existing five-second cap. The predicate's first branch stops authority for every nonzero NVide exit regardless of workload kind. Unexpected PresentMon exit codes remain rejected by `Assert-PresentationExit`.
- Boundary self-tests cover successful clear at the prior five-second drain point (must not stop), failed clear (must stop), and successful edit after quiet drain (must stop). Existing nonzero-authority, quiet/cap, atomic-ACK, and retry-deadline cases remain intact. The Windows PowerShell self-test passes.
- The change adds no dependency or public runtime API and does not modify the clear/edit workload, warmup/measurement counts, formulas, thresholds, join tolerance, finalizer, per-trace deadline, or aggregate edit cap. P0.2 and the binding record correctly invalidate the prior exact harness binding and keep P0-E6 blocked.
- The rejected `formal-568ccfe-clear-01` run independently recalculates to `N=3,542` of `3,600`, missed rate `1.6111%`, and last edge `490.6183 ms`; it therefore cannot be reused even though its capture-level manifest was written before external P0.2 calculation.
- Fresh raw proof `C:\Users\permees\AppData\Local\nvide-phase0\clear-authority-proof-8d4bbb2` is `PASS` and binds exact commit `8d4bbb27306adbbc411759de296b0f635105379b`, Windows-CRLF harness SHA-256 `FE44E372...23BDA490`, PresentMon 2.5.1, PID `3228`, one swapchain, Vulkan/AMD Radeon 860M, and configured 120 Hz.
- Its authority manifest records the required natural disposition: PresentMon exit `0`, `presentmon_stopped_after_application_exit=False`, and `presentmon_post_exit_drain_ms=6727`. The command uses `--timed 50` for the 10-second warmup plus 30-second measurement plus 10-second external authority margin.
- Every one of the 4,791 authority rows uses the bound PID/swapchain, `Runtime=DXGI`, `SyncInterval=1`, and `AllowsTearing=0`; seven dropped rows are excluded. `runtime.csv` contains exactly 3,600 measured frames, sequences 1,192–4,791.
- With `R=8,332,000 ns`, independent decimal calculation gives `N=3,600`, `expected_slots=3,600`, displayed FPS `119.996726647`, missed rate `0`, nearest-rank p99 `9.2157 ms`, first edge `1.1001 ms`, and last edge `6.4151 ms`. Both edges are within `R`; the unchanged FPS and missed-rate rules pass.
- Ran `cargo +1.82.0 fmt --all -- --check`, targeted `nvide-ui` tests (8 pass, 2 intentional subprocess fixtures ignored), targeted Clippy with `-D warnings`, and `git diff --check` (all pass).
- Exact-commit CI [run 30696809503](https://github.com/phucle297/nvide/actions/runs/30696809503) completed successfully across policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64.

No blocking finding remains in this review scope.

## Verdict

**AGREE.** Commit `8d4bbb27306adbbc411759de296b0f635105379b` is acceptable for the successful-clear authority-completion revalidation of the phase-lead P0.2 harness-amendment slot. This fills no performance-reviewer or exit-binding slot and does not approve formal P0-E6 evidence or Phase 0 exit.
