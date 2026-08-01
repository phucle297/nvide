# P0.2 natural-exit binding revalidation — phase-lead review

- Reviewer principal: `agent:/root/p02_exit_binding_lead`
- Role: Phase lead (revalidation of the same existing binding slot)
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `d65d2245fd1390ea67b5c771c3e99ac7b569ebc0`
- Review scope: only revalidation of the P0.2 native host/tool/command/calibration binding after the approved successful-clear natural-authority amendment. This review does not approve formal clear or edit evidence, P0-E6, or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 phase lead | Natural-exit native binding revalidation | `AGREE` |

## Verification

- Reviewed a clean detached worktree at the exact commit. Candidate implementation `6c4542eefe14d3d3f9aad28ea3e847c827654023` is its direct parent; the reviewed commit changes only P0.2/binding documentation and adds the replacement calibration ZIP.
- `p0-e6-calibration-6c4542e.zip` is exactly 168,035 bytes, hashes to `121d1a0a35a47c3b30c71933c22a3102def9f0719ca0a4d576ee73b6dca6c0ed`, passes `unzip -t`, and contains only the two calibration directories and display-mode script with no unsafe member path.
- The native checkout is clean at `6c4542eefe14d3d3f9aad28ea3e847c827654023`. Its binary hashes to `D549E8A3B63E179F4282C208E3DE44E413FBF34DA12A75E73D70D87F5BA257CC`; the Windows-CRLF harness hash reproduces as `FE44E3728E65C2EE76E6D62DF4A54203F8925845D0D54C870920A0C423BDA490`.
- Read-only native checks reproduce the binding: Lenovo model `83HY`; 33,598,853,120-byte RAM; Windows `10.0.26200` build `26200`; Radeon 860M driver `32.0.22024.3004`; `LEN8BAD` panel at 2880×1800, 120 Hz, and DPI 168; Balanced power. Renderer manifests bind Vulkan on the AMD Radeon 860M with vendor `0x1002`, device `0x1114`, and AMD proprietary driver `25.20.24.03 (LLPC)`.
- The exact PresentMon binary reports version 2.5.1, hashes to `9BEC3083069F58F911E6A512F4806DB51A27BD096103087BC1D05EF54C80A191`, and has a valid Intel Corporation Authenticode signature. Both manifests retain the exact candidate/harness/binary/tool hashes, arguments, QPC frequency, PID, sole swapchain, and exit disposition.
- Both raw files use the exact case-sensitive PresentMon v1 schema. Every row is bound to `nvide.exe`, the manifest PID, and one swapchain; every row records `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, and `AllowsTearing=0`, while the renderer uses FIFO presentation. `Dropped` is always `0` or `1`.
- Standard-library recalculation exactly reproduces the binding: 60 Hz — 1,196 raw rows, 1,190 displayed, 6 dropped, median 16.6607000 ms, nearest-rank p95 17.3693000 ms, and cadence 60.021488 Hz; 120 Hz — 2,391 raw rows, 2,384 displayed, 7 dropped, median 8.3300000 ms, p95 8.9897000 ms, and cadence 120.048019 Hz.
- The calibration measurement windows retain complete edges. At 60 Hz, the 600 displayed measurement frames occupy 600 expected slots with first/last gaps 3.2507/13.0947 ms, each below `R`; at 120 Hz, 1,200 displayed frames occupy 1,200 slots with gaps 0.3705/7.8662 ms, also below `R`. Both have zero missed slots in their calibration window.
- Both successful clear captures ended by natural timed PresentMon completion: NVide exit is `0`, `presentmon_stopped_after_application_exit=False`, and `presentmon_post_exit_drain_ms` is 6,636 ms at 60 Hz and 6,621 ms at 120 Hz. The last displayed events occur 53.5455 ms and 25.4960 ms after the respective measurement ends, proving the retained raw tails are not harness-truncated.
- The eligible period is `R=8.3300000 ms`; its quarter-period is 2.0825000 ms, strictly greater than the fixed 2 ms join tolerance. The finalizer requirement also remains exact: the requested 50 ms minimum exceeds six eligible periods (`6R=49.9800000 ms`), and the already-approved native edit proof observed a 53.519 ms minimum target-to-finalizer interval.
- Successful clear cannot trigger the harness stop predicate and therefore reads through PresentMon's natural timed exit. Nonzero NVide exit still stops immediately; successful edit retains its separate one-second quiet drain with five-second cap. PID/swapchain/schema, topmost, exact request/ACK count, atomic replacement, per-trace deadline, aggregate cap, and cleanup checks remain fail-closed.
- `powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest`, `cargo +1.82.0 fmt --all -- --check`, `cargo +1.82.0 test --locked -p nvide-render -p nvide-ui --all-targets` (4 render and 8 UI tests passed; 2 subprocess fixtures ignored), targeted Clippy with `-D warnings`, and `git diff --check d65d224^` passed.
- Exact-commit CI [run 30697220959](https://github.com/phucle297/nvide/actions/runs/30697220959) completed successfully with all six jobs green, including the Windows presentation-harness step.

No blocking finding remains in this revalidation scope.

## Explicit exclusion

**AGREE** revalidates only the same phase-lead slot for the P0.2 natural-exit binding at the exact reviewed commit. It does not fill the independent performance-reviewer slot and does not approve the five formal clear runs, measured edit evidence, P0-E6, or Phase 0 exit.
