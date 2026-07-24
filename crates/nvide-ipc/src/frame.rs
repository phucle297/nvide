//! Length-prefixed NRPC frames: u32 LE len + stream id + flags + payload.

use std::io::{Read, Write};

/// Bytes after the length field: stream_id (4) + flags (2).
pub const HEADER_AFTER_LEN: usize = 6;

/// Maximum payload size accepted by the prototype decoder (16 MiB).
pub const MAX_PAYLOAD: u32 = 16 * 1024 * 1024;

/// Decoded NRPC frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub stream_id: u32,
    pub flags: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame body length {0} exceeds maximum {MAX_PAYLOAD}")]
    TooLarge(u32),
    #[error("frame body length {0} is shorter than header ({HEADER_AFTER_LEN})")]
    TooShort(u32),
    #[error("unexpected EOF while reading frame")]
    UnexpectedEof,
}

/// Encode a frame. `len` is the length of (stream_id + flags + payload).
pub fn encode_frame(stream_id: u32, flags: u16, payload: &[u8]) -> Vec<u8> {
    let body_len = (HEADER_AFTER_LEN as u32)
        .checked_add(payload.len() as u32)
        .expect("frame body length overflow");
    let mut out = Vec::with_capacity(4 + body_len as usize);
    out.extend_from_slice(&body_len.to_le_bytes());
    out.extend_from_slice(&stream_id.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Write one frame to `w`.
pub fn write_frame<W: Write>(
    w: &mut W,
    stream_id: u32,
    flags: u16,
    payload: &[u8],
) -> Result<(), FrameError> {
    let bytes = encode_frame(stream_id, flags, payload);
    w.write_all(&bytes)?;
    Ok(())
}

/// Decode a frame from a full buffer starting at the length field.
pub fn decode_frame(buf: &[u8]) -> Result<(Frame, usize), FrameError> {
    if buf.len() < 4 {
        return Err(FrameError::UnexpectedEof);
    }
    let body_len = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    if body_len < HEADER_AFTER_LEN as u32 {
        return Err(FrameError::TooShort(body_len));
    }
    if body_len > MAX_PAYLOAD + HEADER_AFTER_LEN as u32 {
        return Err(FrameError::TooLarge(body_len));
    }
    let total = 4 + body_len as usize;
    if buf.len() < total {
        return Err(FrameError::UnexpectedEof);
    }
    let stream_id = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let flags = u16::from_le_bytes(buf[8..10].try_into().unwrap());
    let payload = buf[10..total].to_vec();
    Ok((
        Frame {
            stream_id,
            flags,
            payload,
        },
        total,
    ))
}

/// Read exactly one frame from `r`.
pub fn read_frame<R: Read>(r: &mut R) -> Result<Frame, FrameError> {
    let mut len_buf = [0u8; 4];
    read_exact(r, &mut len_buf)?;
    let body_len = u32::from_le_bytes(len_buf);
    if body_len < HEADER_AFTER_LEN as u32 {
        return Err(FrameError::TooShort(body_len));
    }
    if body_len > MAX_PAYLOAD + HEADER_AFTER_LEN as u32 {
        return Err(FrameError::TooLarge(body_len));
    }
    let mut body = vec![0u8; body_len as usize];
    read_exact(r, &mut body)?;
    let stream_id = u32::from_le_bytes(body[0..4].try_into().unwrap());
    let flags = u16::from_le_bytes(body[4..6].try_into().unwrap());
    let payload = body[6..].to_vec();
    Ok(Frame {
        stream_id,
        flags,
        payload,
    })
}

fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<(), FrameError> {
    let mut read = 0;
    while read < buf.len() {
        match r.read(&mut buf[read..]) {
            Ok(0) => return Err(FrameError::UnexpectedEof),
            Ok(n) => read += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FrameError::Io(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip_fields() {
        let payload = b"hello-nrpc";
        let encoded = encode_frame(42, 0x0005, payload);
        let (frame, consumed) = decode_frame(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(frame.stream_id, 42);
        assert_eq!(frame.flags, 0x0005);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn frame_empty_payload() {
        let encoded = encode_frame(0, 1, b"");
        let (frame, _) = decode_frame(&encoded).unwrap();
        assert_eq!(frame.stream_id, 0);
        assert!(frame.payload.is_empty());
        // len = 6 (stream + flags)
        assert_eq!(&encoded[0..4], &6u32.to_le_bytes());
    }

    #[test]
    fn read_write_stream() {
        let mut cursor = std::io::Cursor::new(Vec::new());
        write_frame(&mut cursor, 7, 3, b"abc").unwrap();
        cursor.set_position(0);
        let frame = read_frame(&mut cursor).unwrap();
        assert_eq!(frame.stream_id, 7);
        assert_eq!(frame.flags, 3);
        assert_eq!(frame.payload, b"abc");
    }
}
