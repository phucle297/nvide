# ADR-0005: Rope buffer and Buffer trait

- Status: Accepted
- Catalog source: Architecture v0.2.0
- Acceptance revalidation: PENDING
- Prepared-by principals: `agent:/root`
- Date: 2026-07-31

This file records the accepted catalog decision. Its presence is not P0.1 approval evidence until the tech lead and one independent reviewer complete the record below.

## Context

Large files, random edits, line lookup, Unicode boundaries, and later multi-cursor batches are central editor workloads. Core code needs a stable text abstraction without exposing the concrete storage representation.

## Decision

Define a `Buffer` trait and use a rope with a line-start index as the default implementation in `nvide-buffer`.

The Phase 0 trait covers `insert`, `delete`, `slice`, `line`, `line_count`, and `char_to_byte`. Buffer versions increase monotonically. Undo is a branching tree; each node stores inverse operations, a timestamp, and a cursor snapshot.

The Architecture names both the trait and an example concrete struct `Buffer`; the concrete type name, coordinate/range semantics, and invalid UTF-8-boundary behavior remain P0.1 decisions.

Optional mmap-backed chunks are limited to read-only huge-file work and are not required by Phase 0.

The complete Phase 0 API and undo semantics are proposed separately in ADR-0022. They are not part of this Accepted decision unless ADR-0022 is approved.

## Alternatives

- A piece table was not selected because random access and line lookup require additional indexing and may degrade for the target workload.
- A gap buffer was rejected because it performs poorly for large jumps and multi-cursor edits.
- Exposing a concrete rope directly was rejected because core code must depend on the `Buffer` boundary.

## Pros/Cons

Pros:

- Logarithmic split/merge behavior supports large files and non-local edits.
- Chunked storage supports efficient slices and reads.
- The trait keeps core independent from the concrete rope.

Cons:

- UTF-8, char/byte, and line-index conversions add correctness risk.
- A rope and branching undo tree are more complex than a flat string and linear stack.
- Index maintenance requires property and fuzz coverage over edit sequences.

## Consequences

`nvide-buffer` is a Friend-tier crate and must not depend on UI, render, or plugin-host code. Phase 0 tests insert/delete/replace, line endings, invalid UTF-8 boundaries, conversions, undo/redo branches, and generated edit roundtrips.

## Migration

Core code uses the `Buffer` trait. A piece-table backend for small scratch buffers or a different rope may be introduced only when profiling justifies it and without changing callers unnecessarily. CRDT hooks are research-only until an Accepted ADR authorizes them.

## Scalability

Rope operations and the line-start index are intended to remain responsive for large files. The abstraction permits a later storage replacement while keeping Phase 0 callers stable.

## Approval record

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | PENDING | PENDING | PENDING | PENDING | PENDING |
| Independent reviewer | PENDING | PENDING | PENDING | PENDING | PENDING |
