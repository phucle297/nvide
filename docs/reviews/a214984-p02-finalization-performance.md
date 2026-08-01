# P0.2 presentation-finalization amendment independent performance review

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent P0.2 performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `a21498411649fb7b6967d6e27fd24c41a4001901`
- Review scope: only the P0.2 presentation-finalization amendment, its UI state transitions, and `tools/phase0-presentmon.ps1`; this review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | One-second stabilization, target/finalizer ordering, immutable request and atomic ACK, PresentMon/QPC join, evidence retention, timeout/pass-rule integrity, and harness failure behavior | `CHANGES REQUIRED` |

## Verification

| Check | Result |
| --- | --- |
| Exact commit identity | PASS — review used a clean detached worktree at `a21498411649fb7b6967d6e27fd24c41a4001901`. |
| Architecture v0.2.1, ADR-0003, ADR-0018, ADR-0021, ADR-0023, Phase 0 plan, P0.2, and prior display-ACK approvals | PASS — the proposed change remains Phase 0-only and preserves the separate exit-binding gate. |
| `cargo +1.82.0 fmt --all -- --check` | PASS. |
| `cargo +1.82.0 test --locked -p nvide-ui displayed_ack_gates_the_next_edit_and_readback_must_change` | PASS — 1 focused test passed. |
| `cargo +1.82.0 test --locked -p nvide-render` | PASS — 3 tests passed. |
| `cargo +1.82.0 clippy --locked -p nvide-ui -p nvide-render --all-targets --all-features -- -D warnings` | PASS. |
| `git diff --check HEAD^ HEAD` | PASS. |
| Native PowerShell/PresentMon execution | NOT RUN — this Linux review host has neither Windows nor `pwsh`; the PresentMon findings below are based on the harness's exact command/parser and upstream metric definitions. |

## Confirmed properties

- The first edit is statically gated by one second of repeated empty redraws, and that interval creates no trace/sample.
- In the ordinary no-ACK path, the target frame stores its sequence, post-present QPC-derived runtime timestamp, and readback; one `Continue` requests a second render; the second `presented` call returns `AwaitDisplay` without replacing target fields or requesting another redraw.
- NVide publishes each request through a same-directory temporary file and rename. The harness publishes the ACK through same-directory `MoveFileExW` with replace and write-through flags. The use of `Stopwatch.Frequency` is compatible with Windows QPC frequency semantics, provided the selected CSV field is actually a QPC timestamp for the Present call.
- The approved clear/edit thresholds, measured sample counts, diagnostic status, and the pending native host/tool/calibration and exit-binding approvals remain unchanged.

## Required changes

### F1 — HIGH — an early ACK can skip the required identical finalizing present

After the target `presented` call, `BenchmarkAction::Continue` merely queues a redraw. There is no state bit proving that the finalizing frame was presented. `about_to_wait` calls `display_acknowledged` independently, and that method immediately calls `finish_edit` when a matching ACK exists. If PresentMon can finalize and publish the target without waiting for the queued redraw, the ACK can dispatch the next edit first; that edit changes the renderer contents, so the queued redraw is no longer the required untracked identical finalizer.

Track completion of the finalizing present and refuse to consume an ACK or dispatch another edit before that transition. Add a regression test that installs a valid ACK between the target and finalizer calls and proves that no next edit is dispatched, the finalizer still occurs exactly once with unchanged target identity, and only then may the ACK finish the trace. The one-second pre-edit transition also needs focused coverage under ADR-0018.

### F2 — HIGH — the parser's required columns do not match a documented PresentMon CSV mode

The harness invokes current long-form options with `--qpc_time` but requires both `PresentRuntime` and `TimeInQPC`. PresentMon's current console documentation defines default `--qpc_time` output as `CPUStartQPC`; this is CPU frame-start time, not Present-call time. The same documentation says `--v1_metrics` selects the legacy schema, whose linked 1.x definition uses `Runtime` and `QPCTime` for the Present-call QPC. Thus the harness's `PresentRuntime` + `TimeInQPC` combination matches neither documented mode and will reject real output before it can acknowledge a frame.

Bind and exercise one exact PresentMon version/metric mode. For legacy metrics, explicitly request that mode and parse/validate its actual `Runtime`, `QPCTime`, displayed/dropped fields. For v2 metrics, derive the Present-call origin from documented v2 fields rather than adding a Present-relative `MsUntilDisplayed` offset to `CPUStartQPC`. Preserve the QPC-frequency conversion and prove the exact header plus displayed/dropped fixtures in a runnable harness test.

Upstream definitions checked:

- <https://github.com/GameTechDev/PresentMon/blob/main/README-ConsoleApplication.md>
- <https://github.com/GameTechDev/PresentMon/blob/v1.9.2/README.md>
- <https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.stopwatch>

### F3 — HIGH — the 2 ms uniqueness rule can falsely pass

For every newly read PresentMon row, the harness immediately searches the rows observed so far and ACKs as soon as exactly one displayed candidate exists. It then permanently skips that request. A later row whose Present-call timestamp also lies within the request's ±2 ms window is never checked against the acknowledged request. This is especially relevant here because the finalizing Present may be submitted close to the target and its row can follow the target row in the stream. The run can therefore retain two candidates while claiming the required one-to-one join.

Do not finalize the join until the authority stream has closed the upper side of the 2 ms window, or revalidate every acknowledged request against the complete retained raw capture before allowing the harness to succeed. Any second candidate must fail the run even if the UI already received a provisional ACK. Add fixtures for zero, one, and a later-arriving second candidate.

### F4 — HIGH — output reuse can destroy raw evidence and contaminate request counts

The harness creates `$Output` with `-Force` and opens `presentmon.csv` with overwrite semantics before proving that the directory is a fresh run. Existing request and ACK files are also enumerated, while success requires only `acknowledgements -ge expected`, not exactly one valid unique request/ACK per expected trace. Reusing a path can therefore truncate retained compositor data, mix requests from another attempt, or let extra acknowledgements escape the final count check. NVide's non-replacing request rename may subsequently fail, but by then the prior raw capture has already been modified.

Reject a non-new evidence directory before creating any process or evidence file (or create a new commit/run-addressed directory atomically), validate request filename/body sequence and unique trace identity, and require exactly the configured number of unique acknowledgements. A failed retry must leave the previous bundle byte-for-byte intact.

### F5 — MEDIUM — harness failures are not completely bounded or retained

The main loop synchronously reads PresentMon stdout until its configured timed capture ends. An edit that correctly fails its absolute five-second runtime deadline can therefore take up to the longer capture timer before the harness reports failure. Stderr is read only after normal stdout completion, creating a redirected-pipe deadlock risk, and exceptions do not persist stderr or a failure capture manifest. The `finally` block kills NVide but neither terminates nor waits for PresentMon, so a parse/join failure can leave the named ETW session alive until its timer expires and interfere with a retry.

Drain both streams without pipe deadlock, observe NVide exit/trace failure while capturing, stop and wait for PresentMon on every exceptional path, and always retain flushed raw rows, stderr, exact command/tool identity, and a failure manifest. Preserve the UI's existing single five-second trace deadline; cleanup must not introduce another acceptance window.

## Explicit exclusion

This `CHANGES REQUIRED` verdict applies only to the presentation-finalization amendment at the exact reviewed commit. It does **not** approve or reject the P0.2 native reference host, exact exit-bound tool/version/command, calibration artifact, P0-E6 evidence, Phase 0 exit, or the phase-lead reviewer slot; all of those remain separate and pending.
