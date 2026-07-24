//! NRPC framing, codec, and session helpers (ADR-0002).

mod codec;
mod frame;
mod session;

pub use codec::{decode_message, encode_message, CodecError};
pub use frame::{decode_frame, encode_frame, Frame, FrameError, HEADER_AFTER_LEN};
pub use session::{read_message, write_message, HandshakeError, Session};

use nvide_rpc_schema::{
    flags, stream, Hello, HelloAck, Message, NRPC_VERSION_MAJOR, NRPC_VERSION_MINOR,
};

/// Perform the versioned client handshake on an already-connected stream.
pub fn client_handshake<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    client_name: &str,
) -> Result<HelloAck, HandshakeError> {
    let hello = Message::Hello(Hello {
        major: NRPC_VERSION_MAJOR,
        minor: NRPC_VERSION_MINOR,
        client_name: client_name.to_string(),
    });
    write_message(stream, stream::CONTROL, flags::REQ, &hello)?;
    let (sid, fl, msg) = read_message(stream)?;
    if sid != stream::CONTROL {
        return Err(HandshakeError::UnexpectedStream(sid));
    }
    if fl & flags::RESP == 0 {
        return Err(HandshakeError::ExpectedResponse);
    }
    match msg {
        Message::HelloAck(ack) => {
            if ack.major != NRPC_VERSION_MAJOR {
                return Err(HandshakeError::VersionMismatch {
                    peer_major: ack.major,
                    peer_minor: ack.minor,
                });
            }
            Ok(ack)
        }
        other => Err(HandshakeError::UnexpectedMessage(other.msg_type().as_u16())),
    }
}

/// Perform the versioned server handshake (read Hello, write HelloAck).
pub fn server_handshake<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    server_name: &str,
) -> Result<Hello, HandshakeError> {
    let (sid, fl, msg) = read_message(stream)?;
    if sid != stream::CONTROL {
        return Err(HandshakeError::UnexpectedStream(sid));
    }
    if fl & flags::REQ == 0 {
        return Err(HandshakeError::ExpectedRequest);
    }
    let hello = match msg {
        Message::Hello(h) => h,
        other => return Err(HandshakeError::UnexpectedMessage(other.msg_type().as_u16())),
    };
    if hello.major != NRPC_VERSION_MAJOR {
        return Err(HandshakeError::VersionMismatch {
            peer_major: hello.major,
            peer_minor: hello.minor,
        });
    }
    let ack = Message::HelloAck(HelloAck {
        major: NRPC_VERSION_MAJOR,
        minor: NRPC_VERSION_MINOR,
        server_name: server_name.to_string(),
    });
    write_message(stream, stream::CONTROL, flags::RESP, &ack)?;
    Ok(hello)
}
