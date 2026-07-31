# ADR-0022: Phase 0 buffer semantics

- Status: Proposed
- Date: 2026-07-31
- Extends: ADR-0005, ADR-0012, ADR-0013

## Context

ADR-0005 names both a trait and example struct `Buffer` but does not fix the concrete name, coordinate/range/error semantics, atomic edit boundary, or redo-branch selection required for Phase 0 tests.

## Decision

Name the trait `Buffer` and its only Phase 0 implementation `RopeBuffer`, backed by `ropey` 1.6.1 with `default-features = false` and `features = ["simd"]`.

Coordinates are zero-based Unicode scalar-value (`char`) offsets and ranges are half-open. `len_chars` is a valid insertion/range-end position. Lines are zero-based; LF and CRLF are line endings, line slices include their terminator, and `line_count` includes the final empty line after a trailing line break.

`char_to_byte` accepts `0..=len_chars`. `byte_to_char` is added for P0-R4 and rejects byte offsets inside a multi-byte scalar. Typed errors cover char/byte/line bounds, non-boundary bytes, invalid ranges, invalid cursor snapshots, and version exhaustion. Failure is atomic: no text/history/version changes and no panic.

The sole mutating primitive is an atomic `apply_batch`; the trait's `insert` and `delete` methods are one-operation wrappers over it. A batch contains sequential insert/delete operations plus before/after single-selection cursor snapshots. Each operation's coordinates address the intermediate text produced by preceding operations. The entire batch is validated before commit. A successful non-empty batch returns its new version and after-snapshot, increments the monotonic `u64` version once, and creates exactly one undo node. Empty batches are no-ops.

Each undo node stores forward and inverse operations, a monotonic timestamp, and both cursor snapshots. Phase 0 performs no time-based coalescing. Undo moves to the parent; editing there adds a child without deleting other branches. Redo requires an explicit valid child when multiple branches exist. Undo/redo increment the buffer version and never restore an old version number.

Rope storage, line metadata, and undo tree remain private. Ropey's internal line metadata is the Phase 0 line index; no duplicate line-start structure is maintained. Phase 0 adds no mmap/piece-table backend, persistence, CRDT hook, generic backend framework, or multi-cursor behavior.

## Alternatives

- Calling the concrete type `Buffer` is rejected because it conflicts with the required trait name.
- Byte coordinates are rejected for core edits because they permit mid-scalar positions.
- Per-operation undo nodes are rejected because one accepted batch must be one undo unit.
- A second line index is rejected until profiling or missing semantics justify duplicated state.

## Pros/Cons

Pros: one coordinate system, atomic failure, explicit batching, stable branching undo behavior, and no duplicate line index.

Cons: sequential batch coordinates require callers to account for earlier operations, and strict byte conversion differs from Ropey's permissive conversion.

## Consequences

P0-A3 property/fuzz tests generate sequential batches, verify atomic rejection, line terminators, strict UTF-8 boundaries, monotonic versions, and undo/redo branches.

## Migration

Phase 1 may add multi-selection snapshots and batch constructors without changing Phase 0 coordinate semantics. A new storage backend remains behind `Buffer` and requires measured justification.

## Scalability

Ropey's indexed chunks provide logarithmic edit/query behavior without a second index. Atomic batches keep undo growth proportional to user transactions rather than primitive operations.

## References

- <https://docs.rs/ropey/1.6.1/ropey/>

## Approval record

| Role | Reviewer | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | PENDING | PENDING | PENDING | PENDING | PENDING |
| Independent reviewer | PENDING | PENDING | PENDING | PENDING | PENDING |
