# ADR-0002: NRPC IPC protocol

- Status: Accepted
- Catalog source: Architecture v0.2.0
- Acceptance revalidation: PENDING
- Prepared-by principals: `agent:/root`
- Date: 2026-07-31

This file records the accepted catalog decision. Its presence is not P0.1 approval evidence until the tech lead and one independent reviewer complete the record below.

## Context

NVide separates the UI, core, hosts, and remote agent into processes. Their communication needs framing, concurrent logical streams, cancellation, server push, binary blobs, and an independently versioned compatibility handshake.

## Decision

Use NVide RPC (NRPC), a framed, multiplexed, bidirectional protocol.

- Local transport is Unix domain sockets on Linux/macOS and named pipes on Windows. Remote transport uses vsock or an SSH channel.
- A frame contains a little-endian `u32` length field, `u32` stream ID, `u16` flags, then payload. What the length counts and the byte order of the other numeric fields remain P0.1 decisions.
- Flags are `REQ`, `RESP`, `PUSH`, `ERR`, `CANCEL`, `COMPRESSED`, and `PRIORITY`.
- Cap'n Proto is used for hot paths. MessagePack is allowed for Lua plugin bridge convenience.
- Streams carry request/response, server push, and binary blobs.
- The handshake carries an NRPC `major.minor` version. Minor changes are backward compatible; incompatible majors require dual-stack support or an upgrade.
- Edit ordering is strict per buffer. UI paint and diagnostic pushes may coalesce to latest-wins; edit events are never dropped.

The complete Phase 0 wire profile is proposed separately in ADR-0020. It is not part of this Accepted decision unless ADR-0020 is approved.

## Alternatives

- A single-process product is rejected by ADR-0001 because it expands the crash blast radius.
- MessagePack for hot paths is not selected; Architecture permits it for Lua plugin bridge convenience while choosing Cap'n Proto for hot paths.
- Unversioned or unframed IPC is not selected because it cannot satisfy the required compatibility and multiplexing model.

## Pros/Cons

Pros:

- One protocol model covers local and remote process boundaries.
- Multiplexing avoids a connection per logical operation.
- Cap'n Proto supports low-copy hot-path reads.
- Independent versioning permits coordinated process upgrades.

Cons:

- The codec, handshake, and cancellation paths require dedicated failure and fuzz testing.
- Cross-platform transports need separate Unix-socket and Windows-named-pipe implementations.
- Major-version transitions require dual-stack support or an explicit upgrade.

## Consequences

`nvide-ipc` and `nvide-rpc-schema` are Friend-tier crates. Malformed, oversized, truncated, incompatible, cancelled, and broken-transport cases are recoverable errors and must not panic the UI. Phase 0 must prove a real UI-to-core edit roundtrip over the process boundary.

## Migration

Schema fields may be added without reusing field numbers. Backward-compatible changes increment the minor version. Breaking wire changes increment the major version and require dual-stack support or a coordinated process upgrade.

## Scalability

Stream IDs permit concurrent requests, pushes, and blobs on one connection. Latest-wins coalescing bounds stale UI work while strict per-buffer ordering preserves edits.

## Approval record

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | PENDING | PENDING | PENDING | PENDING | PENDING |
| Independent reviewer | PENDING | PENDING | PENDING | PENDING | PENDING |
