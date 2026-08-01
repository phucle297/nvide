# P0.2 final native presentation exit-binding performance revalidation

- Reviewer principal: `agent:/root/p02_exit_binding_performance`
- Role: Independent P0.2 exit-binding performance reviewer, revalidating the same existing slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `2fcd9a4ead318990288b2dcc1c198c691849441c`
- Review scope: the replacement native host/tool/calibration binding after the approved atomic-ACK, capture-cap, target-preservation, and delayed-finalizer amendment. This review fills no other role and does not approve formal workloads, P0-E6, or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 exit-binding performance reviewer | Replacement calibration integrity, reference environment, authority/tool/harness identity, topmost/drain behavior, finalizer timing, and exact-commit CI | `AGREE` |

## Independent verification

| Check | Result |
| --- | --- |
| Exact revision | PASS — reviewed from a clean detached worktree at `2fcd9a4ead318990288b2dcc1c198c691849441c`; candidate `2d9ed1412657dca23f061017fc0ed0f9508b5278` is its executable ancestor with no intervening source, harness, manifest, lockfile, or CI-workflow change. |
| Archive integrity | PASS — `p0-e6-calibration-2d9ed14.zip` passes `unzip -t`, has no unsafe member path, is exactly 160,362 bytes, and hashes to `4a7c7a0fd94e5dcf27c32e7eb8e263a8ed41e3cd79657e6d16ed4383f2512bf6`. |
| 60 Hz recalculation | PASS — 1,196 rows, 1,190 displayed, 6 dropped; median `16.6672000 ms`, nearest-rank p95 `16.7383000 ms`, cadence `59.9980800614 Hz`. |
| 120 Hz recalculation | PASS — 2,392 rows, 2,385 displayed, 7 dropped; median `8.3320000 ms`, nearest-rank p95 `8.3901000 ms`, cadence `120.0192030725 Hz`. |
| Eligibility and join tolerance | PASS — `120.019203 Hz` is inside `119.5–120.5 Hz`; measured `R=8.3320000 ms`, so `R/4=2.0830000 ms` remains strictly greater than the `2 ms` join threshold. The Reference host table records the same 60/120 values. |
| Authority binding | PASS — both raw files use the exact 20-column PresentMon v1 schema. Every row is `nvide.exe` for one PID and one swapchain per run with `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, `AllowsTearing=0`, and `Dropped` in `{0,1}`. |
| Host/tool/binary/harness | PASS — both manifests bind candidate `2d9ed141...`, NVide SHA-256 `d549e8a3...a257cc`, PresentMon 2.5.1 SHA-256 `9bec3083...c80a191`, and reproducible Windows-CRLF harness SHA-256 `3f033bbb...e6e54`. Renderer manifests agree on the bound Vulkan AMD Radeon 860M adapter and driver. |
| Topmost and drain | PASS — both captures record `status=PASS`, `nvide_window_topmost=True`, PresentMon exit `0`, and successful bounded post-exit drains of 1,879/1,889 ms. Host, display, OS, scale, power, toolchain, and prohibited overlay/remote/frame-limiter conditions remain complete. |
| Finalizer timing | PASS — the eligible period gives `6R=49.9920000 ms`; Rust's requested `50 ms` minimum delay is therefore greater by `0.008 ms`. The 120 Hz calibration's largest displayed-frame offset is `32.9848 ms`, also below the delay. The approved path preserves the target identity and submits exactly one unchanged finalizer. |
| Local checks | PASS — format, schema generation check, `cargo xtask evidence check --phase 0`, JavaScript syntax, diff, and full workspace tests succeeded (36 passed; 2 intentional ignored subprocess fixtures). |
| Exact-commit CI | PASS — GitHub Actions run `30696486612` completed successfully with all six jobs green: policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64. |

No actionable finding remains in this exit-binding performance slot.

## Explicit exclusion

This `AGREE` revalidates only the P0.2 performance-reviewer exit-binding slot. It does not approve the five 30-second clear runs, 30 acknowledged edit traces, shaped-text/readback evidence, formal P0-E6, Phase 0 exit, or the phase-lead slot.
