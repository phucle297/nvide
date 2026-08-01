# P0.2 native presentation exit-binding performance revalidation

- Reviewer principal: `agent:/root/p02_exit_binding_performance`
- Role: Independent P0.2 exit-binding performance reviewer, revalidating the same previously filled slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `19cbe0dfe8ba97d8617c66e39597a49c6783b7d6`
- Review scope: only the native reference host, PresentMon binding, approved drain/topmost harness, and replacement calibration. This review does not fill another role or approve formal workloads, P0-E6, or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 exit-binding performance reviewer | Revalidation after the approved authority-drain/topmost amendment | `CHANGES REQUIRED` |

## Independent verification

| Check | Result |
| --- | --- |
| Exact revision | PASS — reviewed from a clean detached worktree at `19cbe0dfe8ba97d8617c66e39597a49c6783b7d6`. Candidate `3b68fbe7d7fcfcb8288899a37c3630112ea09612` is its executable ancestor and there is no intervening change to crates, harness, manifests, lockfile, or CI workflow. |
| Archive integrity | PASS — `p0-e6-calibration-3b68fbe.zip` passes `unzip -t`, has no unsafe member path, is exactly 170,337 bytes, and hashes to `bc40cb505b4195c7d3992c2d4cbe5b4d24dbc2a88b0ec1ed39a2a88020698790`. |
| Authority stream | PASS — both raw captures use the exact PresentMon v1 header. Every row is `nvide.exe` for one PID and one swapchain with `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, `AllowsTearing=0`, and `Dropped` in `{0,1}`. |
| 60 Hz recalculation | PASS — 1,179 rows, 1,172 displayed, 7 dropped; median `16.6705000 ms`, nearest-rank p95 `17.3956000 ms`, cadence `59.9862031733 Hz`. |
| 120 Hz recalculation | PASS — 2,375 rows, 2,371 displayed, 4 dropped; median `8.3220500 ms`, nearest-rank p95 `9.1056000 ms`, cadence `120.1627002962 Hz`. This is within `119.5–120.5 Hz`; its quarter-period is `2.0805125 ms`, strictly above the `2 ms` join threshold. |
| Harness/tool/binary | PASS — both manifests bind candidate `3b68fbe7...`, NVide SHA-256 `567c91fc...16d293`, PresentMon 2.5.1 SHA-256 `9bec3083...c80a191`, and Windows-CRLF harness SHA-256 `a9fab10f...061a46`; the last reproduces from the committed script. PresentMon exits `0` after retained drains of 2,188/2,326 ms. |
| Drain and unobscured condition | PASS — both manifests record `nvide_window_topmost=True`, successful status, fresh 60/120 modes, and complete authority finalization under the approved bounded drain. P0.2 still prohibits overlays and other topmost windows. |
| Host/environment | PASS except F1 — machine, CPU/RAM, OS/build, GPU/driver, wgpu adapter, panel/resolution/scale, power scheme, toolchain, and native-run exclusions remain recorded. |
| Exact-commit CI | PASS — GitHub Actions run `30694330481` completed successfully at the reviewed SHA with all six jobs green: policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64. |
| Local repository checks | PASS — formatting, schema check, evidence check, JavaScript syntax, and diff checks succeeded. |

## Blocking finding

**F1 — the reference-host measured-refresh fields still point to the superseded archive.** `docs/evidence/p0-e6-binding.md` records `120.127335 Hz` and `60.007981 Hz` in the Reference host table, while the replacement archive and the current Calibration table establish `120.162700 Hz` and `59.986203 Hz`. The stale values are exactly those from `p0-e6-calibration-e34dc0f.zip`, whose approval the same document marks superseded. P0.2 requires the bound reference environment to record configured/measured refresh, so one binding candidate cannot identify two different calibrations as its authoritative measured values.

Update those two Reference host cells to the replacement calibration values and request this same slot to re-review the corrected exact commit. No new native capture or implementation change is required by this finding.

## Explicit exclusion

This verdict covers only exit-binding revalidation. It neither accepts nor rejects the five 30-second clear runs, 30 acknowledged edit traces, shaped-text/readback evidence, formal P0-E6, or Phase 0 exit; those remain separate.
