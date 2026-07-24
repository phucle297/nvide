# ADR-0005: Rope buffer + Buffer trait

- **Status:** Accepted
- **Date:** 2026-07-23
- **Tags:** buffer, rope, undo, editor-core

## Context

IDE buffers must handle multi-megabyte and multi-gigabyte files, multi-cursor
edits, and frequent line lookups for rendering, search, and LSP. Historical
piece-table and gap-buffer designs degrade under large-file random access or
multi-cursor workloads without heavy secondary indexing.

Core code must not freeze on a single concrete data structure forever.

## Decision

Adopt a **rope-backed buffer** behind a stable **`Buffer` trait**.

Trait surface (minimum):

- `insert`, `delete`, `slice`, `line`, `line_count`, `char_to_byte` (and
  related position mapping as needed)

Default implementation: rope (e.g. **ropey**-class algorithms) with line index
support and optional future chunk mmap for read-only huge files.

**Undo:** inverse-op history that restores prior text after edits (Phase 0 ships
a linear undo/redo stack; branching undo tree layout remains the long-term
model described in the architecture document).

All core editor code depends on the trait, not the concrete rope type.

## Alternatives

1. **Piece table** — strong sequential undo story; needs secondary index for
   large random access and line queries.
2. **Gap buffer** — excellent local edits; poor multi-cursor / large jumps.
3. **Plain `String` / `Vec<u8>`** — unacceptable for large-file goals.

## Pros/Cons

**Pros**

- O(log n) split/merge behavior fits large-file and multi-cursor workloads.
- Trait boundary enables alternate backends (e.g. piece table for tiny scratch).
- Mature rope crates reduce implementation risk for Phase 0.

**Cons**

- More complex than a gap buffer for tiny files.
- UTF-8 / char vs byte indexing must be handled carefully at API boundaries.

## Consequences

- Crate `nvide-buffer` is dependency-light (no UI/IPC).
- Property tests exercise edit invariants on the shipped API.
- Buffer content version counters support LSP and dirty tracking.

## Migration

- Phase 0: `RopeBuffer` + tracked undo/redo + property tests.
- Later: piece-table backend for scratch buffers if profiling warrants; CRDT-
  friendly ops only behind a new ADR.

## Scalability

- Rope + line index supports multi-GB files without full copies on edit.
- Slice/line accessors avoid cloning entire ropes for render and search.
- Extmarks/interval trees layer on top without changing the text trait.
