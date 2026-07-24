# Architecture Decision Records

Source of truth for binding architectural decisions. The HTML architecture
document indexes ADRs; it does not re-litigate them.

## Process

| Field | Rule |
|-------|------|
| ID format | `ADR-NNNN` (zero-padded, monotonic, never reused) |
| Status | `Proposed` → `Accepted` → `Superseded by ADR-XXXX` \| `Deprecated` \| `Rejected` |
| Storage | `docs/adr/ADR-NNNN-slug.md` |
| Required sections | Context · Decision · Alternatives · Pros/Cons · Consequences · Migration · Scalability |
| Who may accept | Tech lead + one reviewer (or RFC vote for cross-cutting ADRs) |

## Catalog (Phase 0)

| ID | Title | Status |
|----|-------|--------|
| [ADR-0002](ADR-0002-nrpc-ipc.md) | NRPC IPC protocol | Accepted |
| [ADR-0003](ADR-0003-winit-wgpu-ui.md) | Rendering stack (winit + wgpu + custom UI) | Accepted |
| [ADR-0005](ADR-0005-rope-buffer.md) | Rope buffer + Buffer trait | Accepted |

Remaining catalog entries (ADR-0001, 0004, 0006–0019) are indexed in
`docs/architecture.html` and will receive standalone markdown bodies as they
are implemented.
