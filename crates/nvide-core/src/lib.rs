//! Core editor state: owns buffers and serves NRPC edit requests.

use nvide_buffer::{Buffer, BufferId, RopeBuffer};
use nvide_ipc::{read_message, server_handshake, write_message, HandshakeError};
use nvide_rpc_schema::{flags, stream, ApplyEdit, EditResult, ErrorMsg, Message};

/// In-memory core session with a single default buffer (Phase 0).
pub struct CoreState {
    buffer: RopeBuffer,
}

impl CoreState {
    pub fn new() -> Self {
        Self {
            buffer: RopeBuffer::new(BufferId(1)),
        }
    }

    pub fn with_text(text: &str) -> Self {
        Self {
            buffer: RopeBuffer::from_str(BufferId(1), text),
        }
    }

    pub fn buffer(&self) -> &RopeBuffer {
        &self.buffer
    }

    pub fn apply_edit(&mut self, edit: &ApplyEdit) -> Result<EditResult, String> {
        if edit.buffer_id != self.buffer.id().0 {
            return Err(format!(
                "unknown buffer_id {} (have {})",
                edit.buffer_id,
                self.buffer.id().0
            ));
        }
        let pos = edit.pos as usize;
        let del = edit.delete_len as usize;
        if del > 0 {
            let end = pos
                .checked_add(del)
                .ok_or_else(|| "delete range overflow".to_string())?;
            self.buffer
                .delete_tracked(pos..end)
                .map_err(|e| e.to_string())?;
        }
        if !edit.insert_text.is_empty() {
            self.buffer
                .insert_tracked(pos, &edit.insert_text)
                .map_err(|e| e.to_string())?;
        }
        Ok(EditResult {
            buffer_id: self.buffer.id().0,
            version: self.buffer.version(),
            text: self.buffer.to_string(),
        })
    }
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new()
    }
}

/// Serve one NRPC client connection: handshake then edit loop until EOF.
pub fn serve_connection<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    state: &mut CoreState,
) -> Result<(), HandshakeError> {
    let hello = server_handshake(stream, "nvide-core")?;
    eprintln!(
        "nrpc: handshake ok client={} v{}.{}",
        hello.client_name, hello.major, hello.minor
    );

    loop {
        let (sid, fl, msg) = match read_message(stream) {
            Ok(v) => v,
            Err(HandshakeError::Frame(nvide_ipc::FrameError::UnexpectedEof)) => break,
            Err(HandshakeError::Frame(nvide_ipc::FrameError::Io(e)))
                if e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::ConnectionReset
                    || e.kind() == std::io::ErrorKind::BrokenPipe =>
            {
                break;
            }
            Err(e) => return Err(e),
        };

        match msg {
            Message::ApplyEdit(edit) if sid == stream::EDIT && (fl & flags::REQ) != 0 => {
                match state.apply_edit(&edit) {
                    Ok(result) => {
                        write_message(
                            stream,
                            stream::EDIT,
                            flags::RESP,
                            &Message::EditResult(result),
                        )?;
                    }
                    Err(message) => {
                        write_message(
                            stream,
                            stream::EDIT,
                            flags::RESP | flags::ERR,
                            &Message::Error(ErrorMsg { code: 1, message }),
                        )?;
                    }
                }
            }
            other => {
                write_message(
                    stream,
                    sid,
                    flags::RESP | flags::ERR,
                    &Message::Error(ErrorMsg {
                        code: 2,
                        message: format!(
                            "unsupported message type {} on stream {sid}",
                            other.msg_type().as_u16()
                        ),
                    }),
                )?;
            }
        }
    }
    Ok(())
}

/// Client-side helper: connect already done; handshake + one ApplyEdit + result.
pub fn client_edit_roundtrip<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    client_name: &str,
    edit: ApplyEdit,
) -> Result<EditResult, HandshakeError> {
    let ack = nvide_ipc::client_handshake(stream, client_name)?;
    eprintln!(
        "nrpc: connected to {} v{}.{}",
        ack.server_name, ack.major, ack.minor
    );
    write_message(stream, stream::EDIT, flags::REQ, &Message::ApplyEdit(edit))?;
    let (sid, fl, msg) = read_message(stream)?;
    if sid != stream::EDIT {
        return Err(HandshakeError::UnexpectedStream(sid));
    }
    if fl & flags::RESP == 0 {
        return Err(HandshakeError::ExpectedResponse);
    }
    match msg {
        Message::EditResult(r) => Ok(r),
        Message::Error(e) => Err(HandshakeError::Io(std::io::Error::other(format!(
            "remote error {}: {}",
            e.code, e.message
        )))),
        other => Err(HandshakeError::UnexpectedMessage(other.msg_type().as_u16())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_edit_insert() {
        let mut state = CoreState::new();
        let result = state
            .apply_edit(&ApplyEdit {
                buffer_id: 1,
                pos: 0,
                delete_len: 0,
                insert_text: "hi".into(),
            })
            .unwrap();
        assert_eq!(result.text, "hi");
        assert_eq!(result.version, 1);
    }

    #[cfg(unix)]
    #[test]
    fn in_memory_roundtrip() {
        use std::os::unix::net::UnixStream;

        let mut server = CoreState::new();
        let (mut client_end, mut server_end) = UnixStream::pair().expect("pair");

        let server_thread = std::thread::spawn(move || {
            serve_connection(&mut server_end, &mut server).unwrap();
            server
        });

        let result = client_edit_roundtrip(
            &mut client_end,
            "test-ui",
            ApplyEdit {
                buffer_id: 1,
                pos: 0,
                delete_len: 0,
                insert_text: "hello from ui".into(),
            },
        )
        .unwrap();
        assert_eq!(result.text, "hello from ui");
        drop(client_end);
        let state = server_thread.join().unwrap();
        assert_eq!(state.buffer().to_string(), "hello from ui");
    }
}
