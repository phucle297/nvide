# P0.2 native presentation exit-binding performance review

- Reviewer principal: `agent:/root/p02_exit_binding_performance`
- Role: Independent P0.2 exit-binding performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `19c93f24d346036c09e233dcb9761fbeda7175fb`
- Review scope: only the P0.2 performance-reviewer slot for the native reference host, PresentMon binding, and calibration. This review does not fill the phase-lead slot or approve P0-E6 or Phase 0 exit.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 exit-binding performance reviewer | Reference environment, presentation authority/version/command/hash, calibration integrity and formulas, 120-class eligibility, and binding exclusions | `AGREE` |

## Independent verification

| Check | Result |
| --- | --- |
| Exact revision | PASS — reviewed from a clean detached worktree at `19c93f24d346036c09e233dcb9761fbeda7175fb`; the calibrated implementation commit `e34dc0f279a57b2bd67c5eacebcb5fb68115606e` is its direct executable ancestor and `crates/`, `tools/`, manifests, and lockfile have no intervening change. |
| Archive integrity | PASS — `unzip -t` accepted every member; size is exactly 152,908 bytes and SHA-256 is `727b8d4ab6bb48c81483485725d81d5807920289e8b634fe0fa02bf44be6f6b8`. The archive contains both raw captures, runtime/renderer/capture manifests, stderr, and the refresh-switch script without an unsafe path. |
| Authority identity | PASS — the official Intel GitHub v2.5.1 release asset `PresentMon-2.5.1-x64.exe` independently downloads as 956,768 bytes with SHA-256 `9bec3083069f58f911e6a512f4806db51a27bd096103087bc1d05ef54c80a191`, exactly matching both capture manifests and the binding. |
| Command and v1 semantics | PASS — both manifests retain `--process_id`, `--output_stdout`, `--no_console_stats`, `--qpc_time`, `--v1_metrics`, the timed/session arguments, and QPC frequency `10,000,000`. PresentMon v2.5.1 documents `--qpc_time` and `--v1_metrics`; its v1 contract defines `QPCTime` as the Present-call counter, `msUntilDisplayed` as the display offset, and `Dropped=0` as displayed. |
| Raw schema/binding | PASS — both files have the exact 20-column case-sensitive v1 header and parse without missing/extra columns. Every row is `nvide.exe`, the bound PID and sole swapchain, `Runtime=DXGI`, `SyncInterval=1`, and `AllowsTearing=0`; only `Dropped=0/1` occurs. This supplies fixed FIFO/no-tearing display evidence for the non-VRR binding. |
| Calibrated implementation | PASS — both manifests bind commit `e34dc0f279a57b2bd67c5eacebcb5fb68115606e`, NVide SHA-256 `8283d1070f55dbfdfb659eb6e0fd9a13bb38db38309233d875957f8f121b16dc`, and harness SHA-256 `99bc39abe9ba22a833d68d8b99241513256788a3189b5b2fa1ac09549c09a963`; the latter equals the committed harness. Renderer manifests agree on Vulkan and the AMD Radeon 860M adapter/driver. |
| Independent 60 Hz calculation | PASS — 1,038 rows, 1,033 displayed, 5 dropped; median displayed interval `16.6644500 ms`, nearest-rank p95 `17.4364000 ms`, observed cadence `60.0079810615 Hz`. |
| Independent 120 Hz calculation | PASS — 2,289 rows, 2,284 displayed, 5 dropped; median displayed interval `8.3245000 ms`, nearest-rank p95 `9.1043000 ms`, observed cadence `120.1273349751 Hz`. |
| Eligibility and join tolerance | PASS — the median interval halves under the recorded 60→120 physical-mode change while workload, binary, adapter, harness, and authority remain fixed. `120.127335 Hz` is inside `119.5–120.5 Hz`; its quarter-period is `2.081125 ms`, so the fixed `2 ms` join threshold is strictly smaller. |
| Reference environment | PASS — the binding records the physical model, CPU topology, exact RAM, OS/build, GPU and OS driver, selected wgpu backend/adapter/driver, panel/model/resolution, configured and measured refresh, scale, power scheme, Rust/Cargo versions and target, and Git commit. It also records the unobscured/native-run exclusions required by P0.2. |
| Repository checks | PASS — `cargo fmt --all -- --check`, `cargo xtask schema check`, `cargo xtask evidence check`, `cargo test --workspace --all-targets --all-features --locked` (36 passed, 2 intentional ignored subprocess fixtures), `node --check docs/assets/document.js`, and `git diff --check` all succeeded. |

No actionable finding remains in this exit-binding performance scope.

## Explicit exclusion

This calibration establishes only that the selected authority and timestamp conversion observe the bound display cadence on the eligible reference host. It is not a substitute for the P0.2 measurement-window edge checks, five independent 30-second clear runs, 30 acknowledged edit traces, shaped-text/readback evidence, or their formal two-reviewer P0-E6 verdict. Those remain separate and unapproved by this artifact.
