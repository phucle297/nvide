# NVide implementation plan

Status: REVIEWED — two independent approvals
Last updated: 2026-08-01
Architecture baseline: `docs/architecture.html` v0.2.1
Current implementation phase: Phase 0
Current product target: MVP
Prepared-by principals: `agent:/root`

This Markdown file is the canonical implementation plan for AI contributors. `docs/plan/index.html` is the equivalent human-readable view and may shorten prose, but it must preserve every normative gate, interface, metric, exclusion, and ADR mapping in this file.

Engineering phases and product milestones are different. Phases are execution slices. MVP, 1.0, 2.0, Enterprise, and Cloud are product outcomes governed by ADR-0019. A phase may start only after its dependencies and every listed prerequisite have been approved. Phase 4 may overlap Phase 3 only after Phase 2 exits.

## Plan governance

- Architecture v0.2.1 and Accepted ADR-0001…ADR-0019 remain authoritative.
- A requirement absent from the Architecture is a prerequisite decision or artifact, not an implementation detail to guess.
- The phase lead owns each prerequisite unless a narrower owner is named. An ADR requires the Architecture's acceptance rule: tech lead plus one reviewer, or an RFC vote when cross-cutting. A non-ADR artifact requires the phase lead plus one independent reviewer. Security artifacts also require a security reviewer.
- Human and AI agents may fill reviewer roles. Record principals as `human:<handle>` or `agent:<canonical-task-id>`. Every reviewer principal must differ from every author or implementer principal and every other required reviewer principal; for AI, another turn by the same agent is not independent, and one agent cannot fill multiple required reviewer slots.
- Every phase keeps an evidence ledger at the path named below. Each ledger entry records the stable requirement/acceptance ID, exact artifact path, reproducible verification command, environment/profile, result, date, and reviewer principal. Review records also identify every author or implementer principal and the exact reviewed commit; the inspectable review artifact itself identifies the reviewer principal, verdict, reviewed commit, and review scope. A statement without a runnable command or inspectable artifact is not exit evidence.
- IDs are immutable after review: prerequisites use `Pn.n`; requirements use `Pn-Rn`; runnable acceptance uses `Pn-An`; exit evidence uses `Pn-En`; release blockers use `Pn-Bn`. Milestones 6A–6D use the milestone prefix. CI must reject duplicate/unknown IDs and any required ID without a ledger reference.
- The P0.2 amendment is reviewed at commit `60c80a2c778027d504c965ab125de5937a852977`. Two independent reviewers cover both formats, Phase 0–5, and the Phase 6+ umbrella.

## Overall phase plan

| Phase | Product milestone served | Dependency | Scope | Exit gate | Governing ADRs |
| --- | --- | --- | --- | --- | --- |
| 0 — Foundations & Architecture Hardening | MVP | None | Canonical decisions; benchmark profile; exact Phase 0 workspace; buffer/undo; NRPC; supervisor; shaped-text edit roundtrip | Four-target CI; reproducible schemas; buffer/IPC/failure evidence; approved 120 FPS profile; visible UI↔core edit roundtrip | ADR-0001–0003, ADR-0005, ADR-0010–ADR-0013, ADR-0015, ADR-0017–ADR-0019 |
| 1 — Editable Vertical Slice | MVP | Phase 0 | Local-file editor; approved modal subset; multi-cursor/search; explorer/tabs/splits; Rust/Lua syntax; Lua config subset; isolated terminal; restore/autosave | Runnable editing/safety/isolation suite; ≥50% dogfood for two weeks; cold start <500 ms under the approved protocol | ADR-0001–ADR-0003, ADR-0005, ADR-0006, ADR-0010, ADR-0012–ADR-0014, ADR-0016–ADR-0019; ADR-0015 only if the prerequisite ADR declares a stable Lua/settings surface |
| 2 — IDE Features & LSP | MVP | Phase 1 | `nvide-lang`; seven-feature LSP MVP; problems/navigation/completion/palette; hybrid Git; project search; remaining grammars; WAL recovery | Pinned Rust/TypeScript fixtures fully navigable; no acknowledged edit lost in crash restore; fault suites green | ADR-0001, ADR-0002, ADR-0008, ADR-0010–ADR-0013, ADR-0015–ADR-0019 |
| 3 — Plugins & Compat L1 | MVP | Phase 2 | Minimal API Stability Tier S3 (Rust plugin SDK) v1; isolated Lua host; manifest/lockfile/lifecycle; permissions; frozen Lua compatibility suite | Every frozen plugin smoke test green; measured Tier A ≥35%; plugin crash isolated and quarantined | ADR-0001, ADR-0002, ADR-0004, ADR-0006, ADR-0007, ADR-0011, ADR-0013, ADR-0015–ADR-0019 |
| 4 — Remote Development | MVP | Phase 2; may overlap Phase 3 | SSH agent for the locked FS/PTY/LSP interface; version-vector cache; reconnect; ordered offline writes | Security/data-safety matrix green; weekly real use; approved latency gate; mean reconnect <5 s | ADR-0001, ADR-0002, ADR-0009, ADR-0011–ADR-0019 |
| 5 — MVP Hardening & Release | MVP | Successful exit of Phases 0–4 | Required packages; signed update/release path; measurement gates; memory audit; docs; external acceptance | All release blockers, including Phase 0–4 revalidation, green; signed artifacts install/upgrade; public MVP | ADR-0010, ADR-0013, ADR-0015–ADR-0019 |
| 6A — 1.0 Stability | 1.0 | Phase 5 / public MVP | S1 Lua/API Stability Tier S3 stability, migration policy, budgets/docs, marketplace v0 | 1.0 evidence/threat/release gates | ADR-0004, ADR-0007, ADR-0010, ADR-0015, ADR-0016, ADR-0018, ADR-0019; marketplace ADR required |
| 6B — 2.0 Platform | 2.0 | 6A / 1.0 | Remote parity, DAP, Compat L2, experimental WASM, multi-window/a11y, feasibility-confirmed post-MVP budgets | 2.0 evidence/threat/release gates | ADR-0004, ADR-0007, ADR-0009, ADR-0010, ADR-0015–ADR-0019; DAP and WASM ADRs required |
| 6C — Enterprise | Enterprise | 6B / 2.0 | Private registry, SSO, airgap, policy, audit, support | Enterprise evidence/threat/release gates | ADR-0016, ADR-0018, ADR-0019; enterprise security/policy ADR and threat model required |
| 6D — Cloud | Cloud | 6C / Enterprise | Hosted workspaces, settings sync, optional thin client, retained local UI | Cloud evidence/threat/release gates | ADR-0001, ADR-0002, ADR-0009, ADR-0015–ADR-0019; Cloud tenancy/security ADR required |

## Sequencing and scope rules

1. Protect Phase 0 buffer, rendering, IPC, supervisor, and cross-platform baselines before product features.
2. Do not scaffold later-phase crates or pull plugins, LSP, Git, terminal, remote, marketplace, DAP, or WASM into an earlier phase.
3. Phase 3 and Phase 4 may overlap only after Phase 2; Phase 5 requires successful exit of, then integrates and revalidates, Phases 0–4.
4. Apply ADR-0018's relevant unit, integration, snapshot/golden, fuzz, compatibility, performance, and cross-platform gates in every phase.
5. Keep 6A–6D as separately approved milestones. A later item appearing here is not authorization to implement it early.
6. CRDT collaboration, NRPC v2, renderer replacement, and LuaJIT remain research-only until demonstrated demand and an Accepted ADR authorize implementation.

## MVP scope boundary

MVP includes the Architecture's daily-driver set: native UI; approved modal editing; rope, multi-cursor, undo, search, splits/tabs; selected Tree-sitter languages; Lua configuration; Lua plugin host and Compat L1; the seven LSP features; local terminal; hybrid Git; recovery; basic SSH remote; and required installers.

MVP excludes WASM plugins, full DAP, WSL/Docker/Dev Containers, full Neovim compatibility, Vimscript, collaboration, automatic merge-conflict resolution, mobile/web, full marketplace PKI, delta updates, and pixel-perfect VS Code theme import.

## Detailed phase plans

### Phase 0 — Foundations & Architecture Hardening

**Context:** MVP · 6–8 weeks · no predecessor · current phase
**ADRs:** ADR-0001–ADR-0003, ADR-0005, ADR-0010–ADR-0013, ADR-0015, ADR-0017–ADR-0019
**Evidence ledger:** `docs/evidence/phase-0.md` (created by Phase 0 implementation, not by this planning change)

#### Entry gates

The gates are ordered. Workspace implementation is blocked until P0.1 and the P0.2 protocol are approved. The P0.2 reference-host/tool/calibration binding remains mandatory before P0-E6 and Phase 0 exit.

**Gate status (2026-08-01):** P0.1 `APPROVED`; P0.2 protocol `REVIEW PENDING` and exit binding `PENDING`; P0.3 `APPROVED`. The P0.2 protocol therefore continues to block workspace implementation.

| ID | Required decision or artifact | Owner | Approval | Blocks |
| --- | --- | --- | --- | --- |
| P0.1 | Canonical ADR-0002/0003/0005 records plus coding standards and ADR workflow, consistent with Architecture v0.2.1 | Tech lead | ADR acceptance rule; coding standard by tech lead + independent reviewer | All Phase 0 implementation |
| P0.2 | Benchmark/trace protocol defining required hardware/OS/toolchain fields, release build command, warmup, samples, tool semantics, aggregation, and pass rule. “120 FPS clear” means actually presented steady-state frames for an empty window with no workspace/plugins at the fixed resolution/vsync after warmup. The UI↔core fixture uses a unique scripted edit and correlates UI dispatch → core version increment → viewport receipt → first presented glyph. The eligible native host, exact tool command, and calibration may be bound after implementation begins but not after Phase 0 exit. | Performance owner | Protocol: phase lead + independent performance reviewer before workspace; binding: the same roles before P0-E6/exit | Workspace until protocol approval; P0-E6/exit until binding approval |
| P0.3 | Approved `xtask` command surface and canonical generated-schema ownership/check policy | Build owner | Phase lead + independent reviewer | Schema implementation |

#### Requirement checklist

- [ ] **P0-R1:** Create exactly these Phase 0 crates: `nvide`, `nvide-core`, `nvide-buffer`, `nvide-platform`, `nvide-ipc`, `nvide-rpc-schema`, `nvide-render`, and `nvide-ui`; do not scaffold later-phase crates.
- [ ] **P0-R2:** Enforce ADR-0011 dependency direction and stable Rust/MSRV checks on Windows x64, macOS x64, macOS arm64, and Linux x64.
- [ ] **P0-R3:** Provide reproducible `xtask` schema generation from a clean checkout.
- [ ] **P0-R4:** Implement rope edits, line index/conversions, UTF-8 boundaries, branching undo, and edit-sequence property/fuzz coverage.
- [ ] **P0-R5:** Implement NRPC framing, multiplexing, local Unix-socket/Windows-named-pipe transport, generated schemas, and ADR-0015 major/minor handshake behavior.
- [ ] **P0-R6:** Add the minimum UI/core supervision path needed to prove ADR-0017 heartbeat, failure detection, restart budget, restart/rebind, and user-visible degradation.
- [ ] **P0-R7:** Render shaped text in `winit` + `wgpu`, then prove input → NRPC → core rope edit → viewport snapshot → visible glyph update under P0.2.

#### Ordered work packages and runnable acceptance

1. **P0-A1 — Canonical foundation:** approve P0.1 and the P0.2 protocol, then create only the exact workspace and four-target CI. Acceptance: clean checkout build/test commands and dependency-edge check are recorded and green on all four targets.
2. **P0-A2 — Schema pipeline:** approve P0.3, generate NRPC schemas, and make regeneration deterministic. Acceptance: the approved clean-checkout schema command exits zero and `git diff --exit-code` reports no unexplained change.
3. **P0-A3 — Buffer:** implement rope, line index, conversions, edits, and undo. Acceptance: buffer unit/property tests cover insert/delete/replace, line endings, invalid UTF-8 boundaries, undo/redo branches, and generated edit roundtrips.
4. **P0-A4 — NRPC and supervisor:** implement local transports, codec/handshake, UI/core child lifecycle, and restart/rebind. Acceptance: subprocess tests cover malformed/oversized/truncated frames, compatible minor versions, incompatible major versions, connect/drop/broken transport, heartbeat loss, restart budget exhaustion, and successful supervised restart.
5. **P0-A5 — Rendered vertical path:** shape visible text and connect the real process boundary. Acceptance: under P0.2, M0.1 records the defined 120 FPS clear workload; M0.2 records rope-sourced visible edits; M0.3 records the correlated UI↔core trace through the first presented glyph.

#### Exit evidence

| ID | Gate | Required artifact and verification |
| --- | --- | --- |
| P0-E1 | Cross-platform foundation | Ledger maps P0-R1/P0-R2/P0-A1 to four CI job results and clean-checkout build/test/dependency commands; no forbidden edge or cycle |
| P0-E2 | Reproducible schemas | Ledger maps P0-R3/P0-A2 to the approved schema command, generated paths, tool versions, and clean `git diff --exit-code` result |
| P0-E3 | Buffer correctness | Ledger maps P0-R4/P0-A3 to unit/property/fuzz artifacts for edits, line index, UTF-8 boundaries, branching undo, and roundtrips |
| P0-E4 | IPC compatibility/failure | Ledger maps P0-R5/P0-A4 to codec/handshake/framing fuzz for malformed frames, version mismatch, cancellation, transport failure, and real subprocess roundtrip |
| P0-E5 | Supervisor lifecycle | Ledger maps P0-R6/P0-A4 to forced core failure, restart/rebind, restart-budget exhaustion, and degraded-state evidence under ADR-0017 |
| P0-E6 | M0.1–M0.3 | Ledger maps P0-R7/P0-A5 to the exit-bound P0.2 native host/tool/calibration, exact commands, raw frame samples, shaped-text capture, correlation trace, and first-presented-glyph artifact |

**Out of scope:** full shell, modal editor, terminal, LSP, Git, plugins, remote, DAP, marketplace, persistent production daemon, and all later-phase crate scaffolding. The render prototype may be replaced once; the buffer API remains stable.

### Phase 1 — Editable Vertical Slice (Pre-Alpha)

**Context:** MVP · 3–4 months · requires every Phase 0 exit gate
**ADRs:** ADR-0001–ADR-0003, ADR-0005, ADR-0006, ADR-0010, ADR-0012–ADR-0014, ADR-0016–ADR-0019; ADR-0015 only if P1.4 declares a stable Lua/settings surface
**Evidence ledger:** `docs/evidence/phase-1.md`

#### Entry gates

| ID | Required decision or artifact | Owner | Approval | Blocks |
| --- | --- | --- | --- | --- |
| P1.1 | Modal action/key matrix for normal/insert/visual modes, counts/register behavior where included, and explicit unsupported motions/actions | Editor owner | Phase lead + independent reviewer | Modal/keymap implementation and compatibility claims |
| P1.2 | Autocmd contract listing events, ordering, nesting/reentrancy, cancellation, and per-handler/error behavior | Config owner | Phase lead + independent reviewer | Autocmd implementation |
| P1.3 | Dogfood/startup protocol: reference laptop, workload, cold-cache definition, build command, warmup, samples, aggregation, coding-time capture, and pass rule | Performance owner | Phase lead + independent reviewer | Phase 1 exit metrics |
| P1.4 | ADR resolving Phase 1 in-process/config-only Lua versus the Phase 3 isolated plugin host, including load-order behavior before plugins exist and whether any Lua/settings surface becomes stable | Tech lead | ADR acceptance rule | Lua config implementation; ADR-0015 mapping if stability is declared |

#### Requirement checklist

- [ ] **P1-R1:** Open, edit, save, close, and reopen local files with correct dirty state, external-change detection, atomic-save behavior, and visible save failures.
- [ ] **P1-R2:** Implement only P1.1 modal actions, selections, transactional multi-cursor edits, and buffer search/replace; one multi-cursor batch creates one undo node.
- [ ] **P1-R3:** Provide explorer, editor groups, tabs/splits, and viewport integration sufficient to edit this repository.
- [ ] **P1-R4:** Add incremental Rust/Lua Tree-sitter highlighting and discard stale parse/highlight results by buffer version.
- [ ] **P1-R5:** Load the P1.4 Lua 5.4 configuration subset with typed options, keymaps, and only P1.2 autocmds.
- [ ] **P1-R6:** Run local PTY/ConPTY plus a proven VT emulator in an isolated terminal host; terminal failure must not kill core/UI.
- [ ] **P1-R7:** Restore open buffers, cursors, and layout; autosave safely; recover or reject corrupt/stale sessions; never invent a path for Untitled buffers.

#### Ordered work packages and runnable acceptance

1. **P1-A1 — Local editing:** file open/save plus P1.1 modes, selection, multi-cursor, and buffer search. Acceptance: tests cover dirty transitions, save permission/I/O failure without losing dirty state, external mutation, and exactly one undo node for one multi-cursor batch.
2. **P1-A2 — Editor shell:** explorer, groups, tabs/splits, viewport. Acceptance: a runnable walkthrough opens this repository, navigates files, edits in splits, saves, closes, and restores layout.
3. **P1-A3 — Syntax and Lua config:** Rust/Lua parse/highlight plus approved configuration subset. Acceptance: golden fixtures pass; a deliberately delayed old parse result is discarded; invalid Lua/autocmd errors follow P1.2 without crashing the editor.
4. **P1-A4 — Terminal host:** portable PTY/ConPTY and proven VT engine. Acceptance: cross-platform smoke and VT golden tests pass; forced PTY/terminal-host crash leaves core/UI usable and reports terminal loss.
5. **P1-A5 — Session safety and metrics:** restore/autosave plus P1.3 measurement. Acceptance: corrupt session is quarantined/recovered, stale file metadata prompts instead of overwriting, Untitled remains pathless, dogfood is ≥50% for two consecutive weeks, and cold start is <500 ms.

#### Exit evidence

**P1-E1:** The Phase 1 ledger must map P1-R1…P1-R7 and P1-A1…P1-A5 to runnable commands/artifacts, the approved P1.1 unsupported list, dirty-state/save-failure results, one-batch→one-undo proof, stale-syntax discard, PTY crash isolation, corrupt/stale session recovery, and raw P1.3 dogfood/startup samples.

**Out of scope:** LSP/problems, Git, project search, plugin host/Compat, SSH remote, DAP, marketplace, full Vim parity, and the remaining MVP grammars. Unsupported motions remain explicitly labeled unsupported in Markdown, HTML, tests, and product messaging.

### Phase 2 — IDE Features & LSP (Alpha)

**Context:** MVP · 3–4 months · requires Phase 1 dogfood/startup gates
**ADRs:** ADR-0001, ADR-0002, ADR-0008, ADR-0010–ADR-0013, ADR-0015–ADR-0019
**Evidence ledger:** `docs/evidence/phase-2.md`

#### Entry gates

| ID | Required decision or artifact | Owner | Approval | Blocks |
| --- | --- | --- | --- | --- |
| P2.1 | Executable Rust/TypeScript navigation fixtures, expected results, and pinned rust-analyzer/TypeScript server versions | Language owner | Phase lead + independent reviewer | “Fully navigable” claim |
| P2.2 | Complete MVP command-palette command/provider inventory | UI owner | Phase lead + independent reviewer | Palette completeness claim |
| P2.3 | WAL acknowledgement/durability contract and crash matrix for UI, core, and hosts at before/after acknowledgement points | Core owner | ADR acceptance if architecture changes; otherwise phase lead + independent reviewer | Recovery implementation and no-data-loss claim |

ADR-0016 fixes the trust rule: gitoxide status/diff read-only operations may run in an untrusted workspace; every Git CLI write and hook-capable operation is blocked until the workspace is trusted. This is not left to implementation choice.

#### Requirement checklist

- [ ] **P2-R1:** Run `nvide-lang` with ADR-0017 lifecycle, cancellation, restart budgets, and full versioned-document resynchronization after restart.
- [ ] **P2-R2:** Implement exactly seven LSP MVP features: completion, hover, goto/definition, references, rename, diagnostics, and format.
- [ ] **P2-R3:** Route current-version results to problems, navigation, gutter/extmarks, completion, and every provider in P2.2; discard stale or malformed results.
- [ ] **P2-R4:** Implement hybrid Git: gitoxide status/diff read-only; hunk/line stage/unstage and CLI commit/push/pull only under the fixed trust rule, preserving user config, hooks, and credential helpers.
- [ ] **P2-R5:** Add streaming project search that respects ignore rules and handles unreadable paths and binary files without aborting the whole search.
- [ ] **P2-R6:** Add TS/JS, Python, Go, JSON, TOML, and Markdown grammars, completing the MVP set with Rust/Lua.
- [ ] **P2-R7:** Implement P2.3 WAL/recovery with mtime/hash conflict handling and a user-visible recovery report.

#### Ordered work packages and runnable acceptance

1. **P2-A1:** Versioned buffer/extmark flow and the P2.3 fault harness.
2. **P2-A2:** `nvide-lang`, seven-feature LSP flow, UI consumers, ADR-0017 restart/resync, and ADR-0015 NRPC compatibility.
3. **P2-A3:** Trust-aware hybrid Git and streaming project search.
4. **P2-A4:** Six remaining grammars and the frozen P2.2 palette inventory.
5. **P2-A5:** Pinned P2.1 navigation and forced-crash acceptance suites.

#### Exit evidence

| ID | Gate | Required runnable evidence |
| --- | --- | --- |
| P2-E1 | LSP lifecycle | Maps P2-R1…P2-R3/P2-A1/P2-A2 to forced restart, budget exhaustion, full resync, cancellation, version mismatch, stale/malformed rejection, and multi-server conflicts |
| P2-E2 | Fully navigable | Maps P2-R2/P2-R3/P2-A5 to every P2.1 Rust/TypeScript fixture and pinned server; HTML uses “fully navigable” |
| P2-E3 | Git/trust | Maps P2-R4/P2-A3 to untrusted reads, blocked CLI writes/hooks, and trusted hunk/line stage/unstage plus commit/push/pull and errors |
| P2-E4 | Search/syntax | Maps P2-R5/P2-R6/P2-A3/P2-A4 to eight grammar goldens and search ignores, permissions, symlinks, binaries, cancellation, and streaming |
| P2-E5 | Recovery | Maps P2-R7/P2-A1/P2-A5 to every before/after-ACK process kill, corrupt WAL, external conflicts, recovery report, and no acknowledged-edit loss |

**Out of scope:** plugin host/Compat, remote agent, full DAP, task/test-runner expansion, deep Git history/rebase UI, and non-MVP LSP features such as signature help, code actions, semantic tokens, inlay hints, workspace symbols, call hierarchy, and code lens unless separately approved.

### Phase 3 — Plugins & Compat L1 (Beta)

**Context:** MVP · 3–5 months · requires Phase 2
**ADRs:** ADR-0001, ADR-0002, ADR-0004, ADR-0006, ADR-0007, ADR-0011, ADR-0013, ADR-0015–ADR-0019
**Evidence ledger:** `docs/evidence/phase-3.md`

#### Entry gates

| ID | Required decision or artifact | Owner | Approval | Blocks |
| --- | --- | --- | --- | --- |
| P3.1 | Frozen 5–10-plugin suite naming every plugin, fixed commit/version, smoke actions, expected results, and Tier-A denominator/calculation | Compat owner | Phase lead + two independent reviewers | Compat implementation and percentage claim |
| P3.2 | Minimal API Stability Tier S3 (Rust plugin SDK) v1 contract and deprecation/version plan, limited to the Architecture's MVP plugin API promise | Plugin owner | ADR acceptance if it changes public architecture; API review by tech lead + independent reviewer | Rust SDK implementation |
| P3.3 | Fine-grained UI/LSP capability contract: UI contribution is separate from editor-state reads; LSP is mediated by a bounded API; source/buffer/workspace content requires an explicit read grant; every capability is scoped to workspace trust | Plugin/security owners | Phase lead + independent security reviewer | Permission and API implementation |

Phase 3 ships the minimal API Stability Tier S3 (Rust plugin SDK) v1 required by Architecture. The compatibility suite remains Lua-focused; Rust SDK scope does not expand the frozen suite.

#### Locked plugin interface

- **Manifest:** identity, version, source, dependencies, permissions, and activation.
- **Lockfile:** resolved commit/version/hash.
- **Lifecycle:** discover → resolve → fetch/verify → load → activate → runtime → deactivate → quarantine.
- **Permission defaults:** `fs.read` workspace-only; `fs.write` and clipboard prompt; network and shell deny. Architecture's UI/LSP “allow” defaults are narrow: UI permits declarative contributions, not editor-state reads; LSP is available only through the P3.3 bounded API, not raw transport/server access. Neither permission permits arbitrary buffer/workspace/source reads without an explicit content-read grant. Activation and every grant remain bound to workspace trust. Grants are explicit and revocable; “legacy unrestricted” is opt-in only with a warning.

#### Requirement checklist

- [ ] **P3-R1:** Run Lua plugins out of process through NRPC callbacks with ADR-0017 restart budgets and quarantine.
- [ ] **P3-R2:** Ship only the P3.2 minimal versioned API Stability Tier S3 Rust plugin SDK v1.
- [ ] **P3-R3:** Validate manifests and enforce P3.3 permission grant/revoke plus ADR-0016 workspace trust.
- [ ] **P3-R4:** Resolve Git sources and dependencies deterministically; verify hashes; pin lockfile commit/version/hash; clean up on deactivate.
- [ ] **P3-R5:** Expose the minimum versioned native Lua API and keep Compat L1 inside `nvide-compat-vim`: required buffer/window/extmark/autocmd/command APIs, `vim.keymap`, `vim.opt`, `vim.notify`, `vim.schedule`, and suite-demanded basics of `vim.fn`.
- [ ] **P3-R6:** Reload Lua with cleanup hooks and detect leaked or duplicate registrations.
- [ ] **P3-R7:** Generate a compatibility and unsupported-call report for P3.1.

#### Ordered work packages and runnable acceptance

1. **P3-A1:** Freeze P3.1–P3.3; lock manifest, lockfile, lifecycle, capability, and denominator contracts.
2. **P3-A2:** Build Lua host lifecycle, NRPC callbacks, supervision, P3.3 permissions/trust, and the minimal Tier S3 SDK.
3. **P3-A3:** Add Git fetch/verify, lockfile, dependency ordering, activation/deactivation, and quarantine.
4. **P3-A4:** Add the minimum native Lua surface and isolated Compat L1 calls demanded by P3.1.
5. **P3-A5:** Add cleanup-aware reload and run compatibility, permission, reproducibility, and crash suites.

#### Exit evidence

**P3-E1:** The ledger maps P3-R5/P3-R7/P3-A4/P3-A5 to every frozen plugin/smoke action and the measured Tier-A ratio. The ratio must be ≥35%; results above 45% still pass because 45% is not a ceiling.

**P3-E2:** The ledger maps P3-R1…P3-R4/P3-R6 and P3-A1…P3-A5 to manifest rejection, P3.3 least-privilege checks, grant/revoke/defaults, untrusted auto-run denial, dependency cycles, hash mismatch, identical reinstall, reload leaks/duplicates, crash/restart-budget/quarantine, and the unsupported-call report.

**Out of scope:** registry/update channels, marketplace signing PKI, WASM, Compat L2+, Vimscript, raw libuv FFI, undocumented C internals, 100% Neovim parity, and unrestricted-by-default execution.

### Phase 4 — Remote Development (Beta 2)

**Context:** MVP · 3–4 months · requires Phase 2; may overlap Phase 3
**ADRs:** ADR-0001, ADR-0002, ADR-0009, ADR-0011–ADR-0019
**Evidence ledger:** `docs/evidence/phase-4.md`

#### Entry gates

| ID | Required decision or artifact | Owner | Approval | Blocks |
| --- | --- | --- | --- | --- |
| P4.1 | Client/agent OS-version compatibility and release matrix | Remote owner | Phase lead + independent reviewer | Agent build/release support claims |
| P4.2 | ADR for SSH bootstrap/auth, agent install/update, session-token lifetime/rotation/revocation, and agent binary integrity | Security/remote owners | ADR acceptance rule + security reviewer | Bootstrap and authentication |
| P4.3 | Latency/network benchmark profile: client/host hardware, OS, bandwidth/RTT/loss, workload, warmup, samples, aggregation, measurement tool, pass rule | Performance owner | Phase lead + independent performance reviewer | “Acceptable latency” and reconnect claims |
| P4.4 | ADR for write acknowledgement, durability, per-path ordering, version-vector conflict semantics, and idempotent replay | Core/remote owners | ADR acceptance rule | Remote write/offline queue implementation |

#### Locked Phase 4 interface

- Handshake: `HELLO` / `HELLO_ACK`.
- Services: `agent.health`, `fs.*`, `pty.*`, and `lsp.*` only.
- `dap.*`, `task.*`, and remote Git are not Phase 4 work.
- The local cache uses version vectors. Offline writes are ordered per path, replay is idempotent, and the client never silently overwrites, automatically merges, or hides a conflict.

#### Requirement checklist

- [ ] **P4-R1:** Bootstrap a separate verified agent over SSH and negotiate NRPC version/capabilities/session through the locked handshake.
- [ ] **P4-R2:** Open/read/write/watch remote files through the version-vector cache and P4.4 acknowledgement contract.
- [ ] **P4-R3:** Run PTYs and language servers on the agent while rendering/controlling them locally.
- [ ] **P4-R4:** Resume watches, reattach PTYs when supported or explain loss, fully resync LSP documents, and meet mean reconnect <5 s under P4.3.
- [ ] **P4-R5:** Continue local editing offline; queue ordered idempotent writes; show degraded state; surface concurrent mutations without silent overwrite/merge.
- [ ] **P4-R6:** Enforce workspace trust and capability boundaries; protect secrets in the OS keychain; prevent the remote from invoking local shell or local FS outside granted capability.

#### Ordered work packages and runnable acceptance

1. **P4-A1:** Approve P4.1–P4.4; implement verified SSH bootstrap, handshake, capabilities, token, and trust boundary.
2. **P4-A2:** Add remote FS/watch, version-vector cache, acknowledgement, and versioned writes.
3. **P4-A3:** Add agent-owned PTY and LSP flows.
4. **P4-A4:** Add reconnect, watch resume, PTY reattach-or-loss, and LSP full resync.
5. **P4-A5:** Add offline queue/conflict/replay and dogfood under P4.3.

#### Exit evidence

**P4-E1:** The ledger maps P4-R1…P4-R5/P4-A1…P4-A5 to disconnects before/after ACK, duplicate/reordered replay, concurrent same-path mutation, watch resume, PTY reattach/loss, LSP full resync, mean reconnect <5 s, weekly use, and the P4.3 latency result.

**P4-E2:** The ledger maps P4-R1/P4-R6/P4-A1 to incompatible/malicious agents, malformed frames, path traversal, secret storage, token expiry/revocation, agent integrity failure, and local shell/FS escape attempts.

**Out of scope:** WSL, Docker, Dev Containers, remote-core/co-location, `dap.*`, `task.*`, remote Git, collaboration, automatic conflict resolution/merge, and full remote parity.

### Phase 5 — MVP Hardening & Release

**Context:** MVP · 2–3 months · requires successful exit of Phases 0–4 and release-time revalidation of their evidence
**ADRs:** ADR-0010, ADR-0013, ADR-0015–ADR-0019
**Evidence ledger:** `docs/evidence/phase-5.md`

#### Entry gate

| ID | Required decision or artifact | Owner | Approval | Blocks |
| --- | --- | --- | --- | --- |
| P5.1 | Measurement protocol covering reference hardware/OS, release builds, cold/warm definitions, warmup, samples, aggregation, regression thresholds, waivers, external-user pass rule, and immutable fixture manifests/hashes. The 1 GiB manifest fixes encoding, newline form, line-length distribution/max, content mix, and syntax/highlight mode. The large-repo manifest fixes files, commits, tracked/untracked/modified counts and cache state. The crash contract fixes the observation window and the session formula below. | Release/performance owners | Tech lead + independent performance reviewer; privacy/security review for crash/external data | Every release metric and public MVP |

#### Required packaging matrix

| Platform | Required artifacts |
| --- | --- |
| Windows | Portable `.exe`, `.msi`, and winget manifest |
| macOS | `.dmg`, Homebrew cask, and notarized app bundle; universal binary preferred, not required |
| Linux | AppImage, `.deb`, `.rpm`, and PKGBUILD; Flatpak optional and non-blocking |

#### Release blocker contracts

| ID | Blocker | Exact workload/end point |
| --- | --- | --- |
| P5-B1 | Cold start <400 ms | P5.1 release binary from process launch until the first file-backed buffer is editable and its first viewport is presented; OS and NVide caches start in the declared cold state |
| P5-B2 | Warm start <150 ms | Same end point as P5-B1 with the declared warm OS/session cache state; not a pre-opened hidden window |
| P5-B3 | Idle RSS <120 MB | Sum of resident memory for all NVide-owned processes after the P5.1 workspace reaches quiescence; excluded shared pages/tools are declared |
| P5-B4 | 60 FPS | Actually presented frames under the P5.1 representative editing/scroll workload at fixed resolution/vsync after warmup; this is not the empty-window P0.2 workload |
| P5-B5 | 1 GiB open and scrollable <2 s | Immutable P5.1 UTF-8 fixture; timer ends only when the first viewport is presented and a scripted vertical scroll produces a new presented viewport. Full-file read/highlight is not required, but malformed text or a trivial single-line/easy fixture is not allowed. |
| P5-B6 | Cached large-repo status <200 ms | Immutable P5.1 repo fixture; “cached” means one completed status run and no repository mutation. Timer ends when the complete status model, not a partial first row, is applied. |
| P5-B7 | Crash rate <1% | Session-denominated per Architecture: every non-test launch receives a session ID; numerator is any unexpected NVide-owned process termination that prevents or ends the editable session; denominator is all such sessions in the approved window. Exclusions require P5.1 approval. |
| P5-B8 | Phase 0–4 gates green | Re-run or freshness-check every required Phase 0–4 evidence ID on the release candidate; inherited dependency alone is insufficient |
| P5-B9 | Memory audit complete | Inspectable per-process ownership/leak/growth artifact with commands, workload, raw results, findings, and disposition |

#### Requirement checklist

- [ ] **P5-R1:** Map every MVP inclusion and every required Phase 0–4 ID to passing or freshness-validated evidence; close only documented gaps.
- [ ] **P5-R2:** Exercise supervisor degradation/recovery and collect only opt-in P5-B7 crash metrics under P5.1.
- [ ] **P5-R3:** Enforce P5-B1…P5-B9 under the approved P5.1 fixtures and formulas.
- [ ] **P5-R4:** Produce the entire required packaging matrix and test install, upgrade, launch, uninstall/rollback behavior defined by P5.1 on the supported release OS matrix.
- [ ] **P5-R5:** Sign/notarize applicable artifacts and signed update metadata; accept valid signatures and reject tampered or wrong-key artifacts.
- [ ] **P5-R6:** Complete user docs, tutorial, Neovim migration guide, changelog, and external source-free install/Lua-configuration acceptance.

#### Ordered work packages and runnable acceptance

1. **P5-A1:** Approve P5.1; build the ID-to-evidence matrix and close only verified gaps.
2. **P5-A2:** Freeze release candidates; run Phase 0–4 revalidation plus reliability, recovery, compatibility, remote, performance, memory, and cross-platform gates.
3. **P5-A3:** Build and test all required packages, signatures/notarization, release metadata, and update verification.
4. **P5-A4:** Finish docs and external acceptance, then publish MVP only when P5-B1…P5-B9 and every requirement ID are green.

#### Exit evidence

**P5-E1:** The ledger maps P5-R1…P5-R6, P5-A1…P5-A4, and P5-B1…P5-B9 to raw/aggregated P5.1 samples, fixtures/hashes, waivers, memory audit, Phase 0–4 revalidation, release OS matrix, install/upgrade/launch, signature/notarization, signed metadata, tamper/wrong-key rejection, and external-user results. “Works on a developer machine” is not release evidence.

**Out of scope/non-blocking:** delta updates, marketplace PKI, DAP, WASM, WSL/containers, Compat L2, Enterprise/Cloud, collaboration, and Flatpak. Universal macOS is preferred but non-blocking.

### Phase 6+ — Separately approved post-MVP milestones

Phase 6+ is not one backlog. Each milestone below has its own entry gate, ordered deliverables, evidence ledger, exclusions, threat model, and release matrix. No milestone starts before its predecessor exits.

#### 6A / 1.0 — Stability and marketplace v0

**Entry gate:** public MVP/Phase 5 complete; marketplace architecture and PKI ADR accepted; 1.0 dependency graph, threat model, release matrix, and evidence ledger plan approved. Owner: 1.0 lead plus marketplace/security owners. Approval: ADR acceptance rule and security reviewer.
**Evidence ledger:** `docs/evidence/phase-6a-1.0.md`

**Ordered deliverables:**

1. **6A-R1:** Stabilize and document S1 `nvide.*` and the minimal API Stability Tier S3 Rust plugin SDK v1.
2. **6A-R2:** Enforce semver, deprecation windows, migration tooling/tests, compatibility budgets, and upgrade documentation.
3. **6A-R3:** Sustain signed updates, crash/performance/memory budgets, user/plugin-author docs, and supported OS packages.
4. **6A-R4:** Ship marketplace v0 only under the accepted marketplace/PKI ADR.

**6A-E1:** The ledger maps 6A-R1…6A-R4 to public API diff/semver tests; deprecated and migrated plugin/config fixtures; signed-update and rollback tests; budgets; docs walkthrough; marketplace publisher/package verification, malicious/tampered package rejection, and recovery.
**Threat model:** plugin supply chain, publisher impersonation, malicious packages, compromised update metadata, token/credential theft, and downgrade attacks.
**Release matrix:** supported Windows/macOS/Linux packages and upgrades; S1 Lua/API Stability Tier S3 compatibility across the supported 1.0 version window; marketplace online/failure/recovery paths.
**Excluded:** DAP, WASM, remote parity, Compat L2, Enterprise policy, Cloud, and research items.

#### 6B / 2.0 — Platform, debugger, Compat L2, and experimental WASM

**Entry gate:** 1.0/6A complete; focused DAP and WASM ADRs accepted; 2.0 dependency graph, threat model, release matrix, and evidence ledger plan approved. **6B.1** additionally requires a feasibility report on representative 2.0 builds/workloads for every post-MVP performance number below. The numbers are candidate targets, not release commitments, until the tech lead and independent performance reviewer accept them; an infeasible target requires an ADR/roadmap amendment before implementation. Owner: 2.0 lead plus remote/debug/plugin/performance owners. Approval: ADR acceptance rule, security reviewer, and independent performance reviewer.
**Evidence ledger:** `docs/evidence/phase-6b-2.0.md`

**Ordered deliverables:**

1. **6B-R1:** Add WSL, Docker, and Dev Containers remote parity.
2. **6B-R2:** Ship the approved DAP experience and adapter lifecycle/isolation.
3. **6B-R3:** Reach measured Compat L2 ≥70% on a pinned popular-plugin matrix.
4. **6B-R4:** Add experimental WASM under the approved sandbox/capability model.
5. **6B-R5:** Deliver multi-window/a11y work and only the 6B.1-confirmed subset of these Architecture post-MVP candidate targets: cold <200 ms, idle RSS <80 MB, 120 FPS, 1 GiB open <500 ms, crash <0.1%, reconnect <2 s.

**6B-E1:** The ledger maps 6B-R1…6B-R5 and 6B.1 to remote parity fixtures, disconnect/rebuild/reopen recovery, DAP launch/attach/breakpoint/adapter-crash fixtures, pinned ≥70% compat report, WASM capability/isolation/fuel/memory tests, accessibility audits, multi-window recovery, the feasibility decision, and raw performance samples.
**Threat model:** malicious dev-container metadata, container escape paths, debug adapter compromise, WASM capability escape/resource exhaustion, and expanded compat permissions.
**Release matrix:** supported local OS × WSL/Docker/Dev Container versions; supported debug adapters; WASM experimental platforms; upgrade/migration from 1.0.
**Excluded:** Enterprise controls, Cloud hosting, CRDT, NRPC v2, renderer replacement, and LuaJIT.

#### 6C / Enterprise — Policy and private distribution

**Entry gate:** 2.0/6B complete; enterprise security/policy ADR and full threat model accepted; support/SLO ownership, dependency graph, release matrix, and evidence ledger plan approved. Owner: Enterprise lead plus security/operations owners. Approval: ADR acceptance rule, security reviewer, and release owner.
**Evidence ledger:** `docs/evidence/phase-6c-enterprise.md`

**Ordered deliverables:**

1. **6C-R1:** Private registry and controlled package distribution.
2. **6C-R2:** SSO hooks, centralized workspace/plugin policy, and administrator controls.
3. **6C-R3:** Airgapped install/update/license operation with no undeclared network dependency.
4. **6C-R4:** Tamper-evident audit logs, retention/export controls, and support channels.

**6C-E1:** The ledger maps 6C-R1…6C-R4 to private-registry auth/isolation, SSO lifecycle, policy allow/deny/enforcement, audit integrity/redaction/export, offline install/update, key rotation/revocation, incident recovery, and support/SLO exercises.
**Threat model:** tenant/org boundary errors, malicious administrators/plugins, SSO/token compromise, policy bypass, audit tampering/PII leakage, signing-key compromise, and offline supply chain.
**Release matrix:** supported enterprise OS/package versions, identity providers, online/airgap topologies, private-registry versions, upgrade/rollback, and support lifetime.
**Excluded:** hosted multi-tenant workspaces, settings sync, optional thin client, and research items.

#### 6D / Cloud — Hosted workspaces and settings sync

**Entry gate:** Enterprise/6C complete; Cloud tenancy/security/data-residency ADR accepted; Cloud threat model, service SLOs, dependency graph, release matrix, disaster-recovery plan, and evidence ledger plan approved. Owner: Cloud lead plus security/operations owners. Approval: ADR acceptance rule, security reviewer, and operations/release owner.
**Evidence ledger:** `docs/evidence/phase-6d-cloud.md`

**Ordered deliverables:**

1. **6D-R1:** Hosted workspace provisioning, lifecycle, quota, suspend/resume, and deletion.
2. **6D-R2:** Conflict-safe team settings sync with offline/retry/recovery behavior.
3. **6D-R3:** Optional thin client while retaining and supporting the local UI.
4. **6D-R4:** Tenant isolation, observability, backup/restore, regional/data-retention controls, and incident operations.

**6D-E1:** The ledger maps 6D-R1…6D-R4 to tenant-isolation/adversarial tests, workspace lifecycle and quota tests, settings-sync conflict/recovery, backup/restore and regional failover, auth/session/key rotation, deletion/retention proof, thin/local client compatibility, load/SLO, and incident drills.
**Threat model:** cross-tenant access, control-plane compromise, remote execution abuse, secret leakage, sync poisoning/conflicts, data residency/retention failure, denial of service, and backup exposure.
**Release matrix:** local and thin clients × supported hosted regions/workspace images; connectivity degradation; schema/client compatibility; regional failover; upgrade/rollback.
**Excluded:** replacing the local UI and any research item without demand and an Accepted ADR.

#### Research-only boundary

CRDT collaboration, NRPC v2, renderer replacement, and LuaJIT remain research-only. Demonstrated demand and a focused Accepted ADR are mandatory before any enters an implementation milestone.

## Review record

The P0.2 amendment invalidated the prior exact-revision verdicts without invalidating their historical artifacts. Two eligible reviewers independently inspected both formats at the amendment commit; this revision is `REVIEWED — two independent approvals`.

### Overall plan gate

| Slot | Reviewer principal | Markdown | HTML | UTC date | Reviewed commit | Artifact | Final verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Reviewer 1 | `agent:/root/ai_review_parity` | AGREE | AGREE | 2026-08-01 | `60c80a2c778027d504c965ab125de5937a852977` | [`60c80a2-plan-parity.md`](../reviews/60c80a2-plan-parity.md) | AGREE |
| Reviewer 2 | `agent:/root/plan_amendment_reviewer2` | AGREE | AGREE | 2026-08-01 | `60c80a2c778027d504c965ab125de5937a852977` | [`60c80a2-plan-reviewer2.md`](../reviews/60c80a2-plan-reviewer2.md) | AGREE |

### Detailed plan gate

`Reviewer 1` and `Reviewer 2` below refer to the principals, reviewed commit, and artifacts in the overall gate. A detailed verdict is invalid if that binding is missing or violates the distinct-agent rule.

| Reviewer / format | Phase 0 | Phase 1 | Phase 2 | Phase 3 | Phase 4 | Phase 5 | Phase 6+ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Reviewer 1 / Markdown | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE |
| Reviewer 1 / HTML | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE |
| Reviewer 2 / Markdown | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE |
| Reviewer 2 / HTML | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE | AGREE |

## Final verification checklist

- [x] Phase 0–5, the Phase 6+ umbrella, and milestones 6A–6D exist in both formats.
- [x] Every dependency, prerequisite, stable ID, deliverable, interface, metric, exclusion, ADR mapping, threat model, and release matrix is equivalent across formats.
- [x] Phase 1 HTML preserves dirty-state/save-failure, one-batch→one-undo, and unsupported-motion terms.
- [x] Phase 2 HTML states both “fully navigable” and “crash restore loses no acknowledged edit.”
- [x] HTML asset paths resolve; IDs and `<summary>` text are unique; tags are balanced; keyboard/sidebar controls remain accessible.
- [x] `node --check docs/assets/document.js`, stable-ID checks, whitespace checks, and diff checks pass.
- [x] No runtime source, dependency, or public API changed; the review-policy amendment is mirrored in Architecture, AGENTS, ADRs, Phase 0 artifacts, and both plan formats.
- [x] Two independent reviewers provide inspectable artifacts for the amendment commit, both formats, Phase 0–5, and the Phase 6+ umbrella; both AI reviewers satisfy the distinct-agent rule.
