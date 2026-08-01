# Phase 0 final evidence re-review — reviewer slot 1

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Independent Phase 0 implementation/evidence reviewer, slot 1 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `a3bde4d27479b6b9f94b22cd7810213f5a880225`
- UTC date: 2026-08-01
- Overall verdict: **AGREE — P0-E1…P0-E5 FORMAL EVIDENCE; P0-E6 IMPLEMENTATION ONLY**
- Scope: closure of `docs/reviews/9e31f00-phase0-portability-reviewer1.md`, the diagnostic/fix sequence through the final direct workflow, regression against the prior Phase 0 implementation approval, and separate evidence verdicts for P0-E1…P0-E6.

This review used a clean detached worktree at the exact reviewed commit. The reviewer did not author or implement the reviewed change, fills no other reviewer slot, and does not approve the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Portability finding closure

The `9e31f00` macOS blocker is closed with an inspectable diagnosis and two green validations:

1. [Run 30685476814](https://github.com/phucle297/nvide/actions/runs/30685476814) at `d97986244dabcbe78e174631b03dd69bcce0deff` exposed the hidden stdout. Both macOS architectures failed only `nvide_ipc::tests::edit_deadline_is_shared_across_write_and_read`; lifecycle endpoints, all preceding tests, and the separate aggregate frame-deadline test passed.
2. [Run 30685608117](https://github.com/phucle297/nvide/actions/runs/30685608117) at `80a56011727721aac418ee68f6379ddfea57e72a` still failed that test. Four short sleeps remained cumulative inside the write phase, so scheduler delay could exhaust the deadline before the read phase and produce the wrong typed result.
3. `f6dd768e2c12f5c64002bb19feea2b78031c46fb` isolates the phases: only the first test-double write sleeps for 400 ms, the shared deadline is 800 ms, and the outer bound is 1,100 ms. A correct implementation spends roughly 400 ms writing and the remaining 400 ms waiting for the read; incorrectly granting the read a fresh 800 ms would exceed the outer bound. [Diagnostic run 30685712163](https://github.com/phucle297/nvide/actions/runs/30685712163) completed successfully across policy, fuzz, and all four native jobs.
4. The reviewed commit removes the temporary Python diagnostic wrapper and restores the direct Cargo test command byte-for-byte. [Final run 30685874494](https://github.com/phucle297/nvide/actions/runs/30685874494) completed successfully. Each Windows x64, macOS x64, macOS arm64, and Linux x64 job has successful MSRV check, stable build, stable workspace test, and `clippy -D warnings` steps.

The final delta from `9e31f00` is four changed lines in one `nvide-ipc` test and its test double. It changes no production timeout, public API, dependency, crate, ADR, or roadmap term. The temporary workflow diagnostics have no net delta, and no Phase 1 feature or scaffolding is present.

## Evidence verdicts

| Evidence | Formal evidence verdict | Reason |
| --- | --- | --- |
| `P0-E1` | **AGREE** | Exact-commit run 30685874494 has green direct check/build/test/clippy jobs on all four required targets. The policy job's exact eight-crate/DAG allowlist check is green. |
| `P0-E2` | **AGREE** | On a fresh hosted checkout, the policy job passes pinned schema generation/check followed by `git diff --exit-code`; the local exact-commit pinned Cap'n Proto 1.5.0 check is also byte-clean. |
| `P0-E3` | **AGREE** | All four native workspace test jobs pass the buffer unit/generated-roundtrip coverage, and the exact-commit fuzz job completes 1,000 buffer runs successfully. |
| `P0-E4` | **AGREE** | All native jobs pass NRPC, Unix-socket/Windows-named-pipe, handshake, malformed/version/cancel/deadline, and subprocess tests; Windows native execution closes the prior cross-check gap. The exact-commit NRPC fuzz target completes 1,000 runs. |
| `P0-E5` | **AGREE** | All four native workspace test jobs pass the real supervisor/lifecycle suite, including live hung-child heartbeat loss, degraded state, restart/rebind, child exit, and restart-budget exhaustion. |
| `P0-E6` | **AGREE — IMPLEMENTATION ONLY** | The shaped-text, display-ack/readback, UI↔core edit, and trace implementation remains covered by the prior independent implementation approvals and the green native suite. Formal evidence remains **BLOCKED** because no approved exit-bound P0.2 host/tool/calibration and immutable native benchmark bundle was reviewed. |

These verdicts review the immutable run and exact source. At `a3bde4d`, `docs/evidence/phase-0.md` still describes hosted/formal evidence as pending; the author must record the exact run and independent review artifacts in a later documentation-only commit. This reviewer intentionally did not edit the ledger.

## Checks performed

```text
git rev-parse HEAD
# a3bde4d27479b6b9f94b22cd7810213f5a880225

git diff --check 9e31f0017665e63d0502c647047688255a4ab5fe..HEAD
# PASS

git diff --exit-code d97986244dabcbe78e174631b03dd69bcce0deff^..HEAD -- .github/workflows/ci.yml
# PASS — temporary diagnostics fully removed; direct workflow restored

cargo +1.82.0 fmt --all -- --check
cargo +1.82.0 check --workspace --all-targets --all-features --locked
cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
# PASS

cargo +stable test --workspace --all-targets --all-features --locked
# PASS on Linux host: 34 passed, 2 fixture tests intentionally ignored

# Twenty consecutive repetitions:
cargo +1.82.0 test -p nvide-ipc --locked edit_deadline_is_shared_across_write_and_read
# PASS; each focused body completed in 0.80 s

cargo +1.82.0 xtask schema check
git diff --exit-code
cargo +1.82.0 xtask evidence check --phase 0
# PASS — Cap'n Proto 1.5.0 cached; schema clean; Phase 0 mapping complete
```

## Remaining block

`P0-E1` through `P0-E5` receive **AGREE** formal evidence verdicts, and `P0-E6` receives **AGREE — IMPLEMENTATION ONLY**. The P0.2 exit binding and formal P0-E6 evidence remain **BLOCKED**. Therefore Phase 0 exit remains **BLOCKED**, independently of the now-green portability/CI evidence. This artifact is not a Phase 0 exit approval.
