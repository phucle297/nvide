# P0.2 native presentation binding revalidation candidate

- Status: EXIT BINDING REVALIDATION APPROVED; NOT P0-E6 EVIDENCE
- Candidate implementation commit: `2d9ed1412657dca23f061017fc0ed0f9508b5278`
- Owner: Performance owner
- Prepared by: `agent:/root`
- Calibration artifact: [`p0-e6-calibration-2d9ed14.zip`](p0-e6-calibration-2d9ed14.zip)
- Artifact SHA-256: `4a7c7a0fd94e5dcf27c32e7eb8e263a8ed41e3cd79657e6d16ed4383f2512bf6`
- Artifact size: 160,362 bytes

This record binds only the reference host, presentation authority, exact command, and calibration required by P0.2. It does not approve or contain the five 30-second clear runs, the 30 measured edit traces, formal P0-E6 evidence, or Phase 0 exit.

## Reference host

| Field | Bound value |
| --- | --- |
| Physical machine | Lenovo model `83HY` |
| CPU | AMD Ryzen AI 7 H 350 with Radeon 860M; 8 cores / 16 logical processors |
| RAM | 33,598,853,120 bytes |
| OS | Microsoft Windows 11 Pro `10.0.26200`, build `26200` |
| GPU / OS driver | AMD Radeon 860M Graphics / `32.0.22024.3004` |
| NVide-selected wgpu adapter | Vulkan; AMD Radeon 860M; vendor `0x1002`; device `0x1114`; integrated GPU; AMD proprietary driver `25.20.24.03 (LLPC)` |
| Physical display | Lenovo internal panel `LEN8BAD`; 2880×1800; Windows scale 175% (`AppliedDPI=168`) |
| Eligible mode | 120 Hz configured; authority-observed 120.019203 Hz |
| Comparison mode | 60 Hz configured; authority-observed 59.998080 Hz |
| Power | Windows Balanced scheme `381b4222-f694-41f0-9685-ff5bb260df2e` |
| Toolchain | `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, host `x86_64-pc-windows-msvc` |

The physical panel was unobscured. Remote desktop, screen recording, overlays, battery saver, and application frame limiters were not active. The evidence harness created a fresh directory for each run and rejected tracked checkout changes.

## Presentation authority

- Tool: official Intel PresentMon `2.5.1`, file `PresentMon-2.5.1-x64.exe`.
- SHA-256: `9BEC3083069F58F911E6A512F4806DB51A27BD096103087BC1D05EF54C80A191`.
- Authenticode: valid; signer `Intel Corporation`.
- Metric mode: `--qpc_time --v1_metrics`.
- Required fields: `Runtime`, `Dropped`, `QPCTime`, and `msUntilDisplayed` from the exact case-sensitive v1 header retained in both raw captures.
- Binding: one NVide PID, one swapchain, `Runtime=DXGI`, `SyncInterval=1`, `AllowsTearing=0`; `Dropped=1` is not a displayed event.

The harness command template is:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\phase0-presentmon.ps1 -Kind <clear|edit> -RunId <fresh-id> -Output <fresh-absolute-directory> -NvideExe target\release\nvide.exe -PresentMonExe C:\Users\permees\AppData\Local\nvide-phase0\PresentMon-2.5.1-x64.exe <workload arguments>
```

The harness resolves every path to an absolute path, records executable/harness hashes and arguments, starts NVide suspended, binds PresentMon by PID, then resumes NVide. It waits at most five seconds for NVide's window and makes it topmost once; overlays and other topmost windows remain prohibited. It retains raw CSV, stderr, runtime and renderer manifests, and a success/failure capture manifest. A nonzero NVide exit stops authority capture immediately. A successful exit drains already-produced authority stdout until one second of quiet, capped at five seconds. Success requires PresentMon exit `0`, or exactly `-1` only after the harness observes NVide exit and intentionally terminates the named capture session.

## Calibration

Both calibration runs used the exact candidate checkout and binary hash `D549E8A3B63E179F4282C208E3DE44E413FBF34DA12A75E73D70D87F5BA257CC`, ten seconds of warmup, and ten seconds of capture. Displayed timestamps are `QPCTime / 10,000,000 + msUntilDisplayed / 1,000`; intervals are calculated after sorting displayed timestamps. Nearest-rank p95 is reported.

| Configured mode | Raw rows | Displayed | Dropped | Median displayed interval | p95 displayed interval | Observed cadence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 60 Hz | 1,196 | 1,190 | 6 | 16.6672000 ms | 16.7383000 ms | 59.998080 Hz |
| 120 Hz | 2,392 | 2,385 | 7 | 8.3320000 ms | 8.3901000 ms | 120.019203 Hz |

The median interval halves when the physical display changes from 60 to 120 Hz, while the same NVide workload, candidate binary, wgpu adapter, harness, authority version, and metric mode remain fixed. This demonstrates that the authority timestamps track display cadence rather than submission cadence. The eligible 120 Hz cadence is within P0.2's 119.5–120.5 Hz range. Its quarter-period is 2.0830000 ms, so the fixed 2 ms join tolerance remains below one quarter of the measured period.

Recalculate the table from the extracted artifact with Python 3 standard library:

```bash
python3 - <<'PY'
import csv, statistics
from pathlib import Path
for name in ("calibration-2d9ed14-60", "calibration-2d9ed14-120"):
    with Path(name, "presentmon.csv").open(newline="", encoding="utf-8-sig") as stream:
        rows = list(csv.DictReader(stream))
    displayed = sorted(int(row["QPCTime"]) / 10_000_000 + float(row["msUntilDisplayed"]) / 1_000 for row in rows if row["Dropped"] == "0")
    intervals = sorted((end - start) * 1_000 for start, end in zip(displayed, displayed[1:]))
    p95 = intervals[(95 * len(intervals) + 99) // 100 - 1]
    print(name, len(rows), len(displayed), sum(row["Dropped"] == "1" for row in rows), statistics.median(intervals), p95, 1_000 / statistics.median(intervals))
PY
```

## Superseded exit-binding review record

The amendment changed the harness hash after this approval, so these verdicts are retained only as history and no longer authorize P0-E6.

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Phase lead | `agent:/root/p02_exit_binding_lead` | AGREE | 2026-08-01 | `19c93f24d346036c09e233dcb9761fbeda7175fb` | [`19c93f2-p02-exit-binding-phase-lead.md`](../reviews/19c93f2-p02-exit-binding-phase-lead.md) |
| Performance reviewer | `agent:/root/p02_exit_binding_performance` | AGREE | 2026-08-01 | `19c93f24d346036c09e233dcb9761fbeda7175fb` | [`19c93f2-p02-exit-binding-performance.md`](../reviews/19c93f2-p02-exit-binding-performance.md) |

## Superseded exit-binding revalidation record

The atomic-ACK amendment changes the approved harness hash, so these verdicts no longer authorize P0-E6 and are retained only as history.

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Phase lead | `agent:/root/p02_exit_binding_lead` | AGREE | 2026-08-01 | `d44d8eb63bbe85345c9f92a638fbb8c9eddacf70` | [`d44d8eb-p02-rebinding-phase-lead.md`](../reviews/d44d8eb-p02-rebinding-phase-lead.md) |
| Performance reviewer | `agent:/root/p02_exit_binding_performance` | AGREE | 2026-08-01 | `d44d8eb63bbe85345c9f92a638fbb8c9eddacf70` | [`d44d8eb-p02-rebinding-performance.md`](../reviews/d44d8eb-p02-rebinding-performance.md) |

## Current exit-binding revalidation record

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Phase lead | `agent:/root/p02_exit_binding_lead` | AGREE | 2026-08-01 | `2fcd9a4ead318990288b2dcc1c198c691849441c` | [`2fcd9a4-p02-final-rebinding-phase-lead.md`](../reviews/2fcd9a4-p02-final-rebinding-phase-lead.md) |
| Performance reviewer | `agent:/root/p02_exit_binding_performance` | AGREE | 2026-08-01 | `2fcd9a4ead318990288b2dcc1c198c691849441c` | [`2fcd9a4-p02-final-rebinding-performance.md`](../reviews/2fcd9a4-p02-final-rebinding-performance.md) |
