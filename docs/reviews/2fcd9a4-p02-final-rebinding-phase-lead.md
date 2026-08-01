# P0.2 final exit-binding revalidation — phase-lead review

- Reviewer principal: `agent:/root/p02_exit_binding_lead`
- Role: Phase lead (revalidation of the same existing binding slot)
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `2fcd9a4ead318990288b2dcc1c198c691849441c`
- Review scope: only revalidation of the P0.2 native host/tool/command/calibration binding after the approved atomic-ACK, aggregate-cap, target-preservation, and delayed-finalizer harness amendment. This review does not approve formal workloads, P0-E6, or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 phase lead | Final native exit-binding revalidation | `AGREE` |

## Verification

- Reviewed a clean detached worktree at the exact commit. Candidate implementation `2d9ed1412657dca23f061017fc0ed0f9508b5278` is its direct parent; the reviewed commit changes only P0.2/binding documentation and adds the replacement calibration ZIP.
- `p0-e6-calibration-2d9ed14.zip` is exactly 160,362 bytes, hashes to `4a7c7a0fd94e5dcf27c32e7eb8e263a8ed41e3cd79657e6d16ed4383f2512bf6`, passes `unzip -t`, and contains only the two calibration directories and display-mode script with no unsafe member path.
- The native checkout is clean at `2d9ed1412657dca23f061017fc0ed0f9508b5278`. Its binary hashes to `D549E8A3B63E179F4282C208E3DE44E413FBF34DA12A75E73D70D87F5BA257CC`; the Windows-CRLF harness hash reproduces as `3F033BBB9BEEB1E985FC7E32829A9603CE9855599FA01A0C61DC36DD211E6E54`.
- Read-only host/tool checks match the binding: Lenovo `83HY`; AMD Ryzen AI 7 H 350, 8 cores/16 logical processors; 33,598,853,120-byte RAM; Windows `10.0.26200`; Radeon 860M driver `32.0.22024.3004`; `LEN8BAD` panel at 2880×1800/120 Hz; DPI 168/175%; Balanced power. Renderer manifests bind the recorded Vulkan adapter and driver; the isolated toolchain reports Rust/Cargo 1.97.1.
- The exact PresentMon binary reports version 2.5.1, hashes to `9BEC3083069F58F911E6A512F4806DB51A27BD096103087BC1D05EF54C80A191`, and has a valid Intel Corporation Authenticode signature. Both manifests retain its exact arguments, NVide arguments, QPC frequency, PID, one swapchain, hashes, and exit disposition.
- Both raw files have the exact case-sensitive PresentMon v1 schema. Every row is bound to `nvide.exe`, the recorded PID and sole swapchain, `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, `AllowsTearing=0`, and `Dropped` in `{0,1}`.
- Standard-library recalculation reproduces the binding table: 60 Hz — 1,196 rows, 1,190 displayed, 6 dropped, median 16.6672000 ms, nearest-rank p95 16.7383000 ms, cadence 59.998080 Hz; 120 Hz — 2,392 rows, 2,385 displayed, 7 dropped, median 8.3320000 ms, p95 8.3901000 ms, cadence 120.019203 Hz.
- Both calibration measurement windows retain complete edges: 60 Hz first/last gaps are 15.6066/0.8010 ms, each below `R=16.6672 ms`; 120 Hz gaps are 7.0153/1.0665 ms, each below `R=8.3320 ms`. Manifests record `nvide_window_topmost=True`, PresentMon exit `0`, and bounded post-exit drains of 1,879/1,889 ms.
- The same workload, binary, harness, adapter, authority, and metric mode were held fixed across the physical 60→120 Hz change. Display timestamps use `QPCTime + msUntilDisplayed`, and dropped rows provide no display event; the halved median retains the approved display-versus-submission proof.
- The eligible `R` is 8.3320000 ms and its quarter-period is 2.0830000 ms, strictly greater than the fixed 2 ms join tolerance. The profile's finalizer statement is also exact: the requested 50 ms minimum exceeds six such periods (`6R = 49.992 ms`). The implementation sleeps before its sole finalizer without replacing the target identity, and the already-approved native edit proof observed a larger 53.519 ms minimum target-to-finalizer interval.
- Atomic replacement retries remain limited to Windows errors 5/32, 5 ms spacing, and a strict 250 ms cap consumed inside the unchanged trace deadline. Topmost, successful-exit quiet drain, failed-exit stop, aggregate capture cap, exact request/ACK count, PID/swapchain/schema, and cleanup boundaries remain fail-closed.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest`, `cargo +1.82.0 fmt --all -- --check`, `cargo +1.82.0 test --locked -p nvide-render -p nvide-ui --all-targets` (4 render and 8 UI tests passed; 2 subprocess fixtures ignored), and `git diff --check 2fcd9a4^` passed.
- Exact-commit CI [run 30696486612](https://github.com/phucle297/nvide/actions/runs/30696486612) passed all six jobs; the Windows presentation-harness step passed.

No blocking finding remains in this revalidation scope.

## Explicit exclusion

**AGREE** revalidates only the same phase-lead slot for the P0.2 exit binding at the exact reviewed commit. It does not fill the independent performance-reviewer slot and does not approve the five formal clear runs, measured edit evidence, P0-E6, or Phase 0 exit.
