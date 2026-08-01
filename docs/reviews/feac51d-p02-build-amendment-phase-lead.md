# P0.2 build-command amendment — phase-lead review

- Reviewer principal: `agent:/root/p02_build_amendment_lead`
- Role: Phase lead
- Verdict: `AGREE`
- UTC date: 2026-08-01
- Reviewed commit: `feac51dc597b54c1289c68fb2f17dcf991622dd8`

## Scope

Reviewed only the P0.2 release-build command amendment from
`cargo build --locked --release -p nvide` to
`cargo build --locked --release -p nvide -p nvide-core` and its claim that the
approved workload, measurement semantics, pass rules, and exit-binding gate are
unchanged. This review does not fill the independent performance-review or exit-binding slots.

## Checks

- Compared the exact amendment at the reviewed commit with the previously approved P0.2 protocol.
- Checked Architecture's thin GUI and separate UI/core process requirements, ADR-0002's required real process boundary, ADR-0011 dependency direction, and ADR-0021's Phase 0 ownership split.
- Confirmed `nvide` no longer contains or depends on the duplicated core/buffer/IPC implementation and that `nvide-ui` resolves a sibling `nvide-core` executable.
- Ran `cargo build --locked --release -p nvide -p nvide-core`; both release executables were produced successfully.
- Ran `cargo tree -p nvide --edges normal --prefix none`; `nvide-core` is not in the `nvide` dependency graph, so naming the second package is necessary.
- Ran short release `clear` and `edit` diagnostic smokes. Both exited successfully; the edit run used the sibling core executable and emitted its runtime trace and frame readback.
- Confirmed both smoke manifests remain `UNBOUND_DIAGNOSTIC`; no P0-E6 or Phase 0 exit claim is introduced.

## Findings

None. Building both existing process packages is the minimal sufficient change for the approved edit workload after restoring the thin-binary boundary. The amendment changes neither the launched benchmark commands nor their workload, formulas, thresholds, evidence semantics, or pending native-host/tool/calibration binding.

## Verdict

`AGREE`
