//! Read/write framed messages on a stream.

use std::io::{Read, Write};

use nvide_rpc_schema::Message;

use crate::codec::{decode_message, encode_message, CodecError};
use crate::frame::{read_frame, write_frame, FrameError};

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("unexpected stream id {0}")]
    UnexpectedStream(u32),
    #[error("expected REQ flag")]
    ExpectedRequest,
    #[error("expected RESP flag")]
    ExpectedResponse,
    #[error("unexpected message type {0}")]
    UnexpectedMessage(u16),
    #[error("protocol major mismatch (peer {peer_major}.{peer_minor})")]
    VersionMismatch { peer_major: u16, peer_minor: u16 },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Thin session wrapper around a connected duplex stream.
pub struct Session<S> {
    pub stream: S,
}

impl<S: Read + Write> Session<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn write_message(
        &mut self,
        stream_id: u32,
        flags: u16,
        msg: &Message,
    ) -> Result<(), HandshakeError> {
        write_message(&mut self.stream, stream_id, flags, msg)
    }

    pub fn read_message(&mut self) -> Result<(u32, u16, Message), HandshakeError> {
        read_message(&mut self.stream)
    }
}

pub fn write_message<W: Write>(
    w: &mut W,
    stream_id: u32,
    flags: u16,
    msg: &Message,
) -> Result<(), HandshakeError> {
    let payload = encode_message(msg);
    write_frame(w, stream_id, flags, &payload)?;
    Ok(())
}

pub fn read_message<R: Read>(r: &mut R) -> Result<(u32, u16, Message), HandshakeError> {
    let frame = read_frame(r)?;
    let msg = decode_message(&frame.payload)?;
    Ok((frame.stream_id, frame.flags, msg))
}
