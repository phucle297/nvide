//! Binary message body codec (msg_type u16 LE + fields).

use nvide_rpc_schema::{ApplyEdit, EditResult, ErrorMsg, Hello, HelloAck, Message, MsgType};

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("payload too short for message header")]
    Truncated,
    #[error("unknown message type {0}")]
    UnknownType(u16),
    #[error("invalid utf-8 in string field")]
    InvalidUtf8,
    #[error("length prefix exceeds remaining bytes")]
    BadLength,
}

/// Encode a logical message into a payload buffer.
pub fn encode_message(msg: &Message) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&msg.msg_type().as_u16().to_le_bytes());
    match msg {
        Message::Hello(h) => {
            write_u16(&mut out, h.major);
            write_u16(&mut out, h.minor);
            write_string(&mut out, &h.client_name);
        }
        Message::HelloAck(h) => {
            write_u16(&mut out, h.major);
            write_u16(&mut out, h.minor);
            write_string(&mut out, &h.server_name);
        }
        Message::ApplyEdit(e) => {
            write_u64(&mut out, e.buffer_id);
            write_u64(&mut out, e.pos);
            write_u64(&mut out, e.delete_len);
            write_string(&mut out, &e.insert_text);
        }
        Message::EditResult(e) => {
            write_u64(&mut out, e.buffer_id);
            write_u64(&mut out, e.version);
            write_string(&mut out, &e.text);
        }
        Message::Error(e) => {
            write_u32(&mut out, e.code);
            write_string(&mut out, &e.message);
        }
    }
    out
}

/// Decode a payload into a logical message.
pub fn decode_message(payload: &[u8]) -> Result<Message, CodecError> {
    if payload.len() < 2 {
        return Err(CodecError::Truncated);
    }
    let ty = u16::from_le_bytes(payload[0..2].try_into().unwrap());
    let mut i = 2;
    match MsgType::from_u16(ty) {
        Some(MsgType::Hello) => {
            let major = read_u16(payload, &mut i)?;
            let minor = read_u16(payload, &mut i)?;
            let client_name = read_string(payload, &mut i)?;
            Ok(Message::Hello(Hello {
                major,
                minor,
                client_name,
            }))
        }
        Some(MsgType::HelloAck) => {
            let major = read_u16(payload, &mut i)?;
            let minor = read_u16(payload, &mut i)?;
            let server_name = read_string(payload, &mut i)?;
            Ok(Message::HelloAck(HelloAck {
                major,
                minor,
                server_name,
            }))
        }
        Some(MsgType::ApplyEdit) => {
            let buffer_id = read_u64(payload, &mut i)?;
            let pos = read_u64(payload, &mut i)?;
            let delete_len = read_u64(payload, &mut i)?;
            let insert_text = read_string(payload, &mut i)?;
            Ok(Message::ApplyEdit(ApplyEdit {
                buffer_id,
                pos,
                delete_len,
                insert_text,
            }))
        }
        Some(MsgType::EditResult) => {
            let buffer_id = read_u64(payload, &mut i)?;
            let version = read_u64(payload, &mut i)?;
            let text = read_string(payload, &mut i)?;
            Ok(Message::EditResult(EditResult {
                buffer_id,
                version,
                text,
            }))
        }
        Some(MsgType::Error) => {
            let code = read_u32(payload, &mut i)?;
            let message = read_string(payload, &mut i)?;
            Ok(Message::Error(ErrorMsg { code, message }))
        }
        None => Err(CodecError::UnknownType(ty)),
    }
}

fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_string(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn read_u16(buf: &[u8], i: &mut usize) -> Result<u16, CodecError> {
    if *i + 2 > buf.len() {
        return Err(CodecError::Truncated);
    }
    let v = u16::from_le_bytes(buf[*i..*i + 2].try_into().unwrap());
    *i += 2;
    Ok(v)
}
fn read_u32(buf: &[u8], i: &mut usize) -> Result<u32, CodecError> {
    if *i + 4 > buf.len() {
        return Err(CodecError::Truncated);
    }
    let v = u32::from_le_bytes(buf[*i..*i + 4].try_into().unwrap());
    *i += 4;
    Ok(v)
}
fn read_u64(buf: &[u8], i: &mut usize) -> Result<u64, CodecError> {
    if *i + 8 > buf.len() {
        return Err(CodecError::Truncated);
    }
    let v = u64::from_le_bytes(buf[*i..*i + 8].try_into().unwrap());
    *i += 8;
    Ok(v)
}
fn read_string(buf: &[u8], i: &mut usize) -> Result<String, CodecError> {
    let len = read_u32(buf, i)? as usize;
    if *i + len > buf.len() {
        return Err(CodecError::BadLength);
    }
    let s = std::str::from_utf8(&buf[*i..*i + len]).map_err(|_| CodecError::InvalidUtf8)?;
    *i += len;
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_roundtrip_edit() {
        let msg = Message::ApplyEdit(ApplyEdit {
            buffer_id: 1,
            pos: 0,
            delete_len: 0,
            insert_text: "hello".into(),
        });
        let bytes = encode_message(&msg);
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn message_roundtrip_hello() {
        let msg = Message::Hello(Hello {
            major: 0,
            minor: 1,
            client_name: "ui".into(),
        });
        let decoded = decode_message(&encode_message(&msg)).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn message_roundtrip_result() {
        let msg = Message::EditResult(EditResult {
            buffer_id: 9,
            version: 3,
            text: "αβ".into(),
        });
        assert_eq!(decode_message(&encode_message(&msg)).unwrap(), msg);
    }
}
