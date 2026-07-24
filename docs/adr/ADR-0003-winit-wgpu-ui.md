# ADR-0003: Rendering stack (winit + wgpu + custom UI)

- **Status:** Accepted
- **Date:** 2026-07-23
- **Tags:** ui, gpu, rendering

## Context

NVide targets a native, GPU-accelerated IDE with a high refresh clear/edit path
(Phase 0 milestone M0.1/M0.2). Product requirements reject shipping the shell
inside a web runtime or an all-in-one retained GUI that owns process architecture.

Constraints: cross-platform (Windows, macOS, Linux), stable Rust only, thin UI
process, editor text quality (shaping, atlas, damage regions) under our control.

## Decision

Use **winit + wgpu + a custom UI toolkit** as the product shell.

- **winit** for the event loop, windows, and input.
- **wgpu** for portable GPU access (Vulkan/Metal/DX12/GL fallbacks).
- **Custom widgets** for docks, chrome, and editor surface — not Electron, Tauri,
  or GPUI as the product shell.
- Text path (target): cosmic-text / HarfBuzz-class shaping → glyph runs → atlas →
  compositor. Phase 0 prototype: clear pass + monospaced glyph mapping from the
  rope buffer so typing is observable without a full atlas stack.

## Alternatives

1. **Electron / Chromium embed** — fast UI iteration, high RAM, non-native feel.
2. **Tauri** — lighter than Electron but still web-view centric for chrome.
3. **GPUI (Zed)** — strong editor-oriented, ties us to another product’s toolkit
   and release cadence.
4. **egui / iced alone** — productive, not ideal as sole IDE chrome + complex
   text editor surface at 120 Hz with custom damage.

## Pros/Cons

**Pros**

- Full control of frame budget, damage, and text pipeline.
- Aligns with multi-process design: UI process stays thin and restartable.
- Portable GPU API via wgpu without per-OS graphics backends in app code.

**Cons**

- Large implementation surface (widgets, accessibility, IME).
- Text rendering complexity is a Phase 0 risk (prototype rewrite expected once).

## Consequences

- `crates/nvide` is the thin UI binary; `nvide-render` holds GPU/layout primitives.
- No product dependency on Electron/Tauri/GPUI shells.
- Headless CI may not open a window; structural use of winit+wgpu plus unit tests
  for buffer→glyph mapping remain the verification fallback.

## Migration

- Phase 0: empty clear window + typed buffer glyphs (monospaced path).
- Phase 1+: replace monospaced path with shaped runs; introduce widget crate
  APIs without changing NRPC edit ownership in core.

## Scalability

- Damage regions and atlas caching scale to large files and multi-pane layouts.
- GPU path supports high refresh (mailbox/vsync policies) independent of core
  process load; paint streams are coalesced over NRPC.
