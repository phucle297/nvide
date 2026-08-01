# P0.2 presentation-finalization amendment — phase-lead review

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Phase lead
- Author/implementer principal: `agent:/root`
- Verdict: **CHANGES REQUIRED**
- UTC date: 2026-08-01
- Reviewed commit: `a21498411649fb7b6967d6e27fd24c41a4001901`
- Scope: Only the P0.2 presentation-finalization amendment, its Phase 0 Rust state machine/tests, and `tools/phase0-presentmon.ps1`. This review does not approve the P0.2 exit binding, P0-E6, or Phase 0 exit.

## Authority checked

- Architecture v0.2.1 ADR process, process/render ownership, rendering pipeline, error policy, performance profile, and ADR-0018 testing gates.
- Accepted ADR-0003, ADR-0021, and ADR-0023.
- Phase 0 `P0-R7`, `P0-A5`, and `P0-E6` in `docs/plan/README.md`.
- `docs/phase-0/P0.2-benchmark-profile.md` at the reviewed commit.
- The approved display-acknowledgement amendment and both reviews at `5b2e972f188f52a8f4f37b127737262224e7a2d9`.

## Checks

- Inspected the exact commit and traced stabilization, target present, immutable request publication, one finalizing present, ACK polling, the shared five-second deadline, and next-edit dispatch.
- Confirmed the target frame remains the recorded/read-back frame and the finalizing frame cannot replace its sequence, timestamp, or pixels.
- Ran `cargo +1.82.0 test --locked -p nvide-ui displayed_ack_gates_the_next_edit_and_readback_must_change` (pass).
- Ran `cargo test -p nvide-ui --all-targets --locked` (pass: 7, with 2 subprocess fixtures intentionally ignored).
- Ran `cargo +1.82.0 clippy --locked -p nvide-ui --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check` (pass).
- Ran `git diff --check` for the reviewed commit (pass).
- Performed static trust/error-boundary review of `tools/phase0-presentmon.ps1`; PowerShell and native PresentMon execution were unavailable on this review host.

## Findings

1. **Relative paths split the runtime and harness evidence directories.** The script uses `$Output` relative to its own current directory, but starts NVide with the executable directory as the child current directory while forwarding the same unqualified value. The documented relative form such as `target/phase0-evidence/edit-01` therefore makes the harness poll one directory and NVide publish requests/ACK input in another. `$NvideExe` is also used to derive the repository root before it is canonicalized. Resolve all executable/output paths once to absolute paths before validation, process launch, file polling, hashing, and manifest recording, or reject non-absolute inputs explicitly.

2. **Display-request validation and acknowledgement accounting do not fail closed.** The harness checks only row count and PID before trusting a request. It does not require the exact header, validate decimal `trace_id`, require the filename sequence to equal `frame_sequence`, or reject a sequence repeated under another filename. Because acknowledgements are tracked by file path and the terminal check accepts any count greater than or equal to the requested total, duplicate request files can inflate the count and mask a missing edit. Validate the complete one-row schema, bind filename/row identity, enforce one acknowledgement per unique requested sequence, and require the exact expected acknowledgement count.

3. **Harness failure can leave the presentation authority running.** The `finally` block kills NVide but never terminates/waits for `$presentMon`. Any malformed row, request, swapchain change, or atomic-write error can leave the timed PresentMon process and named ETW session active, making retries collide and contaminating capture state. Terminate and wait/dispose PresentMon on every exceptional path before returning failure.

4. **The new contract is not fully regression-tested as required by ADR-0018.** The Rust test covers target/finalizing-frame ordering and request publication, but does not exercise the one-second no-edit stabilization branch. The new 369-line harness has no deterministic parser/state test for invalid headers/IDs, duplicate sequences, ambiguous matches, changed swapchains, exact acknowledgement count, or cleanup. Add the smallest focused Rust state test plus a Windows-runnable harness fixture/check that fails if these trust boundaries regress.

The amendment remains within Phase 0 ownership and does not change product/runtime APIs, thresholds, or sample counts. The findings are confined to evidence correctness, deterministic execution, and required test coverage.

## Verdict

**CHANGES REQUIRED.** The Rust finalizing-present transition is directionally correct and its existing focused test passes, but the bound harness can separate its file channel from NVide, accept/account malformed duplicate requests, and leak the presentation process after failure. This artifact fills only the phase-lead review slot for this candidate revision. It does not approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.
