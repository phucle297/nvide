# ADR-0002: NRPC IPC protocol

- **Status:** Accepted
- **Date:** 2026-07-23
- **Tags:** ipc, multiprocess, protocol

## Context

NVide is a multi-process IDE: a thin UI process, a core editor process, and
future plugin/language/agent hosts. Inter-process communication must support
low-latency edits, multiplexed streams (request/response, push, binary blobs),
and evolution over time without forcing a full monorepo freeze.

Cross-platform transports differ (Unix domain sockets, Windows named pipes,
remote channels). The wire protocol must be transport-agnostic and versioned.

## Decision

Adopt **NVide RPC (NRPC)**: a framed, multiplexed, bidirectional protocol.

**Wire frame layout:**

```
┌──────────┬──────────┬────────┬────────────────────────────┐
│ len u32  │ stream   │ flags  │ payload                    │
│   LE     │ id u32   │ u16 LE │                            │
└──────────┴──────────┴────────┴────────────────────────────┘
```

- `len` is the length in bytes of `(stream_id + flags + payload)`.
- Flags: `REQ | RESP | PUSH | ERR | CANCEL | COMPRESSED | PRIORITY`.
- **Versioned handshake** carries `major.minor`; minor is backward compatible;
  major requires dual-stack or coordinated upgrade.
- **Serialization (target):** Cap’n Proto for hot paths; MessagePack allowed for
  the Lua plugin bridge. Phase 0 ships a compact interim binary body codec with
  the same frame header and message catalog so schema generation and multi-process
  edit roundtrips work without a full Cap’n Proto toolchain.

Transports: Unix domain sockets (Linux/macOS), named pipes (Windows),
vsock/SSH channel (remote).

## Alternatives

1. **JSON-RPC over stdio/WebSocket** — simple, poor zero-copy and framing control.
2. **gRPC / HTTP2** — heavy runtime, awkward for local UDS and binary PTY streams.
3. **Raw custom length-prefixed JSON** — easy to debug, weak typing and evolution.
4. **Neovim msgpack-rpc as the only bus** — couples core to compat concerns.

## Pros/Cons

**Pros**

- Explicit multiplexing and flags match IDE traffic patterns (edits vs paint vs LSP).
- Transport-agnostic framing supports local and remote with one schema family.
- Versioned handshake allows controlled protocol evolution (S4 API tier).

**Cons**

- Custom protocol needs docs, fuzzing, and careful compatibility tests.
- Cap’n Proto toolchain adds build complexity (mitigated by interim codec).

## Consequences

- All process pairs speak NRPC; UI remains free of direct core memory access.
- `nvide-ipc` owns framing/codec; `nvide-rpc-schema` owns message catalog and
  schema documents generated into `schemas/` via `cargo xtask schema-gen`.
- Heartbeats, restart budgets, and coalescing policies layer on top of streams.

## Migration

- Phase 0: frame + handshake + `Hello` / `ApplyEdit` / `EditResult`.
- Later: Cap’n Proto bodies behind the same frame; dual-decode if needed during
  a major bump; never reuse field/message numbers.

## Scalability

- Multiplexed streams avoid connection-per-feature blowups.
- Coalesced UI paint streams and cancelable LSP requests keep latency budgets
  under multi-GB buffers and many plugins.
- Remote agent uses the same schema with extensions (ADR-0009).
