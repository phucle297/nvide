# Phase 0 final implementation re-review — reviewer slot 1

- Reviewer principal: `agent:/root/phase0_impl_reviewer1`
- Role: Independent Phase 0 implementation reviewer, slot 1 only
- Author and implementer principal: `agent:/root`
- Reviewed commit: `10f98da6040da2ad789d1ae2667b56f3493f76b2`
- UTC date: 2026-08-01
- Overall verdict: **CHANGES REQUIRED**
- Scope: `P0-R1`…`P0-R7`, `P0-A1`…`P0-A5`, implementation support for `P0-E1`…`P0-E6`, accepted ADR-0002/0003/0005/0020/0021/0022/0023, exact Phase 0 crate/CI/schema/buffer/NRPC/supervisor/render boundaries, prior implementation findings, and the approved P0.2 amendments.

This review used a detached worktree at the exact commit above. The reviewer did not author or implement the reviewed change, fills no other reviewer role, and does not approve the P0.2 exit binding, P0-E6 formal evidence, or Phase 0 exit.

## Re-evaluation summary

The findings recorded against `745b098` and `4a34c83` are substantially resolved. The exact eight-crate DAG and thin `nvide` executable remain correct; CI carries stable and Rust 1.82 across all four targets; schema and buffer code are unchanged from their prior clean reviews; NRPC now uses one aggregate frame deadline; the UI test exercises degradation plus real restart/rebind/budget exhaustion; and the approved display-acknowledgement amendment passes one absolute trace deadline through edit, readback, and ACK waiting. Bound edit advancement now requires a matching compositor acknowledgement, while `--unbound-diagnostic` remains explicitly ineligible for P0-E6.

Two transport/liveness defects remain.

## Findings

### F1 — HIGH — Windows named-pipe flush bypasses every NRPC deadline

The Windows `LocalStream` still delegates `Write::flush` to `std::fs::File::flush` (`crates/nvide-platform/src/lib.rs:303-305`). `flush_before` calls that operation first and checks the deadline only after it returns (`crates/nvide-ipc/src/lib.rs:287-292`). The UI's `Client` owns the server end returned by `LocalListener::accept`; Microsoft specifies that `FlushFileBuffers` on a named-pipe server end does not return until the client has read all buffered data ([official documentation](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)). Rust's Windows file implementation maps file flush to that API.

Therefore a connected core that stops reading can hang handshake, edit, or heartbeat flush indefinitely even though writes and reads themselves are nonblocking. The later deadline check cannot recover a blocked UI thread. Remove the unnecessary blocking pipe flush from the protocol path or make it deadline-aware, and add a Windows stalled-peer test proving the complete write-plus-flush operation terminates within its original absolute deadline. This blocks `P0-R5`, `P0-A4`, and implementation agreement for `P0-E4`.

### F2 — HIGH — a hung live core skips the three-missed-heartbeat state

`App::about_to_wait` performs heartbeat synchronously (`crates/nvide-ui/src/lib.rs:219-240`). `CoreSupervisor::heartbeat` calls `Client::heartbeat` (`crates/nvide-ui/src/lib.rs:1133-1152`), whose response read uses a fresh five-second NRPC deadline (`crates/nvide-ipc/src/lib.rs:782-791`). If the child remains alive but stops replying, the first ping blocks the UI event loop for approximately five seconds. Only then is `missed_heartbeats` incremented once; because `last_healthy.elapsed()` is already five seconds, the policy jumps directly to `RestartRequired`. It cannot issue one-second pings or enter the Architecture's `3 missed → unhealthy` state at three seconds.

The real supervisor test kills the child, so `try_wait` fails immediately and three manual calls exercise only that fast-exit path (`crates/nvide-ui/src/lib.rs:1511-1558`). Add a hung-but-live subprocess case and ensure heartbeat waits are scheduled/bounded so the one-second ping, three-miss degradation, and five-second restart rules remain observable without freezing the UI. This blocks `P0-R6`, `P0-A4`, and implementation agreement for `P0-E5`.

## Evidence verdicts

| Evidence | Implementation verdict | Reason |
| --- | --- | --- |
| `P0-E1` | **AGREE — LOCAL IMPLEMENTATION** | Exact eight-package/allowed-edge enforcement passes; the thin executable and four-target stable/MSRV CI configuration remain correct. Hosted native job artifacts are still an exit-ledger concern. |
| `P0-E2` | **AGREE — LOCAL IMPLEMENTATION** | The approved command surface, exact Cap'n Proto 1.5.0 pin/version check, committed output, and byte-comparison implementation are unchanged from the prior clean schema review. |
| `P0-E3` | **AGREE — LOCAL IMPLEMENTATION** | Slice/UTF-8/line/version/atomic sequential edits, branching undo/redo, generated coverage, and the expanded fuzz target are unchanged; current workspace tests pass. |
| `P0-E4` | **CHANGES REQUIRED** | Aggregate frame deadlines and prior Windows read/write fixes are present, but blocking named-pipe flush can still exceed the absolute handshake/request/write deadline indefinitely. |
| `P0-E5` | **CHANGES REQUIRED** | Exit/restart/rebind/budget and degraded-string checks pass, but the actual hung-child path cannot produce one-second misses and three-miss degradation before restart. |
| `P0-E6` | **AGREE — IMPLEMENTATION ONLY; FORMAL EVIDENCE BLOCKED** | The approved display-ACK ingress, shared trace deadline, shaped sentinel/frame correlation, changing readback, typed render failure path, and bound/unbound separation are implemented. P0-E6 and Phase 0 exit still lack the eligible native 120 Hz host, compositor tool/version/command, calibration, immutable artifacts, and two exit-binding approvals. |

## Checks performed

All runnable commands targeted the detached reviewed commit.

```text
git rev-parse HEAD
# 10f98da6040da2ad789d1ae2667b56f3493f76b2

cargo fmt --all -- --check
cargo +1.82.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.82.0 test --workspace --all-targets --all-features --locked
cargo xtask evidence check --phase 0
# PASS; 33 tests passed, with one subprocess fixture intentionally ignored as a direct test

# Exact package/allowed-edge script embedded in .github/workflows/ci.yml
# PASS
```

The four-target CI/MSRV configuration, schema generator, buffer/fuzz sources, generated schema, and their prior exact-commit review evidence were re-inspected; none changed after `4a34c83`. No native 120 Hz run or display claim was performed or inferred.

Overall implementation verdict remains **CHANGES REQUIRED**. Formal `P0-E6` evidence and Phase 0 exit remain **BLOCKED** independently of these findings.
