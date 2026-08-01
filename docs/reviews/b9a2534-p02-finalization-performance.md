# P0.2 presentation-finalization amendment independent performance final re-review

- Reviewer principal: `agent:/root/phase0_impl_reviewer2`
- Role: Independent P0.2 performance reviewer
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `b9a2534457c4b7b6aa6006b2a06bcd27a8b83bf2`
- Review scope: closure of F6 from `docs/reviews/eca9d20-p02-finalization-performance.md` plus regression review of F1–F5 from `docs/reviews/a214984-p02-finalization-performance.md`, for the presentation-finalization amendment only. This review fills no phase-lead, exit-binding, P0-E6, or Phase 0 exit role.

## Verdict

| Role | Scope | Verdict |
| --- | --- | --- |
| P0.2 independent performance reviewer | Stabilization/finalizer ordering, PresentMon 2.5.1 v1/QPC join, full-capture uniqueness, fresh evidence output, failure retention, and fail-closed authority termination | `AGREE` |

## Verification

| Check | Result |
| --- | --- |
| Exact commit identity | PASS — review used a clean detached worktree at `b9a2534457c4b7b6aa6006b2a06bcd27a8b83bf2`. |
| Candidate delta | PASS — relative to the `eca9d20` reviewed implementation, executable behavior changes only in `Assert-PresentationExit` and its self-test; the remainder is the two `eca9d20` review artifacts. |
| `git diff --check HEAD^` | PASS. |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/phase0-presentmon.ps1 -SelfTest` | PASS on Windows PowerShell. |
| Direct exit-policy probe against the committed function | PASS — accepted `(0, false)` and `(-1, true)`; rejected `(-1, false)` and `(5, true)`. |
| `cargo +1.82.0 fmt --all -- --check` | PASS. |
| `cargo +1.82.0 test --locked -p nvide-ui --all-targets` | PASS — 8 passed, 2 subprocess fixtures intentionally ignored. |
| `cargo +1.82.0 test --locked -p nvide-render` | PASS — 3 tests passed. |
| `cargo +1.82.0 clippy --locked -p nvide-ui -p nvide-render --all-targets --all-features -- -D warnings` | PASS. |

## Finding closure

- **F1 remains closed:** ACK consumption is gated by `finalizer_presented`; the tests cover an ACK arriving between target and finalizer and the one-second pre-edit stabilization.
- **F2 remains closed:** the exact PresentMon 2.5.1 `--qpc_time --v1_metrics` header and semantics remain enforced: Present-call `QPCTime`, `msUntilDisplayed`, and `Dropped=1` excluded from display evidence.
- **F3 remains closed:** provisional ACKs are revalidated against the complete retained capture, with exactly one displayed match and exact request/acknowledgement counts required for success.
- **F4 remains closed:** absolute paths, a fresh evidence directory, immutable request hashes, strict filename/body/trace identity, and exact unique counts remain unchanged.
- **F5 remains closed:** asynchronous stderr draining, bounded process/session cleanup, raw/stderr/command/hash retention, and the success/failure manifest remain unchanged.
- **F6 closed:** `Assert-PresentationExit` now accepts only exit code `0`, or exactly `-1` after the harness has observed NVide exit and intentionally stopped the authority. Every other nonzero code fails even when that flag is true, and the self-test covers the previously accepted `5` case.

No actionable finding remains in this amendment-review scope.

## Explicit exclusion

This `AGREE` verdict fills only the independent performance-reviewer slot for the P0.2 presentation-finalization amendment at the exact reviewed commit. It does **not** approve the native reference host, exit-bound tool command/hash, calibration artifact, P0-E6 evidence, Phase 0 exit, or the phase-lead reviewer slot; all remain separate and pending.
