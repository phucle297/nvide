# P0.2 exit-binding revalidation — phase-lead review

- Reviewer principal: `agent:/root/p02_exit_binding_lead`
- Role: Phase lead (revalidation of the existing binding slot)
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `d44d8eb63bbe85345c9f92a638fbb8c9eddacf70`
- Review scope: only revalidation of the P0.2 native host/tool/command/calibration binding after the approved topmost-window and post-exit authority-drain amendment. This review does not approve formal workloads, P0-E6, or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 phase lead | Native exit-binding revalidation | `AGREE` |

## Verification

- Reviewed a clean detached worktree at the exact commit. Relative to the changes-required candidate `19cbe0dfe8ba97d8617c66e39597a49c6783b7d6`, the only binding change corrects the two stale measured-refresh cells; the committed historical performance review records that finding.
- F1 is closed: the reference-host table now records 120.162700 Hz and 59.986203 Hz, exactly matching the replacement calibration table and independent raw-data recalculation. The superseded 120.127335/60.007981 values no longer appear in the candidate binding.
- `p0-e6-calibration-3b68fbe.zip` is unchanged, is exactly 170,337 bytes, hashes to `bc40cb505b4195c7d3992c2d4cbe5b4d24dbc2a88b0ec1ed39a2a88020698790`, passes `unzip -t`, and has no unsafe member path.
- The native checkout is clean at candidate `3b68fbe7d7fcfcb8288899a37c3630112ea09612`. Its binary hashes to `567C91FCE6E10C7B9893D2BE9259112DE77DD475A3E5114D4ADC556EF016D293`; the Windows-CRLF harness hash reproduces as `A9FAB10FE45D7BA2D1EF7B6F67A03744E904006A5BDBCE8FA1288A62BA061A46`.
- Read-only host/tool checks match the record: Lenovo `83HY`, 33,598,853,120-byte RAM, Windows `10.0.26200`, Radeon 860M driver `32.0.22024.3004`, 2880×1800/120 Hz, DPI 168/175%, and Balanced power. The exact PresentMon 2.5.1 binary hashes to `9BEC3083069F58F911E6A512F4806DB51A27BD096103087BC1D05EF54C80A191` and has a valid Intel Corporation Authenticode signature; renderer manifests bind the recorded Vulkan adapter and driver.
- Both raw captures have the exact PresentMon v1 header. Every row is bound to `nvide.exe`, the recorded PID and sole swapchain, `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, `AllowsTearing=0`, and `Dropped` in `{0,1}`.
- Standard-library recalculation gives: 60 Hz — 1,179 rows, 1,172 displayed, 7 dropped, median 16.6705000 ms, p95 17.3956000 ms, cadence 59.986203 Hz; 120 Hz — 2,375 rows, 2,371 displayed, 4 dropped, median 8.3220500 ms, p95 9.1056000 ms, cadence 120.162700 Hz.
- The eligible 120 Hz cadence is within 119.5–120.5 Hz. Its quarter-period is 2.0805125 ms, so the fixed 2 ms edit join remains strictly below the approved bound. The unchanged mode-switch data retains the approved display-versus-submission proof.
- Both manifests record `nvide_window_topmost=True`, PresentMon exit `0`, and bounded post-exit drains of 2,188/2,326 ms. Last displayed events remain within one period of the measurement end: 14.1829 ms at 60 Hz and 5.5022 ms at 120 Hz. Topmost timeout/API failures and drain exit/quiet/cap boundaries remain fail-closed.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest`, `cargo +1.82.0 fmt --all -- --check`, `cargo +1.82.0 test --locked -p nvide-render -p nvide-ui --all-targets` (4 render and 8 UI tests passed; 2 subprocess fixtures ignored), and `git diff --check d44d8eb^` passed.
- Exact-commit CI [run 30694956953](https://github.com/phucle297/nvide/actions/runs/30694956953) passed all six jobs; the Windows presentation-harness step passed.

No blocking finding remains in this revalidation scope.

## Explicit exclusion

**AGREE** revalidates only the existing phase-lead slot for the P0.2 exit binding at the exact reviewed commit. It does not fill the independent performance-reviewer slot and does not approve the five formal clear runs, measured edit evidence, P0-E6, or Phase 0 exit.
