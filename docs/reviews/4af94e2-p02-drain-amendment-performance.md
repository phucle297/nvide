# P0.2 post-exit authority-drain amendment performance review

- Reviewer principal: `agent:/root/p02_drain_amendment_performance`
- Role: Independent P0.2 performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `4af94e20d9003b067b936bc85575bf405b3b666e`
- Review scope: only the P0.2 post-exit authority-drain and unobscured-window amendment from `f7d90a74284` through the reviewed commit. This review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Successful-exit authority draining, failed-exit behavior, unchanged measurement window/formulas/thresholds, and reproducible unobscured-window setup | `AGREE` |

## Independent verification

| Check | Result |
| --- | --- |
| Exact revision and delta | PASS — the Windows-native proof checkout resolves to `4af94e20d9003b067b936bc85575bf405b3b666e` and is clean according to the same `git.exe` check used by the harness. Relative to `f7d90a74284`, the executable change is confined to the bounded authority drain, its self-test, one topmost-window call, and manifest fields; P0.2 documents the same amendment and invalidates the old binding. |
| Exact proof identity | PASS — `capture-manifest.txt` binds commit `4af94e20d9003b067b936bc85575bf405b3b666e`, NVide SHA-256 `567c91fce6e10c7b9893d2be9259112de77dd475a3e5114d4adc556ef016d293`, and harness SHA-256 `a9fab10fe45d7ba2d1ef7b6f67a03744e904006a5bdbce8fa1288a62ba061a46`; both hashes match the exact Windows checkout. The capture is `PASS`, PresentMon exits `0`, and all 4,776 raw rows are retained. |
| Bound presentation stream | PASS — all raw rows use PID `12264`, one swapchain, `Runtime=DXGI`, `SyncInterval=1`, and `AllowsTearing=0`; the exact PresentMon 2.5.1 v1 header and QPC frequency `10,000,000` are retained. There are 4,772 displayed and 4 dropped rows overall. |
| Independent clear calculation | PASS — with `s=63,679,503,473,900 ns`, `e=63,709,503,473,900 ns`, and approved `R=8,324,500 ns`, the half-open measurement window contains `N=3,600` displayed rows and no dropped row; `expected_slots=3,603`, displayed FPS is `119.996264545`, missed rate is `0.000832639`, and nearest-rank p99 is `9.2920 ms`. |
| Measurement-window edges | PASS — the first displayed event is `3.4804 ms` after `s` and the last is `3.9193 ms` before `e`; both are below `R=8.3245 ms`. The candidate therefore passes the unchanged edge, FPS, and missed-rate rules. |
| Prior-run diagnosis | PASS — independently recomputing the five rejected `f7-clear-*` runs gives end gaps of `0.851–1.521 s` and missed rates of `3.05–5.13%`; every run fails despite FPS remaining at least `119.8259`. They remain rejected and are not reused by this verdict. |
| Bounded drain behavior | PASS — nonzero NVide exit requests immediate authority stop; successful exit waits for one second of stdout quiet and is capped at five seconds. The native proof records a `2,255 ms` post-exit drain and closes both measurement edges without changing runtime markers. Existing strict PresentMon exit validation still accepts only `0`, or intentional `-1` after application exit. |
| Unobscured-window setup | PASS — after resume, the harness waits at most five seconds for the NVide main window and performs checked `SetWindowPos(HWND_TOPMOST, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW)` before capture processing. Failure to find or raise the window aborts the run; overlays and other topmost windows remain prohibited by P0.2. The proof records `nvide_window_topmost=True`. |
| Regression checks | PASS — Windows PowerShell `-SelfTest` passes, including the 999/1,000 ms quiet boundary, nonzero application exit, and 5,000 ms cap. GitHub Actions run `30693956251` completed successfully with all six jobs green: four target jobs, policy, and fuzz. |

No actionable finding remains in this amendment-review scope.

## Scope conclusion

The amendment changes capture finalization and test-environment enforcement only. It does not alter the 10-second warmup, 30-second measurement window, five-run requirement, edit sample count, timestamp conversion, formulas, acceptance thresholds, or runtime measurement markers.

## Explicit exclusion

This `AGREE` verdict fills only the independent performance-reviewer slot for the P0.2 post-exit authority-drain amendment at the exact reviewed commit. The proof run is diagnostic evidence for this amendment, not an exit-binding calibration or formal P0-E6 workload run. Exit-binding revalidation, five formal clear runs, edit evidence, P0-E6, Phase 0 exit, and the phase-lead slot remain separate and unapproved by this artifact.
