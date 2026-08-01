# ADR-0021: Phase 0 rendering baseline

- Status: Accepted
- Date: 2026-07-31
- Extends: ADR-0003, ADR-0010, ADR-0011
- Prepared-by principals: `agent:/root`

## Context

ADR-0003 permits several proven shapers and does not assign the Phase 0 event-loop/render resources precisely. Current major releases of some candidates exceed the Architecture's Rust 1.82 MSRV.

## Decision

Phase 0 pins `winit` 0.30.13, `wgpu` 23.0.1, and `cosmic-text` 0.14.2. Winit and cosmic-text declare Rust floors 1.70 and 1.75; wgpu's v23.0.1 upstream README states 1.76. Newer dependency versions are not used when they exceed MSRV 1.82.

- `nvide-ui` owns `winit` event-loop code and converts keyboard input into core edit requests; the `nvide` binary invokes that entry point.
- `nvide-render` owns the `wgpu` instance, surface, adapter, device, queue, pipeline, and glyph atlas.
- `cosmic-text` provides font discovery/fallback, bidi, shaping, layout, and `SwashCache` rasterization. Import its `Buffer` as `ShapingBuffer` to distinguish it from `nvide_buffer::Buffer`.
- One atlas and one WGSL pipeline draw textured glyph quads. The only Phase 0 retained content is the current viewport text.
- The proof path is `winit input → NRPC edit → RopeBuffer version → viewport text → cosmic-text shaping → atlas/quads → wgpu present`.
- A lost/outdated surface is reconfigured and retried for one frame. Out-of-memory or unrecoverable device loss returns a typed fatal/degraded result and never panics.

Phase 0 does not add `glyphon`, a second shaper, a general widget framework, a renderer abstraction, docking, a damage graph, multi-window UI, or an accessibility bridge.

## Alternatives

- Latest `wgpu` and `cosmic-text` releases are rejected while their MSRVs exceed 1.82.
- Direct Swash or HarfBuzz integration is not selected because `cosmic-text` already provides the required shaping, fallback, layout, and rasterization path.
- A general UI framework is outside the Phase 0 shaped-text proof.

## Pros/Cons

Pros: one proven cross-platform shaping stack, explicit ownership, minimal rendering scope, and MSRV compatibility.

Cons: pinning older releases defers upstream fixes and requires a deliberate dependency upgrade after an approved MSRV change.

## Consequences

P0-R7 and P0-A5 use only this dependency set and ownership split. Dependency upgrades require the four-target and MSRV checks.

## Migration

Upgrade within this stack only after verifying Rust 1.82 or approving an ADR-0010 MSRV change. A different shaper or renderer requires a new ADR.

## Scalability

The glyph atlas and shaped-run boundary support later scene/damage work without implementing those later-phase systems now.

## References

- <https://docs.rs/crate/winit/0.30.13>
- <https://github.com/gfx-rs/wgpu/blob/v23.0.1/README.md#msrv-policy>
- <https://docs.rs/crate/cosmic-text/0.14.2>

## Approval record

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | `agent:/root/ai_review_policy` | AGREE | 2026-08-01 | `710644906cd9589c2e3f2c25a8484088e710feac` | [`7106449-p0-plan-policy.md`](../reviews/7106449-p0-plan-policy.md) |
| Independent reviewer | `agent:/root/ai_review_records` | AGREE | 2026-08-01 | `710644906cd9589c2e3f2c25a8484088e710feac` | [`7106449-p0-records.md`](../reviews/7106449-p0-records.md) |
