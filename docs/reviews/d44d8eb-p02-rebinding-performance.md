# P0.2 native presentation exit-binding performance final revalidation

- Reviewer principal: `agent:/root/p02_exit_binding_performance`
- Role: Independent P0.2 exit-binding performance reviewer, revalidating the same existing slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `d44d8eb63bbe85345c9f92a638fbb8c9eddacf70`
- Review scope: closure of F1 from `docs/reviews/19cbe0d-p02-rebinding-performance.md` and regression review of the native host/tool/calibration binding after the approved drain/topmost amendment. This review fills no other role and does not approve formal workloads, P0-E6, or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 exit-binding performance reviewer | Corrected reference environment, replacement calibration, authority/tool/harness identity, drain/topmost behavior, and exact-commit CI | `AGREE` |

## Verification

- **F1 closed:** the Reference host table now records `120.162700 Hz` and `59.986203 Hz`, matching the replacement archive and its Calibration table. The only normative edit since `19cbe0dfe8ba97d8617c66e39597a49c6783b7d6` is this two-cell correction; the previous changes-required artifact is retained unchanged.
- The immutable ZIP remains exactly 170,337 bytes with SHA-256 `bc40cb505b4195c7d3992c2d4cbe5b4d24dbc2a88b0ec1ed39a2a88020698790`; `unzip -t` passes and no member path is unsafe.
- Independent recalculation remains exact: 60 Hz has 1,179 rows, 1,172 displayed, 7 dropped, median `16.6705000 ms`, nearest-rank p95 `17.3956000 ms`, and `59.9862031733 Hz`; 120 Hz has 2,375 rows, 2,371 displayed, 4 dropped, median `8.3220500 ms`, p95 `9.1056000 ms`, and `120.1627002962 Hz`.
- The eligible cadence remains inside `119.5–120.5 Hz`; its quarter-period is `2.0805125 ms`, strictly greater than the fixed `2 ms` join threshold.
- Every raw row remains bound to `nvide.exe`, one PID and one swapchain per run, `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, `AllowsTearing=0`, and `Dropped` in `{0,1}` under the exact v1 schema.
- Both manifests still bind candidate `3b68fbe7d7fcfcb8288899a37c3630112ea09612`, NVide SHA-256 `567c91fc...16d293`, PresentMon 2.5.1 SHA-256 `9bec3083...c80a191`, and reproducible Windows-CRLF harness SHA-256 `a9fab10f...061a46`. They record `PASS`, PresentMon exit `0`, topmost success, and bounded post-exit drains of 2,188/2,326 ms.
- The recorded physical host, CPU/RAM, OS/build, GPU/driver, wgpu adapter, panel/resolution/scale, power scheme, toolchain, and prohibited overlay/remote/frame-limiter conditions remain complete and unchanged.
- Local formatting, schema, evidence, JavaScript, diff, and full workspace tests pass (36 passed; 2 intentional ignored subprocess fixtures).
- Exact-commit GitHub Actions run `30694956953` completed successfully with all six jobs green: policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64.

No actionable finding remains in this exit-binding performance slot.

## Explicit exclusion

This `AGREE` revalidates only the P0.2 performance-reviewer exit-binding slot. It does not approve the five 30-second clear runs, 30 acknowledged edit traces, shaped-text/readback evidence, formal P0-E6, Phase 0 exit, or the phase-lead slot.
