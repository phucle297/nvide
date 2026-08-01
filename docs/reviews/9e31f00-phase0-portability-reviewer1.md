# Phase 0 portability re-review — reviewer slot 1

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Independent Phase 0 implementation reviewer, slot 1 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `9e31f0017665e63d0502c647047688255a4ab5fe`
- Reviewed parent: `7bc5fed4d60d28e015d56bd342762026aea58521`
- UTC date: 2026-08-01
- Overall verdict: **CHANGES REQUIRED — PHASE 0 PORTABILITY**
- Scope: CI policy metadata deletion, macOS-safe short unique lifecycle endpoints, hosted-scheduler deadline-test margins, and the Windows named-pipe `ReadFile` repair.

This review used a clean detached worktree at the exact reviewed commit. The reviewer did not author or implement the reviewed change, fills no other reviewer slot, and does not approve hosted/native evidence beyond the exact results listed below, the P0.2 exit binding, formal P0-E6 evidence, or Phase 0 exit.

## Blocking finding

### P0-R2/P0-A1 remains red on both macOS targets

The exact-commit hosted [GitHub Actions run #30685372716](https://github.com/phucle297/nvide/actions/runs/30685372716) proves that the candidate is not yet portable across the four required targets. Both native macOS jobs pass MSRV check and stable build, then fail the required workspace test command at `.github/workflows/ci.yml:37` with exit code 101:

| Job | Check | Build | Test | Test interval (UTC) |
| --- | --- | --- | --- | --- |
| `aarch64-apple-darwin` | PASS | PASS | **FAIL** | 05:12:54–05:12:54 |
| `x86_64-apple-darwin` | PASS | PASS | **FAIL** | 05:15:07–05:15:08 |

The same run passes Linux x64, Windows x64, policy, and fuzz. Therefore the metadata-policy deletion and Windows repair are no longer blockers, but Architecture's required Windows/macOS/Linux matrix and roadmap `P0-R2`, `P0-A1`, and `P0-E1` are not satisfied. A source-only cross-check cannot replace native macOS execution.

Public GitHub check annotations expose only exit code 101, not the failing test's stdout. The sub-second duration proves the remaining blocker is earlier than the five-second `nvide-ui` hung-core test. The strongest deterministic candidates, in execution order observed on the Linux host, are:

1. `nvide-core` lifecycle subprocess/Unix-socket tests;
2. `nvide-ipc` aggregate deadline tests;
3. `nvide-render::cosmic_text_shapes_bidi_and_fallback_text`, which depends on the native system-font inventory and fallback selection.

These are candidates for isolation, not a claimed root cause. The reviewed commit shortened only the `nvide-core` lifecycle endpoints; increasing timing caps again without first identifying the failing test would not be an evidence-based repair.

Minimal diagnostic workflow on each macOS runner:

```text
RUST_BACKTRACE=1 cargo +stable test --workspace --all-targets --all-features \
  --locked --target <matrix-target> --no-fail-fast -- \
  --nocapture --test-threads=1
```

If the complete log remains inaccessible, split only the three candidate packages into named steps, in this order, with the same target and harness arguments:

```text
cargo +stable test -p nvide-core --all-targets --all-features --locked --target <matrix-target> -- --nocapture --test-threads=1
cargo +stable test -p nvide-ipc --all-targets --all-features --locked --target <matrix-target> -- --nocapture --test-threads=1
cargo +stable test -p nvide-render --all-targets --all-features --locked --target <matrix-target> -- --nocapture --test-threads=1
```

The next candidate needs the named failing test, a portable fix or portable invariant, and green native reruns on both macOS architectures.

## Delta assessment

- **CI policy metadata deletion:** correct and minimal. The redundant full `cargo metadata` call is removed, while the dependency-policy step still consumes `cargo metadata --locked --no-deps`; the exact hosted policy job passes.
- **Lifecycle endpoint shortening:** preserves per-process/per-test uniqueness using process ID plus an atomic ID and keeps the Unix socket name short. It introduces no production API or later-phase scope. It did not, however, close the native macOS test failure.
- **Deadline-test margins:** the aggregate test now has a clear semantic margin: a 100 ms shared deadline, a 50 ms one-byte reader, and a 500 ms outer cap. The focused test passed in 20 consecutive local runs, but the macOS failure must be isolated before this change can be credited with closing portability.
- **Windows named-pipe read:** the direct synchronous `ReadFile` boundary handles empty buffers, maps `ERROR_NO_DATA` to `WouldBlock`, maps `ERROR_BROKEN_PIPE` to EOF, retains typed errors, and has a local `SAFETY` explanation. It cross-compiles on MSRV, and the exact hosted Windows workspace test job—including the Windows-only nonblocking read regression—passes.
- **Scope:** the complete delta is four existing files, adds no crate or dependency, changes no public runtime API, and contains no Phase 1 feature.

## Checks performed

All local commands targeted the clean detached worktree at the reviewed commit.

```text
git rev-parse HEAD
# 9e31f0017665e63d0502c647047688255a4ab5fe

git diff --check 7bc5fed4d60d28e015d56bd342762026aea58521..HEAD
# PASS

cargo +1.82.0 fmt --all -- --check
# PASS

cargo +1.82.0 check --workspace --all-targets --all-features --locked
# PASS

cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
# PASS

cargo +stable test --workspace --all-targets --all-features --locked -- --nocapture
# PASS on Linux host: 34 passed, 2 fixture tests ignored

cargo +1.82.0 check --workspace --all-targets --all-features --locked --target x86_64-pc-windows-msvc
cargo +1.82.0 check --workspace --all-targets --all-features --locked --target x86_64-apple-darwin
cargo +1.82.0 check --workspace --all-targets --all-features --locked --target aarch64-apple-darwin
# PASS

cargo +1.82.0 xtask evidence check --phase 0
# PASS — mapping validation only; it does not override red native jobs

# Twenty consecutive repetitions of each focused command:
cargo +1.82.0 test -p nvide-ipc --locked frame_deadlines_cover_the_aggregate_operation
cargo +1.82.0 test -p nvide-core --test lifecycle --locked
cargo +1.82.0 test -p nvide-render --locked cosmic_text_shapes_bidi_and_fallback_text
# PASS on Linux host
```

## Evidence and exit status

The prior `e7c9620` implementation agreement remains historical evidence for the implementation reviewed there, and the narrow `17cf2e3` fuzz-toolchain agreement remains valid. This candidate does not earn four-target portability approval while both required macOS test jobs are red.

| Evidence | Status after this review | Reason |
| --- | --- | --- |
| `P0-E1` | **CHANGES REQUIRED** | Both required native macOS workspace test jobs fail at the exact candidate commit. |
| `P0-E2`…`P0-E5` | PENDING formal evidence | This portability review does not convert implementation coverage into exit evidence. |
| `P0-E6` | **BLOCKED** | No approved exit-bound P0.2 native host/tool/calibration or complete immutable benchmark bundle was reviewed. |
| Phase 0 exit | **BLOCKED** | `P0-E1` is red and the independent P0.2/P0-E6 block remains. |

Overall verdict: **CHANGES REQUIRED — PHASE 0 PORTABILITY**. This artifact is not a formal Phase 0 exit approval.
