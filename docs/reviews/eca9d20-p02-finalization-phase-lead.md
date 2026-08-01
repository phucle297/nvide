# P0.2 presentation-finalization amendment — phase-lead re-review

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Phase lead
- Author/implementer principal: `agent:/root`
- Verdict: **CHANGES REQUIRED**
- UTC date: 2026-08-01
- Reviewed commit: `eca9d20e0c828ddc7a9425989d1d8ba7743a47c3`
- Scope: Only the P0.2 presentation-finalization amendment, closure of the `a214984` phase-lead findings, and interactions with the independent performance review's F1–F5. This review does not approve the P0.2 exit binding, P0-E6, or Phase 0 exit.

## Authority checked

- Architecture v0.2.1 ADR process, process/render ownership, rendering pipeline, error policy, performance profile, and ADR-0018 testing gates.
- Accepted ADR-0003, ADR-0021, and ADR-0023.
- Phase 0 `P0-R7`, `P0-A5`, and `P0-E6` in `docs/plan/README.md`.
- `docs/phase-0/P0.2-benchmark-profile.md`, the prior display-acknowledgement approvals, and both `a214984` presentation-finalization review artifacts.

## Checks

- Reviewed a clean detached worktree at exact commit `eca9d20e0c828ddc7a9425989d1d8ba7743a47c3`; it remained clean after all checks.
- Ran `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest` through the Windows host (pass).
- Ran `cargo +1.82.0 fmt --all -- --check` (pass).
- Ran `cargo +1.82.0 test --locked -p nvide-ui -p nvide-render --all-targets` (pass: UI 8 with 2 subprocess fixtures intentionally ignored; render 3).
- Ran `cargo +1.82.0 clippy --locked -p nvide-ui -p nvide-render --all-targets --all-features -- -D warnings` (pass).
- Ran `git diff --check HEAD^ HEAD` (pass).
- Inspected `eca-calibration-60` and `eca-calibration-120`: both record `PASS`, the exact reviewed commit, the exact committed harness SHA-256, PresentMon 2.5.1 and its SHA-256, one PID/swapchain, exact manifest/raw row counts, and `presentmon_exit_code=-1` with `presentmon_stopped_after_application_exit=True`. These directories are diagnostic calibration inputs only, not exit-binding or P0-E6 evidence.
- Inspected the accessible `e28-smoke-edit` bundle. Its one request/ACK/runtime row is internally correlated, but its prior commit and harness SHA make it supplemental only for this review.

## Closure assessment

- **Prior phase-lead finding 1 / F4 path and reuse:** closed. Executable and output paths are canonicalized before use, and an existing evidence directory is rejected before process launch or truncation.
- **Prior finding 2 / F3–F4 request and join integrity:** closed. The harness validates the exact request schema and decimal identities, binds filename to sequence, rejects duplicate trace/changed sequence identity, requires exact unique counts, and revalidates one displayed match against the complete retained capture.
- **Prior finding 3 / F5 cleanup and retention:** the requested stream draining, named-session termination, process wait/kill path, raw/stderr flushing, and capture manifest are implemented. The remaining exit-code acceptance defect below still prevents this interaction from being fail-closed.
- **Prior finding 4 / ADR-0018:** closed. Rust tests cover the one-second stabilization and early-ACK/finalizer ordering; the Windows self-test covers malformed schema/IDs, swapchain mismatch, displayed/dropped parsing, ambiguous joins, exact counts, cleanup, output reuse, and the intentional `-1` branch.
- **F1:** closed. `finalizer_presented` gates ACK consumption, and the focused test proves an early matching ACK cannot finish the trace before the untracked finalizer.
- **F2:** closed for this amendment revision. The harness pins PresentMon 2.5.1 with `--qpc_time --v1_metrics`, requires the exact v1 header, reads `QPCTime`/`msUntilDisplayed`, rejects dropped rows as display evidence, and the supplied native captures exhibit that exact schema.
- **F3:** closed by complete-capture uniqueness revalidation before success.
- **F4:** closed by fresh absolute output plus exact unique request/acknowledgement accounting.

## Remaining finding

### F6 — HIGH — intentional capture-stop handling accepts arbitrary authority failures

`Assert-PresentationExit` accepts every nonzero PresentMon exit code whenever `StoppedAfterApplicationExit` is true. The exact native calibration captures establish only the intentional termination result `-1`; they do not justify treating any other failure code as success. A different PresentMon failure racing after the observed application exit can therefore survive this gate and allow a `PASS` manifest if the other retained checks happen to complete.

Keep exit `0` valid. Accept `-1` only when the harness itself observed the application exit and initiated `Stop-PresentationCapture`; reject every other nonzero code. Extend the self-test so `-1/true` passes, `-1/false` fails, and another nonzero code with `true` also fails.

## Verdict

**CHANGES REQUIRED.** Every `a214984` finding and performance-review interaction is otherwise closed, but the presentation authority's final status is still fail-open for arbitrary nonzero exits. This artifact fills only the phase-lead review slot for this candidate revision. It does not approve the P0.2 exit binding, calibration binding, formal P0-E6 evidence, or Phase 0 exit.
