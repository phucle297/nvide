# P0.2 exit-binding independent phase-lead review

- Reviewer principal: `agent:/root/p02_exit_binding_lead`
- Role: Phase lead
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `19c93f24d346036c09e233dcb9761fbeda7175fb`
- Review scope: only the P0.2 native reference-host, presentation-authority, command, and calibration exit binding. This review does not approve P0-E6 or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 phase lead | Native host/tool/command/calibration exit binding | `AGREE` |

## Verification

- Reviewed a clean detached worktree at the exact commit. `e34dc0f279a57b2bd67c5eacebcb5fb68115606e`, the calibrated implementation, is its direct ancestor and differs only by this binding record, its immutable ZIP, and the P0.2 binding references.
- `p0-e6-calibration-e34dc0f.zip` is 152,908 bytes, has SHA-256 `727b8d4ab6bb48c81483485725d81d5807920289e8b634fe0fa02bf44be6f6b8`, passes `unzip -t`, and contains only the two calibration directories plus the display-mode script; no unsafe archive path exists.
- The retained `nvide.exe` hashes to `8283D1070F55DBFDFB659EB6E0FD9A13BB38DB38309233D875957F8F121B16DC`; its checkout is clean at `e34dc0f279a57b2bd67c5eacebcb5fb68115606e`. The retained harness hash `99BC39ABE9BA22A833D68D8B99241513256788A3189B5B2FA1AC09549C09A963` matches the committed script at both the candidate and reviewed commits.
- Read-only native-host revalidation matched the record: Lenovo `83HY`; AMD Ryzen AI 7 H 350, 8 cores/16 logical processors; 33,598,853,120-byte RAM; Windows 11 Pro `10.0.26200`; Radeon 860M driver `32.0.22024.3004`; `LEN8BAD` internal panel; 2880×1800 at 120 Hz; DPI 168/175%; Balanced power scheme. The renderer manifests bind Vulkan and the recorded adapter/vendor/device/driver values. The isolated toolchain reports the recorded Rust/Cargo 1.97.1 versions.
- The exact local `PresentMon-2.5.1-x64.exe` hashes to `9BEC3083069F58F911E6A512F4806DB51A27BD096103087BC1D05EF54C80A191`, reports PresentMon 2.5.1, and has a valid Intel Corporation Authenticode signature. Both capture manifests retain the exact NVide and PresentMon arguments, QPC frequency, PID, swapchain, hashes, and exit disposition.
- Both raw files have the exact case-sensitive PresentMon v1 header. Every row is `nvide.exe` for the bound PID and sole swapchain, `Runtime=DXGI`, `SyncInterval=1`, `AllowsTearing=0`, and `Dropped` in `{0,1}`. This fixes the effective presentation path to compositor-synchronized, non-tearing output rather than an unbound/VRR submission count.
- Standard-library recalculation reproduced the binding table exactly: 60 Hz has 1,038 rows, 1,033 displayed, 5 dropped, median 16.6644500 ms, p95 17.4364000 ms, and 60.007981 Hz; 120 Hz has 2,289 rows, 2,284 displayed, 5 dropped, median 8.3245000 ms, p95 9.1043000 ms, and 120.127335 Hz.
- The same binary, harness, workload, adapter, authority version, and metric mode were held fixed while the physical mode changed. Display timestamps use `QPCTime + msUntilDisplayed`; displayed rows have positive display offsets while dropped rows supply no display event. The displayed median halves with the physical 60→120 Hz change, supporting the required display-versus-submission calibration.
- The eligible measured period is 8.3245000 ms; one quarter is 2.081125 ms. The fixed 2 ms join tolerance is therefore strictly below the approved quarter-period bound.
- Trust boundaries remain fail-closed: absolute fresh output paths, clean tracked checkout, exact tool filename/version and CSV schema, numeric request parsing, PID/swapchain/FIFO/no-tearing checks, unique sequence/trace and full-capture join checks, exact acknowledgement counts, and bounded authority/application cleanup. `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest` passed.
- `cargo +1.82.0 fmt --all -- --check`, `cargo +1.82.0 test --locked -p nvide-render -p nvide-ui --all-targets` (4 render and 8 UI tests passed; 2 subprocess fixtures ignored), and `git diff --check 19c93f2^` passed. Exact-commit CI run `30692934274` passed all six jobs.

No blocking finding remains in this review scope.

## Explicit exclusion

**AGREE** fills only the phase-lead slot for the P0.2 exit binding at the exact reviewed commit. It does not fill the independent performance-reviewer slot and does not approve the five 30-second clear runs, the 30 measured edit traces, formal P0-E6 evidence, or Phase 0 exit.
