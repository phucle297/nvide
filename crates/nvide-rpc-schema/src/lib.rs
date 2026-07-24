//! NRPC message schema types (Phase 0 prototype).
//!
//! Full Cap'n Proto codegen is deferred when the toolchain is heavy; the wire
//! layout (framed NRPC) and versioned handshake still match ADR-0002. Message
//! bodies use a compact binary layout described by [`schema_document`].

use serde::{Deserialize, Serialize};

/// Protocol major.minor for the versioned handshake.
pub const NRPC_VERSION_MAJOR: u16 = 0;
pub const NRPC_VERSION_MINOR: u16 = 1;

/// Well-known stream IDs for the Phase 0 prototype.
pub mod stream {
    /// Control / handshake stream.
    pub const CONTROL: u32 = 0;
    /// Editor edit request/response stream.
    pub const EDIT: u32 = 1;
}

/// Frame flag bits (ADR-0002).
pub mod flags {
    pub const REQ: u16 = 1 << 0;
    pub const RESP: u16 = 1 << 1;
    pub const PUSH: u16 = 1 << 2;
    pub const ERR: u16 = 1 << 3;
    pub const CANCEL: u16 = 1 << 4;
    pub const COMPRESSED: u16 = 1 << 5;
    pub const PRIORITY: u16 = 1 << 6;
}

/// Message type tags in the payload (u16 LE prefix of body).
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgType {
    Hello = 1,
    HelloAck = 2,
    ApplyEdit = 3,
    EditResult = 4,
    Error = 5,
}

impl MsgType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::Hello),
            2 => Some(Self::HelloAck),
            3 => Some(Self::ApplyEdit),
            4 => Some(Self::EditResult),
            5 => Some(Self::Error),
            _ => None,
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Client → server hello.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub major: u16,
    pub minor: u16,
    pub client_name: String,
}

/// Server → client hello acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAck {
    pub major: u16,
    pub minor: u16,
    pub server_name: String,
}

/// Apply a text edit to the core buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyEdit {
    pub buffer_id: u64,
    /// Character index.
    pub pos: u64,
    /// Characters to delete starting at `pos` (0 for pure insert).
    pub delete_len: u64,
    /// UTF-8 text to insert at `pos` after delete.
    pub insert_text: String,
}

/// Result of an edit: full buffer text for Phase 0 (snapshot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditResult {
    pub buffer_id: u64,
    pub version: u64,
    pub text: String,
}

/// Error payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMsg {
    pub code: u32,
    pub message: String,
}

/// High-level message enum used by the codec helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Hello(Hello),
    HelloAck(HelloAck),
    ApplyEdit(ApplyEdit),
    EditResult(EditResult),
    Error(ErrorMsg),
}

impl Message {
    pub fn msg_type(&self) -> MsgType {
        match self {
            Message::Hello(_) => MsgType::Hello,
            Message::HelloAck(_) => MsgType::HelloAck,
            Message::ApplyEdit(_) => MsgType::ApplyEdit,
            Message::EditResult(_) => MsgType::EditResult,
            Message::Error(_) => MsgType::Error,
        }
    }
}

/// JSON Schema document describing Phase 0 NRPC messages (written by xtask).
pub fn schema_document() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://nvide.dev/schemas/nrpc-v0.1.json",
        "title": "NVide NRPC Phase 0",
        "description": "Framed multiplexed IPC (u32 LE len + stream id + flags + payload). Payload starts with msg_type u16 LE then message-specific fields. Cap'n Proto may replace the body encoder later without changing the frame header.",
        "nrpc": {
            "version": { "major": NRPC_VERSION_MAJOR, "minor": NRPC_VERSION_MINOR },
            "frame": {
                "len": "u32 LE — length of (stream_id + flags + payload)",
                "stream_id": "u32 LE",
                "flags": "u16 LE — REQ|RESP|PUSH|ERR|CANCEL|COMPRESSED|PRIORITY",
                "payload": "bytes"
            },
            "flags": {
                "REQ": flags::REQ,
                "RESP": flags::RESP,
                "PUSH": flags::PUSH,
                "ERR": flags::ERR,
                "CANCEL": flags::CANCEL,
                "COMPRESSED": flags::COMPRESSED,
                "PRIORITY": flags::PRIORITY
            },
            "streams": {
                "CONTROL": stream::CONTROL,
                "EDIT": stream::EDIT
            }
        },
        "messages": {
            "Hello": {
                "type": MsgType::Hello.as_u16(),
                "fields": ["major:u16", "minor:u16", "client_name:string"]
            },
            "HelloAck": {
                "type": MsgType::HelloAck.as_u16(),
                "fields": ["major:u16", "minor:u16", "server_name:string"]
            },
            "ApplyEdit": {
                "type": MsgType::ApplyEdit.as_u16(),
                "fields": ["buffer_id:u64", "pos:u64", "delete_len:u64", "insert_text:string"]
            },
            "EditResult": {
                "type": MsgType::EditResult.as_u16(),
                "fields": ["buffer_id:u64", "version:u64", "text:string"]
            },
            "Error": {
                "type": MsgType::Error.as_u16(),
                "fields": ["code:u32", "message:string"]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_frame_and_version() {
        let doc = schema_document();
        assert_eq!(doc["nrpc"]["version"]["major"], NRPC_VERSION_MAJOR);
        assert!(doc["messages"]["ApplyEdit"].is_object());
        assert!(doc["nrpc"]["frame"]["len"].is_string());
    }
}
