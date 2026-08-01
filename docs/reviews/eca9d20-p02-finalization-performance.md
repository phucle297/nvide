# P0.2 presentation-finalization amendment independent performance re-review

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent P0.2 performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `eca9d20e0c828ddc7a9425989d1d8ba7743a47c3`
- Review scope: closure of F1–F5 from `docs/reviews/a214984-p02-finalization-performance.md` for the presentation-finalization amendment only. This review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role. The cancelled `e28f576` review supplied no verdict or artifact.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Stabilization/finalizer ordering, PresentMon 2.5.1 v1 schema and QPC join, full-capture uniqueness, fresh evidence output, cleanup/failure retention, and intentional authority termination | `CHANGES REQUIRED` |

## Verification

| Check | Result |
| --- | --- |
| Exact commit identity | PASS — review used a clean detached worktree at `eca9d20e0c828ddc7a9425989d1d8ba7743a47c3`. |
| Architecture v0.2.1, ADR-0003, ADR-0018, ADR-0021, ADR-0023, Phase 0 plan, P0.2, prior display-ACK approvals, and the `a214984` findings | PASS — scope remains Phase 0-only and the exit binding remains separate and pending. |
| `git diff --check a21498411649fb7b6967d6e27fd24c41a4001901..HEAD` | PASS. |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest` | PASS on Windows PowerShell. |
| `cargo +1.82.0 fmt --all -- --check` | PASS. |
| `cargo +1.82.0 test --locked -p nvide-ui --all-targets` | PASS — 8 passed, 2 subprocess fixtures intentionally ignored. |
| `cargo +1.82.0 test --locked -p nvide-render` | PASS — 3 tests passed. |
| `cargo +1.82.0 clippy --locked -p nvide-ui -p nvide-render --all-targets --all-features -- -D warnings` | PASS. |
| Exact PresentMon release/schema | PASS — official `PresentMon-2.5.1-x64.exe` SHA-256 is `9bec3083069f58f911e6a512f4806db51a27bd096103087bc1d05ef54c80a191`, matching the supplemental captures. Its `--qpc_time --v1_metrics` output and tagged source use `Runtime`, `Dropped`, Present-call `QPCTime`, and `msUntilDisplayed` in the exact header enforced by the harness. |
| Supplemental native captures | PASS for diagnostic consistency only — `eca-calibration-60` and `eca-calibration-120` record the exact reviewed commit and harness hash, the exact v1 header, retained raw rows/stderr, PresentMon exit `-1`, and `presentmon_stopped_after_application_exit=True`. They are not exit-binding calibration or P0-E6 evidence. |
| Fail-closed authority-exit probe | FAIL — invoking the committed `Assert-PresentationExit 5 $true` returns successfully and prints `unexpectedly accepted exit 5`. |

Upstream sources checked:

- <https://github.com/GameTechDev/PresentMon/releases/tag/v2.5.1>
- <https://github.com/GameTechDev/PresentMon/blob/v2.5.1/PresentMon/CsvOutput.cpp>
- <https://github.com/GameTechDev/PresentMon/blob/v2.5.1/README-ConsoleApplication.md>

## Finding closure

- **F1 closed:** each trace records `finalizer_presented`; ACK consumption is blocked until the finalizer has occurred. The focused test places a valid ACK between target and finalizer, proves it cannot advance the trace, then verifies the target sequence/timestamp remain unchanged. A separate test covers the one-second stabilization transition.
- **F2 closed:** the harness pins the executable name to PresentMon 2.5.1, explicitly selects `--qpc_time --v1_metrics`, requires the complete case-sensitive v1 header, treats `QPCTime` as the Present-call QPC, adds `msUntilDisplayed`, and never treats `Dropped=1` as display evidence. The official release asset, tagged source, and supplemental raw captures agree with that contract.
- **F3 closed:** provisional ACKs remain necessary for workload progress, but `Assert-CompleteJoin` rechecks every request against the complete retained capture and requires exactly one displayed match before harness success. The self-test covers one match, a later second match, and missing acknowledgement.
- **F4 closed:** all paths are canonicalized before launch, the evidence directory must not exist, request header/decimal identity/filename/hash/trace uniqueness are validated, and terminal request plus acknowledgement counts must equal the configured edit count.
- **F5 partially closed:** stderr is drained asynchronously; the stdout loop observes NVide exit; exceptional cleanup stops/waits for both processes; raw rows, stderr, commands, hashes, exit status, and a failure manifest are retained. The authority exit-code exception remains over-broad as described below.

## Required change

### F6 — HIGH — intentional-stop handling accepts every nonzero PresentMon exit code

`Assert-PresentationExit` throws only when the exit code is nonzero **and** `StoppedAfterApplicationExit` is false. Once the harness has observed NVide exit and set that flag, the function accepts `1`, `5`, an access-violation code, or any other PresentMon failure just as it accepts the observed intentional `-1`. The manifest records the code but the run is still marked `PASS`, so an authority failure after application exit can falsely satisfy the measurement gate.

Fail closed: accept only exit code `0`, or exactly `-1` when `StoppedAfterApplicationExit` is true. Every other code must fail regardless of the flag. Extend the self-test to reject at least one non-`-1` nonzero code with the flag true; retain the existing cases that accept `-1` only with the flag and reject `-1` without it.

## Explicit exclusion

This `CHANGES REQUIRED` verdict applies only to the independent performance-reviewer slot for the presentation-finalization amendment at the exact reviewed commit. It does **not** approve or reject the native reference host, exit-bound tool command/hash, calibration artifact, P0-E6 evidence, Phase 0 exit, or the phase-lead reviewer slot; all remain separate and pending.
