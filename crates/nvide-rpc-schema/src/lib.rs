//! Committed NRPC schema bindings.

use capnp::{message::ReaderOptions, serialize};
use std::{fmt, io::Cursor};

#[allow(clippy::all, clippy::unwrap_used, dead_code)]
pub mod nrpc_capnp {
    include!("generated/nrpc_capnp.rs");
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Ui,
    Core,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hello {
    pub supported_versions: Vec<Version>,
    pub role: Role,
    pub max_payload: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelloAck {
    pub selected_version: Version,
    pub role: Role,
    pub max_payload: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditRequest {
    pub trace_id: u64,
    pub expected_version: u64,
    pub char_offset: u64,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewportSnapshot {
    pub trace_id: u64,
    pub version: u64,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    IncompatibleMajor,
    MalformedRequest,
    UnknownMethod,
    InvalidArgument,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcError {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug)]
pub enum SchemaError {
    Capnp(capnp::Error),
    UnknownEnum,
    InvalidUtf8,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capnp(error) => write!(formatter, "malformed Cap'n Proto: {error}"),
            Self::UnknownEnum => formatter.write_str("schema contains an unknown enum value"),
            Self::InvalidUtf8 => formatter.write_str("schema text is not valid UTF-8"),
        }
    }
}

impl std::error::Error for SchemaError {}

impl From<capnp::Error> for SchemaError {
    fn from(error: capnp::Error) -> Self {
        Self::Capnp(error)
    }
}

pub fn encode_hello(value: &Hello) -> Result<Vec<u8>, SchemaError> {
    let mut message = capnp::message::Builder::new_default();
    let mut root = message.init_root::<nrpc_capnp::hello::Builder<'_>>();
    root.set_role(role_to_schema(value.role));
    root.set_max_payload(value.max_payload);
    root.set_compression(nrpc_capnp::Compression::None);
    let count = u32::try_from(value.supported_versions.len())
        .map_err(|_| capnp::Error::failed("too many versions".to_owned()))?;
    let mut versions = root.init_supported_versions(count);
    for (index, version) in value.supported_versions.iter().enumerate() {
        let mut item = versions.reborrow().get(index as u32);
        item.set_major(version.major);
        item.set_minor(version.minor);
    }
    Ok(serialize::write_message_to_words(&message))
}

pub fn decode_hello(bytes: &[u8]) -> Result<Hello, SchemaError> {
    let message = serialize::read_message(&mut Cursor::new(bytes), ReaderOptions::new())?;
    let root = message.get_root::<nrpc_capnp::hello::Reader<'_>>()?;
    if root
        .get_compression()
        .map_err(|_| SchemaError::UnknownEnum)?
        != nrpc_capnp::Compression::None
    {
        return Err(SchemaError::UnknownEnum);
    }
    let versions = root.get_supported_versions()?;
    Ok(Hello {
        supported_versions: versions
            .iter()
            .map(|version| Version {
                major: version.get_major(),
                minor: version.get_minor(),
            })
            .collect(),
        role: role_from_schema(root.get_role().map_err(|_| SchemaError::UnknownEnum)?),
        max_payload: root.get_max_payload(),
    })
}

pub fn encode_hello_ack(value: HelloAck) -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    let mut root = message.init_root::<nrpc_capnp::hello_ack::Builder<'_>>();
    root.set_role(role_to_schema(value.role));
    root.set_max_payload(value.max_payload);
    root.set_compression(nrpc_capnp::Compression::None);
    let mut version = root.init_selected_version();
    version.set_major(value.selected_version.major);
    version.set_minor(value.selected_version.minor);
    serialize::write_message_to_words(&message)
}

pub fn decode_hello_ack(bytes: &[u8]) -> Result<HelloAck, SchemaError> {
    let message = serialize::read_message(&mut Cursor::new(bytes), ReaderOptions::new())?;
    let root = message.get_root::<nrpc_capnp::hello_ack::Reader<'_>>()?;
    if root
        .get_compression()
        .map_err(|_| SchemaError::UnknownEnum)?
        != nrpc_capnp::Compression::None
    {
        return Err(SchemaError::UnknownEnum);
    }
    let version = root.get_selected_version()?;
    Ok(HelloAck {
        selected_version: Version {
            major: version.get_major(),
            minor: version.get_minor(),
        },
        role: role_from_schema(root.get_role().map_err(|_| SchemaError::UnknownEnum)?),
        max_payload: root.get_max_payload(),
    })
}

pub fn encode_edit(value: &EditRequest) -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    let mut root = message.init_root::<nrpc_capnp::edit_request::Builder<'_>>();
    root.set_trace_id(value.trace_id);
    root.set_expected_version(value.expected_version);
    root.set_char_offset(value.char_offset);
    root.set_text(value.text.as_str());
    serialize::write_message_to_words(&message)
}

pub fn decode_edit(bytes: &[u8]) -> Result<EditRequest, SchemaError> {
    let message = serialize::read_message(&mut Cursor::new(bytes), ReaderOptions::new())?;
    let root = message.get_root::<nrpc_capnp::edit_request::Reader<'_>>()?;
    Ok(EditRequest {
        trace_id: root.get_trace_id(),
        expected_version: root.get_expected_version(),
        char_offset: root.get_char_offset(),
        text: root
            .get_text()?
            .to_str()
            .map_err(|_| SchemaError::InvalidUtf8)?
            .to_owned(),
    })
}

pub fn encode_viewport(value: &ViewportSnapshot) -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    let mut root = message.init_root::<nrpc_capnp::viewport_snapshot::Builder<'_>>();
    root.set_trace_id(value.trace_id);
    root.set_version(value.version);
    root.set_text(value.text.as_str());
    serialize::write_message_to_words(&message)
}

pub fn decode_viewport(bytes: &[u8]) -> Result<ViewportSnapshot, SchemaError> {
    let message = serialize::read_message(&mut Cursor::new(bytes), ReaderOptions::new())?;
    let root = message.get_root::<nrpc_capnp::viewport_snapshot::Reader<'_>>()?;
    Ok(ViewportSnapshot {
        trace_id: root.get_trace_id(),
        version: root.get_version(),
        text: root
            .get_text()?
            .to_str()
            .map_err(|_| SchemaError::InvalidUtf8)?
            .to_owned(),
    })
}

pub fn encode_error(value: &RpcError) -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    let mut root = message.init_root::<nrpc_capnp::error::Builder<'_>>();
    root.set_code(match value.code {
        ErrorCode::IncompatibleMajor => nrpc_capnp::ErrorCode::IncompatibleMajor,
        ErrorCode::MalformedRequest => nrpc_capnp::ErrorCode::MalformedRequest,
        ErrorCode::UnknownMethod => nrpc_capnp::ErrorCode::UnknownMethod,
        ErrorCode::InvalidArgument => nrpc_capnp::ErrorCode::InvalidArgument,
        ErrorCode::Internal => nrpc_capnp::ErrorCode::Internal,
    });
    root.set_message(value.message.as_str());
    serialize::write_message_to_words(&message)
}

pub fn decode_error(bytes: &[u8]) -> Result<RpcError, SchemaError> {
    let message = serialize::read_message(&mut Cursor::new(bytes), ReaderOptions::new())?;
    let root = message.get_root::<nrpc_capnp::error::Reader<'_>>()?;
    let code = match root.get_code().map_err(|_| SchemaError::UnknownEnum)? {
        nrpc_capnp::ErrorCode::IncompatibleMajor => ErrorCode::IncompatibleMajor,
        nrpc_capnp::ErrorCode::MalformedRequest => ErrorCode::MalformedRequest,
        nrpc_capnp::ErrorCode::UnknownMethod => ErrorCode::UnknownMethod,
        nrpc_capnp::ErrorCode::InvalidArgument => ErrorCode::InvalidArgument,
        nrpc_capnp::ErrorCode::Internal => ErrorCode::Internal,
    };
    Ok(RpcError {
        code,
        message: root
            .get_message()?
            .to_str()
            .map_err(|_| SchemaError::InvalidUtf8)?
            .to_owned(),
    })
}

pub fn encode_heartbeat(sequence: u64) -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    message
        .init_root::<nrpc_capnp::heartbeat::Builder<'_>>()
        .set_sequence(sequence);
    serialize::write_message_to_words(&message)
}

pub fn decode_heartbeat(bytes: &[u8]) -> Result<u64, SchemaError> {
    let message = serialize::read_message(&mut Cursor::new(bytes), ReaderOptions::new())?;
    Ok(message
        .get_root::<nrpc_capnp::heartbeat::Reader<'_>>()?
        .get_sequence())
}

fn role_to_schema(role: Role) -> nrpc_capnp::Role {
    match role {
        Role::Ui => nrpc_capnp::Role::Ui,
        Role::Core => nrpc_capnp::Role::Core,
    }
}

fn role_from_schema(role: nrpc_capnp::Role) -> Role {
    match role {
        nrpc_capnp::Role::Ui => Role::Ui,
        nrpc_capnp::Role::Core => Role::Core,
    }
}

#[cfg(feature = "xtask")]
pub mod xtask;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_messages_roundtrip() -> Result<(), SchemaError> {
        let hello = Hello {
            supported_versions: vec![Version { major: 1, minor: 0 }],
            role: Role::Ui,
            max_payload: 4096,
        };
        assert_eq!(decode_hello(&encode_hello(&hello)?)?, hello);

        let edit = EditRequest {
            trace_id: 7,
            expected_version: 2,
            char_offset: 1,
            text: "λ".to_owned(),
        };
        assert_eq!(decode_edit(&encode_edit(&edit))?, edit);
        Ok(())
    }
}
