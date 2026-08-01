# P0.2 post-exit authority-drain amendment — phase-lead review

- Reviewer principal: `agent:/root/p02_drain_amendment_lead`
- Role: Phase lead
- Author/implementer principal: `agent:/root`
- Verdict: **AGREE**
- UTC date: 2026-08-01
- Reviewed commit: `4af94e20d9003b067b936bc85575bf405b3b666e`
- Scope: P0.2 post-exit authority drain and unobscured-window enforcement only. This review does not approve the exit-binding revalidation, formal P0-E6 evidence, or Phase 0 exit.

## Checks

- Inspected `AGENTS.md`, the Architecture Phase 0 rendering milestone, ADR-0003, ADR-0021, the approved P0.2 profile, and the exact two-file delta from `f7d90a74284` through the reviewed commit.
- Confirmed the clear/edit workloads, measurement markers, sample counts, formulas, `R`, `displayed_fps >= 119`, `missed_rate <= 0.005`, edge checks, and acceptance rules are unchanged. P0.2 explicitly rejects the five truncated `f7-clear-*` runs and resets both amendment and exit-binding review records to `PENDING`.
- Ran the exact committed harness self-test through Windows PowerShell (pass). Its drain fixtures cover quiet time at 999/1,000 ms, immediate stop on nonzero NVide exit, and the 5,000 ms cap.
- Verified the successful-exit path drains stdout until one second of quiet with a five-second cap. A nonzero NVide exit stops capture immediately; unexpected PresentMon exits remain rejected by `Assert-PresentationExit`, so the amendment does not broaden authority success conditions.
- Verified `SetWindowPos(HWND_TOPMOST, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW)` is attempted after the benchmark window appears and before the harness consumes the capture. Missing window, early NVide exit, timeout, or API failure fails the run. Other topmost windows and overlays remain prohibited by P0.2.
- Independently recalculated the rejected `f7-clear-01` through `f7-clear-05` raw captures. Their last-edge gaps are 851.21–1,520.64 ms and missed rates are 3.053–5.135%, so none is reusable.
- Independently recalculated native diagnostic proof `C:\Users\permees\AppData\Local\nvide-phase0\drain-proof-4af94e2` from raw PresentMon data using the bound `R = 8,324,500 ns`: `N=3,600`, `expected_slots=3,603`, displayed FPS `119.996265`, missed rate `0.000833`, first edge `3.4804 ms`, last edge `3.9193 ms`, and nearest-rank p99 `9.2920 ms`. Both edges are within `R`, and the proof passes the unchanged FPS/missed thresholds.
- The proof manifest binds commit `4af94e20d9003b067b936bc85575bf405b3b666e`, PresentMon 2.5.1 hash `9BEC3083...C80A191`, 120 Hz, one PID/swapchain, topmost success, and a bounded 2,255 ms post-exit drain. The Windows CRLF harness hash `A9FAB10F...061A46` reproduces from the reviewed Git blob.
- Exact-commit CI [run 30693956251](https://github.com/phucle297/nvide/actions/runs/30693956251) completed successfully: policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64 passed; the Windows presentation-harness step passed.
- Ran `git diff --check` and confirmed the amendment changes only `tools/phase0-presentmon.ps1` and `docs/phase-0/P0.2-benchmark-profile.md` (pass).

No blocking finding remains in this review scope.

## Verdict

**AGREE.** Commit `4af94e20d9003b067b936bc85575bf405b3b666e` is acceptable for the phase-lead slot of the P0.2 post-exit authority-drain amendment. This verdict fills no performance-reviewer or exit-binding slot and does not approve formal P0-E6 evidence or Phase 0 exit.
