# Phase 0 final exit review — reviewer A

- Reviewer principal: `agent:/root/phase0_exit_reviewer_a`
- Role: Independent final Phase 0 evidence reviewer, slot A only
- Author/implementer principal: `agent:/root`
- UTC date: `2026-08-01`
- Reviewed commit: `3f5fd0ba0cfd6ef2e0c8eb39808f5c27b29a28e4`
- Formal workload commit: `6e156b605618389ae50b58d6ad4ec3eee0763bae`
- Exact-commit CI: [run 30697871027](https://github.com/phucle297/nvide/actions/runs/30697871027), six of six jobs successful
- Review scope: P0-E1 through P0-E6 and the overall Phase 0 exit gate
- Reviewer verdict: **AGREE**

This principal did not author or implement the reviewed change, has not filled any prior review slot, and fills only this final reviewer-A slot. This verdict is one independent review; it does not substitute for the separately required reviewer-B verdict.

## Method and authority

I reviewed `AGENTS.md`, Architecture v0.2.1, its Accepted ADR-0001…ADR-0019 catalog, Accepted Phase 0 ADR-0002/0003/0005 and ADR-0020…ADR-0023, the canonical Phase 0 roadmap, P0.2, the evidence ledger, the approved native binding, its two independent revalidation artifacts, and the prior P0-E1…P0-E5 review artifacts. I inspected the exact candidate source and its delta from those earlier reviews.

The candidate CI page binds run `30697871027` to the exact reviewed SHA and reports six successful jobs. Locally, the exact tree has only the required eight packages; `cargo xtask schema check`, `cargo xtask evidence check --phase 0`, `cargo test --workspace --all-targets --all-features --locked`, and the candidate diff checks pass. The local suite completed with 36 passing tests and two intentionally ignored subprocess fixtures. Hosted CI supplies the required four native targets, policy checks, and both 1,000-run fuzz targets.

Changes after the prior P0-E1…P0-E5 review are limited to the Windows presentation-harness CI self-test and Phase 0 presentation evidence hooks in `nvide-render`, `nvide-ui`, and `tools/phase0-presentmon.ps1`. They add no crate, dependency, schema, later-phase feature, or public runtime API. The supervisor regressions remain green. Product, schema, dependency, and harness sources are identical from the final approved binding through the formal workload commit; the reviewed candidate then adds only documentation and immutable evidence.

## Binding and artifact integrity

- `p0-e6-calibration-6c4542e.zip` is 168,035 bytes, passes `unzip -t`, and independently hashes to `121d1a0a35a47c3b30c71933c22a3102def9f0719ca0a4d576ee73b6dca6c0ed`. Recalculation reproduces 60.021488 Hz at 60 Hz and 120.048019 Hz at 120 Hz, with eligible period `R = 8,330,000 ns`; the 2 ms join is below `R/4 = 2.0825 ms`.
- The current binding has independent `AGREE` verdicts from `agent:/root/p02_exit_binding_lead` and `agent:/root/p02_exit_binding_performance` at exact commit `d65d2245fd1390ea67b5c771c3e99ac7b569ebc0`.
- `p0-e6-formal-6e156b6.zip` is 1,355,732 bytes, passes `unzip -t`, has no absolute or parent-traversal member, contains exactly five clear roots plus one edit root, and independently hashes to `e731e169e6a4726f407cd45e6cbb62819873d57175b3b2f745b39c68600bcf37`.
- Every capture manifest is `PASS`, names workload commit `6e156b605618389ae50b58d6ad4ec3eee0763bae`, one bound PID/swapchain, PresentMon 2.5.1 exit `0`, topmost 120 Hz presentation, and the same NVide, harness, and PresentMon hashes. Converting the canonical harness blob to its Windows CRLF checkout independently reproduces harness SHA-256 `FE44E3728E65C2EE76E6D62DF4A54203F8925845D0D54C870920A0C423BDA490`; the workload implementation is byte-identical to the approved binding implementation.
- Every raw PresentMon row has the exact v1 header and is bound to `nvide.exe`, the manifest PID and sole swapchain, `Runtime=DXGI`, `SyncInterval=1`, `AllowsTearing=0`, and `Dropped` in `{0,1}`.

## Independent clear recalculation

I calculated displayed nanoseconds as `QPCTime * 100 + floor(msUntilDisplayed * 1,000,000)`, excluded `Dropped=1`, filtered the half-open runtime window, sorted timestamps, and applied P0.2's FPS, missed-slot, edge, and nearest-rank p99 formulas.

| Run | N / expected | Displayed FPS | Missed rate | p99 interval | Start / end edge | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `clear-01` | 3600 / 3601 | 120.000185 | 0.000277701 | 9.2401 ms | 4.0077 / 4.3718 ms | AGREE |
| `clear-02` | 3600 / 3601 | 119.996810 | 0.000277701 | 9.2360 ms | 4.2572 / 3.2789 ms | AGREE |
| `clear-03` | 3600 / 3601 | 119.996719 | 0.000277701 | 9.2475 ms | 0.5578 / 6.9554 ms | AGREE |
| `clear-04` | 3600 / 3601 | 119.996131 | 0.000277701 | 9.2333 ms | 0.9966 / 6.3696 ms | AGREE |
| `clear-05` | 3600 / 3601 | 119.996552 | 0.000277701 | 9.2323 ms | 6.7206 / 0.7509 ms | AGREE |

Each runtime CSV has exactly 3,600 unique, ordered measurement presents. All 3,600 corresponding displayed events fall in the 30-second window, every run independently exceeds 119 FPS, missed rate is below 0.005, both edges are within `R`, and clear capture drains 6,525–6,628 ms to natural authority exit rather than truncating the measurement tail.

## Independent edit recalculation

The artifact has exactly 40 immutable requests with unique trace IDs `1…40`, unique odd frame sequences `113…191`, and unique present timestamps. Each request has exactly one complete-capture PresentMon row within 2 ms; every match is displayed, and the largest absolute join delta is 0.8448 ms. The retained final acknowledgement exactly matches sequence 191. The capture has 191 raw PresentMon rows, 40 acknowledged requests, exit `0`, and a 1,889 ms bounded post-exit drain.

`runtime.csv` has exactly 30 measured traces `11…40`. Each has equal trace/version, one unique ASCII sentinel, shaped sentinel and changed sentinel pixels, equal expected/actual frame sequence, a matching request and displayed event, and ordered dispatch → core receipt → version increment → viewport emit → viewport receive → present → display timestamps. No next edit dispatch precedes the prior display, and every trace is far below the unchanged five-second deadline. All 30 retained `1920x64` RGBA readbacks are 491,520 bytes, non-uniform, mutually unique, and change between consecutive measured edits; the runtime check also compared the first measured readback with the preceding warmup frame.

| Interval | Median | Nearest-rank p95 | P0.2 treatment |
| --- | ---: | ---: | --- |
| UI dispatch → version increment | 0.9537 ms | 1.5483 ms | Diagnostic; below 2 ms |
| Viewport receipt → present | 4.1541 ms | 6.3395 ms | Diagnostic; below 8 ms |
| UI dispatch → compositor display | 20.48935 ms | 27.8352 ms | Correlation evidence |

## Evidence and exit verdicts

| Evidence | Verdict | Independent basis |
| --- | --- | --- |
| `P0-E1` | **AGREE** | Exact eight-crate workspace, allowed DAG, MSRV/stable locked checks, and Windows x64, macOS x64, macOS arm64, and Linux x64 CI are green at the reviewed commit. |
| `P0-E2` | **AGREE** | Pinned Cap'n Proto 1.5.0 schema check is byte-clean locally and in hosted policy CI; generated ownership and command remain unchanged. |
| `P0-E3` | **AGREE** | Buffer atomic batch, strict UTF-8/line semantics, branching undo, monotonic version, generated roundtrip tests, and the pinned buffer fuzz job are green. |
| `P0-E4` | **AGREE** | Codec, handshake/minor compatibility, incompatible major, cancellation, aggregate deadline, malformed/oversized/truncated input, real subprocess/local transports, Windows named pipe, and NRPC fuzz evidence are green. |
| `P0-E5` | **AGREE** | Exact-commit supervisor tests cover heartbeat loss, degraded state, child exit, restart/rebind, restart budget, and budget exhaustion; later presentation-only changes do not alter that contract. |
| `P0-E6` | **AGREE** | Approved eligible native binding plus the immutable raw bundle independently satisfy all five clear runs, 40/40 compositor acknowledgements, 30/30 measured shaped-text/readback traces, and M0.1–M0.3 correlation requirements. |
| Overall Phase 0 exit | **AGREE** | Ordered prerequisites are approved, every P0-R/P0-A item maps to passing reviewed evidence, exact-commit CI is green, and no later-phase scope or unresolved Phase 0 blocker remains. |

No blocking finding remains. This artifact authorizes only reviewer slot A; Phase 0 may be recorded as approved only after a distinct eligible reviewer also returns `AGREE` for the same exact commit and scope.
