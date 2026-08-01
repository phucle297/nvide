# Phase 0 exit review — independent reviewer B

- Reviewer principal: `agent:/root/phase0_exit_reviewer_b`
- Role: independent final evidence and Phase 0 exit reviewer, slot B only
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `3f5fd0ba0cfd6ef2e0c8eb39808f5c27b29a28e4`
- Formal workload commit: `6e156b605618389ae50b58d6ad4ec3eee0763bae`
- Overall verdict: **AGREE — P0-E1…P0-E6 AND PHASE 0 EXIT**

The reviewer is a new orchestrator-issued canonical agent, did not author or implement the reviewed change, and fills no other required review slot. The scope covers the accepted Architecture/ADRs, Phase 0 roadmap and prerequisites, the complete evidence ledger, independent recalculation of the formal P0-E6 archive, regression of P0-E1…P0-E5, and the overall Phase 0 exit gate.

## Exact revision and CI

- The reviewed commit is the direct child of the formal workload commit. Its six-file delta is evidence documentation plus `docs/evidence/p0-e6-formal-6e156b6.zip`; it changes no product source, dependency, schema, workflow, or public API.
- There is no product, harness, workflow, dependency, or schema delta between calibration implementation `6c4542eefe14d3d3f9aad28ea3e847c827654023` and workload commit `6e156b605618389ae50b58d6ad4ec3eee0763bae`.
- Exact-commit GitHub Actions [run 30697871027](https://github.com/phucle297/nvide/actions/runs/30697871027) completed successfully. Its policy, fuzz, Windows x64, Linux x64, macOS x64, and macOS arm64 jobs are green.
- Local locked workspace tests passed: 36 tests passed and the two subprocess fixture entry points were intentionally ignored. `cargo xtask evidence check --phase 0`, pinned Cap'n Proto 1.5.0 `cargo xtask schema check`, generated-schema diff, formatting, Clippy with `-D warnings`, and whitespace checks passed.

## P0-E6 independent recalculation

The archive was extracted into a fresh temporary directory only after checking every member for absolute paths, parent traversal, backslashes, and symlinks.

- Formal archive SHA-256: `e731e169e6a4726f407cd45e6cbb62819873d57175b3b2f745b39c68600bcf37`; size: 1,355,732 bytes; 113 safe members; `unzip -t` passes.
- Approved calibration SHA-256: `121d1a0a35a47c3b30c71933c22a3102def9f0719ca0a4d576ee73b6dca6c0ed`; size: 168,035 bytes.
- The checked-out LF harness converts to Windows CRLF SHA-256 `FE44E3728E65C2EE76E6D62DF4A54203F8925845D0D54C870920A0C423BDA490`, exactly matching all six capture manifests. Every manifest also binds workload commit `6e156b6`, NVide SHA-256 `D549E8A3B63E179F4282C208E3DE44E413FBF34DA12A75E73D70D87F5BA257CC`, and PresentMon 2.5.1 SHA-256 `9BEC3083069F58F911E6A512F4806DB51A27BD096103087BC1D05EF54C80A191`.
- Every run reports `PASS`, one bound PID and swapchain, Vulkan on the approved Radeon 860M, `Runtime=DXGI`, `SyncInterval=1`, tearing disabled, a topmost physical 120 Hz panel, and PresentMon exit `0`. All five clear captures reached natural authority completion.

Using `displayed_ns = QPCTime * 100 + floor(msUntilDisplayed * 1,000,000)`, the P0.2 half-open measurement window, `R = 8,330,000 ns`, and nearest-rank p99 gives:

| Clear run | Raw rows | N / expected | FPS | Missed rate | p99 | Start / end edge | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `clear-01` | 4,790 | 3,600 / 3,601 | 120.000185 | 0.000277701 | 9.2401 ms | 4.0077 / 4.3718 ms | AGREE |
| `clear-02` | 4,791 | 3,600 / 3,601 | 119.996810 | 0.000277701 | 9.2360 ms | 4.2572 / 3.2789 ms | AGREE |
| `clear-03` | 4,791 | 3,600 / 3,601 | 119.996719 | 0.000277701 | 9.2475 ms | 0.5578 / 6.9554 ms | AGREE |
| `clear-04` | 4,791 | 3,600 / 3,601 | 119.996131 | 0.000277701 | 9.2333 ms | 0.9966 / 6.3696 ms | AGREE |
| `clear-05` | 4,792 | 3,600 / 3,601 | 119.996552 | 0.000277701 | 9.2323 ms | 6.7206 / 0.7509 ms | AGREE |

All five independently exceed 119 FPS, remain below the 0.005 missed-slot limit, and retain both edges within one refresh period.

The edit capture contains 191 PresentMon rows, 40 unique immutable display requests, 40 unique actually-displayed authority joins within the fixed 2 ms tolerance (maximum observed join delta 0.8448 ms), 10 warmups, exactly 30 measured runtime rows, and exactly 30 retained 1920×64 RGBA readbacks. Trace IDs, versions, requested/actual frame sequences, request filenames, and authority rows are unique and equal where required. Every measured timestamp is ordered from dispatch through compositor display; every sentinel-line readback is non-uniform and changes from the preceding edit. The final acknowledgement exactly matches frame 191. Recalculated latency is:

| Interval | Median | Nearest-rank p95 |
| --- | ---: | ---: |
| UI dispatch → version increment | 0.9537 ms | 1.5483 ms |
| Viewport receipt → present call | 4.1541 ms | 6.3395 ms |
| UI dispatch → compositor display | 20.48935 ms | 27.8352 ms |

The first two remain diagnostic under P0.2 and are below the Architecture's 2 ms and 8 ms targets. The calibrated quarter-period is 2.0825 ms, so the fixed 2 ms join is valid; the required 50 ms finalizer delay exceeds six calibrated periods. No unbound diagnostic run, missing readback, malformed frame, duplicate join, stale version, unordered timestamp, truncated clear edge, or replacement sample appears in the accepted bundle.

## Evidence verdicts

| Evidence | Verdict | Independent basis |
| --- | --- | --- |
| `P0-E1` | **AGREE** | The exact eight crates and allowed dependency edges match P0-R1/P0-R2; exact-commit CI passes MSRV, locked build/test/Clippy on all four required targets and the policy DAG check. |
| `P0-E2` | **AGREE** | Pinned Cap'n Proto 1.5.0 schema generation is byte-clean locally and in the exact-commit policy job; the generated path and approved command match P0.3. |
| `P0-E3` | **AGREE** | Buffer unit/generated-roundtrip coverage passes on all native jobs; the exact-commit pinned fuzz job completes 1,000 buffer runs. Coverage includes atomic batches, line/UTF-8 boundaries, monotonic versions, branching undo, and roundtrips. |
| `P0-E4` | **AGREE** | Codec, handshake, malformed/oversized/truncated input, cancellation, aggregate deadline, generated messages, real subprocess/local transport, Unix socket, and Windows named-pipe evidence passes; the exact-commit NRPC fuzz target completes 1,000 runs. |
| `P0-E5` | **AGREE** | Exact-commit native tests preserve heartbeat loss, degraded UI state, child exit, restart/rebind, and restart-budget exhaustion under ADR-0017. Later P0-E6 evidence-hook changes do not alter that contract. |
| `P0-E6` | **AGREE** | The approved native binding, immutable hashes, five independently passing clear runs, and complete 30/30 shaped-text UI↔core→first-displayed-glyph correlations satisfy P0-R7/P0-A5 and M0.1–M0.3. |

## Overall Phase 0 verdict

All ordered prerequisites are approved; all P0-R1…P0-R7 and P0-A1…P0-A5 are mapped to independently reviewable P0-E1…P0-E6 evidence; the accepted source remains inside Phase 0; and no later-phase crate or feature is present. **AGREE — Phase 0 may exit at reviewed commit `3f5fd0ba0cfd6ef2e0c8eb39808f5c27b29a28e4`.**

This artifact fills reviewer slot B only. It does not substitute for reviewer slot A, edit the canonical ledger, approve Phase 1 prerequisites, or authorize later-phase implementation.
