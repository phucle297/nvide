# ADR-0020: Phase 0 NRPC wire profile

- Status: Proposed
- Date: 2026-07-31
- Extends: ADR-0002, ADR-0015, ADR-0017

## Context

ADR-0002 selects framed, multiplexed NRPC but does not define enough wire detail to implement interoperable codecs, bounded allocation, cancellation, or failure tests without guessing.

## Decision

Phase 0 uses NRPC `1.0` and the following profile.

Each frame is a 10-byte header followed by payload:

```text
payload_len: u32 LE
stream_id:   u32 LE
flags:       u16 LE
payload:     payload_len bytes
```

Payload is capped at 16 MiB and validated before allocation. Larger logical values are schema-level chunks. Each connection permits at most 1,024 open requests and bounds each inbound/outbound queue at both 1,024 frames and 32 MiB.

Flag values are:

```text
REQ        = 0x0001
RESP       = 0x0002
PUSH       = 0x0004
ERR        = 0x0008
CANCEL     = 0x0010
COMPRESSED = 0x0020
PRIORITY   = 0x0040
```

Exactly one of `REQ`, `RESP`, `PUSH`, or `CANCEL` is present. `ERR` is valid only with `RESP`; `PRIORITY` never changes ordering within a stream. Phase 0 rejects `COMPRESSED`. Unknown bits and invalid combinations are protocol errors.

Stream `0` is control-only. The connector allocates increasing odd IDs and the listener increasing even IDs; IDs are never reused within a connection. `REQ` opens a stream, optional `PUSH` frames may follow, and exactly one `RESP` or `RESP|ERR` closes it. A standalone `PUSH` uses a fresh ID and closes immediately.

Payload-free `CANCEL` is idempotent. Repeated cancellation of a valid previously allocated ID and a terminal response racing a local cancellation are ignored. Future IDs, wrong-parity IDs, reused request IDs, and duplicate terminal responses not associated with local cancellation are protocol errors.

The connector's first frame is `REQ` on stream `0` with `HELLO(supported_versions, role, max_payload, compression)`. The listener replies on stream `0` with `HELLO_ACK(selected_version, role, max_payload, compression)`. Phase 0 advertises only `1.0`, roles `UI` and `CORE`, and compression `NONE`. The negotiated payload limit is the lower nonzero limit capped at 16 MiB. No application frame is accepted before acknowledgement. No common major returns `RESP|ERR(INCOMPATIBLE_MAJOR)` and closes.

The handshake, incomplete-frame read, and stalled-write timeouts are five seconds and close the connection with typed errors. A full outbound queue waits up to five seconds and then fails only that send. A request deadline sends `CANCEL` and fails only that request; its default is five seconds. ADR-0017 heartbeat rules govern idle liveness.

Oversized frames, malformed Cap'n Proto, illegal flags or lifecycle, pre-handshake application data, and truncated input close the connection without panic. Unknown well-framed methods and invalid request arguments return `RESP|ERR` without closing. Paint coalescing happens before enqueueing; edit frames are never dropped.

## Alternatives

- Leaving constants and lifecycle implementation-defined would make compatibility and failure evidence non-reproducible.
- One stream direction without parity is simpler but cannot safely allocate bidirectional request IDs.
- Compression in Phase 0 adds decompression limits and negotiation without serving the required edit path.

## Pros/Cons

Pros: deterministic interoperability, bounded memory, explicit cancellation, and testable malformed-input behavior.

Cons: arbitrary safety limits may need later tuning, and stream state must track allocation high-water marks and local cancellation races.

## Consequences

The Phase 0 codec and handshake tests use these exact values. Any implementation before this ADR is Accepted is prohibited by P0.1.

## Migration

Compatible fields and capabilities extend NRPC minor versions without reusing schema field numbers. Changing header layout, flag meaning, allocation parity, or incompatible lifecycle rules requires a new major and dual-stack/upgrade path.

## Scalability

Bounded queues and requests cap per-connection memory. Stream IDs and schema chunks permit concurrent operations without increasing the frame limit.

## Approval record

| Role | Reviewer | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | PENDING | PENDING | PENDING | PENDING | PENDING |
| Independent reviewer | PENDING | PENDING | PENDING | PENDING | PENDING |
