# P0.2 natural-exit binding performance revalidation

- Reviewer principal: `agent:/root/p02_exit_binding_performance`
- Role: Independent P0.2 exit-binding performance reviewer, revalidating the same existing slot
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `d65d2245fd1390ea67b5c771c3e99ac7b569ebc0`
- Review scope: replacement calibration and exit-binding regression after the approved natural clear-authority completion change. This review fills no phase-lead, formal-workload, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 exit-binding performance reviewer | Replacement archive, host/tool/harness/binary identity, calibration calculations, natural authority tail, finalizer safety, and exact-commit CI | `AGREE` |

## Independent verification

- The immutable ZIP `docs/evidence/p0-e6-calibration-6c4542e.zip` is exactly 168,035 bytes with SHA-256 `121d1a0a35a47c3b30c71933c22a3102def9f0719ca0a4d576ee73b6dca6c0ed`; `unzip -t` passes, all 15 members are present, and no member path is unsafe.
- Both raw manifests bind candidate `6c4542eefe14d3d3f9aad28ea3e847c827654023`, NVide SHA-256 `d549e8a3b63e179f4282c208e3de44e413fbf34da12a75e73d70d87f5ba257cc`, reproducible Windows-CRLF harness SHA-256 `fe44e3728e65c2ee76e6d62df4a54203f8925845d0d54c870920a0c423bda490`, and official PresentMon 2.5.1 SHA-256 `9bec3083069f58f911e6a512f4806db51a27bd096103087bc1d05ef54c80a191`. The candidate is an ancestor of the reviewed commit, and no product, harness, workflow, or dependency file differs between them.
- Independent recalculation from the raw v1 CSVs is exact. The 60 Hz run has 1,196 rows, 1,190 displayed, 6 dropped, median `16.6607 ms`, nearest-rank p95 `17.3693 ms`, and cadence `60.0214876926 Hz`. The eligible 120 Hz run has 2,391 rows, 2,384 displayed, 7 dropped, median `R=8.3300 ms`, nearest-rank p95 `8.9897 ms`, and cadence `120.0480192077 Hz`.
- The eligible cadence is inside `119.5–120.5 Hz`. Its quarter-period is `2.0825 ms`, strictly greater than the fixed `2 ms` join threshold.
- Every raw row is bound to `nvide.exe`, exactly one PID and one swapchain per run, `Runtime=DXGI`, `SyncInterval=1`, `PresentFlags=0`, `AllowsTearing=0`, and `Dropped` in `{0,1}`. The manifests record the intended 60/120 Hz modes and the renderer manifests bind the same AMD Radeon 860M Vulkan adapter and driver as the declared physical reference host.
- Both captures completed through PresentMon's natural timed authority exit: `exit_code=0`, `presentmon_stopped_after_application_exit=False`, and post-application-exit drains of `6,636 ms` and `6,621 ms`. This is the approved successful-clear behavior; successful edit capture retains its bounded quiet drain, and nonzero application exit still requests an immediate authority stop.
- The unchanged finalizer wait is sufficient for the eligible calibration: `50 ms > 6R = 49.98 ms`. The largest observed eligible displayed offset is `33.5388 ms`, so the raw stream also supports that bound.
- The recorded host, CPU/RAM, OS/build, GPU/driver, panel/resolution/scale, power scheme, Rust/Cargo toolchain, PresentMon command, prohibited overlay/remote/frame-limiter conditions, and topmost result are complete and mutually consistent. Both manifests record `PASS` and `topmost=True`.
- Local `cargo fmt`, schema check, Phase 0 evidence check, JavaScript syntax check, diff check, and full locked workspace tests pass (36 passed; 2 intentional ignored subprocess fixtures).
- Exact-commit GitHub Actions run `30697220959` completed successfully with all six jobs green: policy, fuzz, Linux x64, Windows x64, macOS x64, and macOS arm64.

No actionable finding remains in this exit-binding performance slot.

## Explicit exclusion

This `AGREE` revalidates only the P0.2 performance-reviewer exit-binding slot at the exact reviewed commit. It does not approve the five formal 30-second clear runs, 30 acknowledged edit traces, shaped-text/readback evidence, formal P0-E6, Phase 0 exit, or the phase-lead slot.
