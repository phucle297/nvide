# ADR-0003: Rendering stack

- Status: Accepted
- Catalog source: Architecture v0.2.0
- Acceptance record attachment: PENDING
- Date: 2026-07-31

This file records the accepted catalog decision. Its presence is not P0.1 approval evidence until the tech lead and one reviewer attach an inspectable review record.

## Context

NVide needs a native, GPU-accelerated IDE shell with high-DPI text, multi-window support, accessibility, dense editor layouts, and long-term control of frame performance. The core must remain independent from painting.

## Decision

Use `winit` for windows and events, `wgpu` for GPU rendering, and an internal custom UI toolkit owned by `nvide-ui` and `nvide-render`.

- Electron, Tauri, and GPUI are not the product shell.
- The toolkit is retained-mode and internal.
- Text shaping reuses a proven shaper; NVide does not invent shaping. Selection among `cosmic-text`, `swash`, and HarfBuzz bindings remains a P0.1 decision.
- The rendering design uses a scene graph, glyph cache/atlas, damage tracking, high-DPI support, and an accessibility tree.
- Core produces state/snapshots and never paints.

The Phase 0 dependency and ownership baseline is proposed separately in ADR-0021. The P0.2 measurement profile is a separate artifact because it has a performance owner and approval rule.

## Alternatives

- GPUI was rejected because of ecosystem coupling and API churn.
- Tauri was rejected because a WebView conflicts with the native product goal.
- `egui` was rejected for the product shell because of accessibility and dense-IDE limitations.
- Slint and iced were not selected because their trade-offs do not provide the same editor-specific control.

## Pros/Cons

Pros:

- Full control over text rendering, high-DPI behavior, multi-window support, and frame budgets.
- No browser or WebView runtime in the product shell.
- The core remains isolated from renderer implementation details.

Cons:

- NVide must build and maintain layout, focus, accessibility, virtualization, and widget behavior.
- Initial implementation cost is higher than adopting a complete UI framework.
- Platform variation requires cross-platform evidence.

## Consequences

`nvide-ui` depends on `nvide-render`; neither may import plugin-host code. Phase 0 implements only the shaped-text edit path and the approved empty-window presentation benchmark, not the later IDE shell.

## Migration

The toolkit remains internal. A future renderer or UI replacement requires an Accepted ADR and may reimplement widgets against the retained scene-graph boundary without moving painting into core.

## Scalability

The retained scene graph, damage tracking, glyph atlas, and virtualization are the path to many splits, multiple high-DPI monitors, and 120 Hz presentation. The Phase 0 prototype may be replaced once; no public renderer API is frozen.
