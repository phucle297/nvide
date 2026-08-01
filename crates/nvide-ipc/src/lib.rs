//! NRPC framing, version handshake, multiplexing, and bounded queues.

pub use nvide_rpc_schema as schema;
use std::{
    collections::{BTreeSet, VecDeque},
    fmt,
    io::{self, Read, Write},
    sync::{Condvar, Mutex, MutexGuard},
    time::{Duration, Instant},
};

pub use nvide_platform::{monotonic_ns as platform_monotonic_ns, LocalListener, LocalStream};

pub const HEADER_LEN: usize = 10;
pub const MAX_PAYLOAD: usize = 16 * 1024 * 1024;
pub const MAX_OPEN_REQUESTS: usize = 1_024;
pub const MAX_QUEUE_FRAMES: usize = 1_024;
pub const MAX_QUEUE_BYTES: usize = 32 * 1024 * 1024;
const QUEUE_WAIT: Duration = Duration::from_secs(5);

pub const REQ: u16 = 0x0001;
pub const RESP: u16 = 0x0002;
pub const PUSH: u16 = 0x0004;
pub const ERR: u16 = 0x0008;
pub const CANCEL: u16 = 0x0010;
pub const COMPRESSED: u16 = 0x0020;
pub const PRIORITY: u16 = 0x0040;

const KNOWN_FLAGS: u16 = REQ | RESP | PUSH | ERR | CANCEL | COMPRESSED | PRIORITY;
const BASE_FLAGS: u16 = REQ | RESP | PUSH | CANCEL;
pub const MESSAGE_HELLO: u8 = 1;
pub const MESSAGE_HELLO_ACK: u8 = 2;
pub const MESSAGE_ERROR: u8 = 3;
pub const MESSAGE_EDIT: u8 = 10;
pub const MESSAGE_VIEWPORT: u8 = 11;
pub const MESSAGE_HEARTBEAT: u8 = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub stream_id: u32,
    pub flags: u16,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(stream_id: u32, flags: u16, payload: Vec<u8>) -> Result<Self, ProtocolError> {
        validate_flags(flags, &payload)?;
        if payload.len() > MAX_PAYLOAD {
            return Err(ProtocolError::Oversized {
                length: payload.len(),
                maximum: MAX_PAYLOAD,
            });
        }
        Ok(Self {
            stream_id,
            flags,
            payload,
        })
    }

    pub fn write_to(&self, mut writer: impl Write) -> Result<(), ProtocolError> {
        self.write_to_before(&mut writer, Instant::now() + QUEUE_WAIT)
    }

    fn write_to_before(
        &self,
        writer: &mut impl Write,
        deadline: Instant,
    ) -> Result<(), ProtocolError> {
        validate_flags(self.flags, &self.payload)?;
        let length = u32::try_from(self.payload.len()).map_err(|_| ProtocolError::Oversized {
            length: self.payload.len(),
            maximum: MAX_PAYLOAD,
        })?;
        if self.payload.len() > MAX_PAYLOAD {
            return Err(ProtocolError::Oversized {
                length: self.payload.len(),
                maximum: MAX_PAYLOAD,
            });
        }
        write_all_before(writer, &length.to_le_bytes(), deadline)?;
        write_all_before(writer, &self.stream_id.to_le_bytes(), deadline)?;
        write_all_before(writer, &self.flags.to_le_bytes(), deadline)?;
        write_all_before(writer, &self.payload, deadline)?;
        Ok(())
    }

    fn write_once(&self, writer: &mut impl Write) -> Result<(), ProtocolError> {
        let bytes = self.encoded()?;
        match writer.write(&bytes) {
            Ok(written) if written == bytes.len() => Ok(()),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "NRPC cancellation write was partial",
            )
            .into()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn encoded(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.payload.len());
        self.write_to(&mut bytes)?;
        Ok(bytes)
    }
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    TruncatedFrame,
    Oversized { length: usize, maximum: usize },
    UnknownFlags(u16),
    InvalidFlags(u16),
    CompressionUnsupported,
    CancelHasPayload,
    Schema(schema::SchemaError),
    HandshakeRequired,
    InvalidHandshake,
    IncompatibleMajor,
    RoleMismatch,
    StreamIdExhausted,
    InvalidStream(u32),
    Lifecycle(&'static str),
    QueueFull,
    RequestTimeout(u32),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::TruncatedFrame => formatter.write_str("truncated NRPC frame"),
            Self::Oversized { length, maximum } => {
                write!(formatter, "NRPC payload {length} exceeds {maximum}")
            }
            Self::UnknownFlags(flags) => write!(formatter, "unknown NRPC flags {flags:#06x}"),
            Self::InvalidFlags(flags) => write!(formatter, "invalid NRPC flags {flags:#06x}"),
            Self::CompressionUnsupported => formatter.write_str("NRPC compression is unsupported"),
            Self::CancelHasPayload => formatter.write_str("NRPC cancellation must be payload-free"),
            Self::Schema(error) => error.fmt(formatter),
            Self::HandshakeRequired => formatter.write_str("NRPC handshake is not complete"),
            Self::InvalidHandshake => formatter.write_str("invalid NRPC handshake"),
            Self::IncompatibleMajor => formatter.write_str("incompatible NRPC major version"),
            Self::RoleMismatch => formatter.write_str("NRPC peers advertise the same role"),
            Self::StreamIdExhausted => formatter.write_str("NRPC stream IDs are exhausted"),
            Self::InvalidStream(stream) => write!(formatter, "invalid NRPC stream {stream}"),
            Self::Lifecycle(message) => write!(formatter, "NRPC lifecycle error: {message}"),
            Self::QueueFull => formatter.write_str("NRPC queue budget is exhausted"),
            Self::RequestTimeout(stream) => write!(formatter, "NRPC stream {stream} timed out"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<schema::SchemaError> for ProtocolError {
    fn from(error: schema::SchemaError) -> Self {
        Self::Schema(error)
    }
}

pub fn read_frame(mut reader: impl Read, maximum: usize) -> Result<Option<Frame>, ProtocolError> {
    read_frame_before(&mut reader, maximum, Instant::now() + QUEUE_WAIT)
}

fn read_frame_before(
    reader: &mut impl Read,
    maximum: usize,
    deadline: Instant,
) -> Result<Option<Frame>, ProtocolError> {
    let maximum = maximum.min(MAX_PAYLOAD);
    let mut header = [0_u8; HEADER_LEN];
    if read_before(reader, &mut header[..1], deadline)? == 0 {
        return Ok(None);
    }
    read_exact_or_truncated(reader, &mut header[1..], deadline)?;
    let length = u32::from_le_bytes(
        header[0..4]
            .try_into()
            .map_err(|_| ProtocolError::TruncatedFrame)?,
    ) as usize;
    if length > maximum {
        return Err(ProtocolError::Oversized { length, maximum });
    }
    let stream_id = u32::from_le_bytes(
        header[4..8]
            .try_into()
            .map_err(|_| ProtocolError::TruncatedFrame)?,
    );
    let flags = u16::from_le_bytes(
        header[8..10]
            .try_into()
            .map_err(|_| ProtocolError::TruncatedFrame)?,
    );
    let mut payload = vec![0; length];
    read_exact_or_truncated(reader, &mut payload, deadline)?;
    Ok(Some(Frame::new(stream_id, flags, payload)?))
}

fn read_exact_or_truncated(
    reader: &mut impl Read,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), ProtocolError> {
    while !bytes.is_empty() {
        match read_before(reader, bytes, deadline)? {
            0 => return Err(ProtocolError::TruncatedFrame),
            read => bytes = &mut bytes[read..],
        }
    }
    Ok(())
}

fn read_before(
    reader: &mut impl Read,
    bytes: &mut [u8],
    deadline: Instant,
) -> Result<usize, ProtocolError> {
    loop {
        if Instant::now() >= deadline {
            return Err(
                io::Error::new(io::ErrorKind::TimedOut, "NRPC frame read timed out").into(),
            );
        }
        match reader.read(bytes) {
            Ok(read) if Instant::now() < deadline => return Ok(read),
            Ok(_) => {
                return Err(
                    io::Error::new(io::ErrorKind::TimedOut, "NRPC frame read timed out").into(),
                )
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn write_all_before(
    writer: &mut impl Write,
    mut bytes: &[u8],
    deadline: Instant,
) -> Result<(), ProtocolError> {
    while !bytes.is_empty() {
        if Instant::now() >= deadline {
            return Err(
                io::Error::new(io::ErrorKind::TimedOut, "NRPC frame write timed out").into(),
            );
        }
        match writer.write(bytes) {
            Ok(0) => std::thread::sleep(Duration::from_millis(1)),
            Ok(written) if Instant::now() < deadline => bytes = &bytes[written..],
            Ok(_) => {
                return Err(
                    io::Error::new(io::ErrorKind::TimedOut, "NRPC frame write timed out").into(),
                )
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn flush_before(writer: &mut impl Write, deadline: Instant) -> Result<(), ProtocolError> {
    writer.flush()?;
    if Instant::now() >= deadline {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "NRPC flush timed out").into());
    }
    Ok(())
}

fn validate_flags(flags: u16, payload: &[u8]) -> Result<(), ProtocolError> {
    if flags & !KNOWN_FLAGS != 0 {
        return Err(ProtocolError::UnknownFlags(flags & !KNOWN_FLAGS));
    }
    if flags & COMPRESSED != 0 {
        return Err(ProtocolError::CompressionUnsupported);
    }
    if (flags & BASE_FLAGS).count_ones() != 1 || flags & ERR != 0 && flags & RESP == 0 {
        return Err(ProtocolError::InvalidFlags(flags));
    }
    if flags & CANCEL != 0 && !payload.is_empty() {
        return Err(ProtocolError::CancelHasPayload);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Connector,
    Listener,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HandshakeState {
    Awaiting,
    Established,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcceptedFrame {
    Request(Frame),
    Response(Frame),
    Push(Frame),
    Cancel(u32),
    Heartbeat(u64),
}

pub struct Session {
    side: Side,
    role: schema::Role,
    state: HandshakeState,
    maximum: usize,
    next_stream: u32,
    remote_high_water: u32,
    local_allocated: BTreeSet<u32>,
    local_open: BTreeSet<u32>,
    local_cancelled: BTreeSet<u32>,
    local_terminal: BTreeSet<u32>,
    remote_allocated: BTreeSet<u32>,
    remote_open: BTreeSet<u32>,
    remote_cancelled: BTreeSet<u32>,
    remote_terminal: BTreeSet<u32>,
}

impl Session {
    pub fn new(side: Side, role: schema::Role, maximum: usize) -> Self {
        Self {
            side,
            role,
            state: HandshakeState::Awaiting,
            maximum: maximum.min(MAX_PAYLOAD),
            next_stream: if side == Side::Connector { 1 } else { 2 },
            remote_high_water: 0,
            local_allocated: BTreeSet::new(),
            local_open: BTreeSet::new(),
            local_cancelled: BTreeSet::new(),
            local_terminal: BTreeSet::new(),
            remote_allocated: BTreeSet::new(),
            remote_open: BTreeSet::new(),
            remote_cancelled: BTreeSet::new(),
            remote_terminal: BTreeSet::new(),
        }
    }

    pub fn is_established(&self) -> bool {
        self.state == HandshakeState::Established
    }

    pub fn maximum_payload(&self) -> usize {
        self.maximum
    }

    pub fn start_handshake(&self) -> Result<Frame, ProtocolError> {
        if self.side != Side::Connector || self.state != HandshakeState::Awaiting {
            return Err(ProtocolError::InvalidHandshake);
        }
        let hello = schema::Hello {
            supported_versions: vec![schema::Version { major: 1, minor: 0 }],
            role: self.role,
            max_payload: self.maximum as u32,
        };
        Frame::new(
            0,
            REQ,
            message(MESSAGE_HELLO, schema::encode_hello(&hello)?),
        )
    }

    pub fn accept_hello(&mut self, frame: Frame) -> Result<Frame, ProtocolError> {
        if self.side != Side::Listener
            || self.state != HandshakeState::Awaiting
            || frame.stream_id != 0
            || frame.flags & REQ == 0
        {
            self.state = HandshakeState::Closed;
            return Err(ProtocolError::InvalidHandshake);
        }
        let hello = schema::decode_hello(message_body(&frame.payload, MESSAGE_HELLO)?)?;
        if hello.role == self.role {
            self.state = HandshakeState::Closed;
            return Err(ProtocolError::RoleMismatch);
        }
        if !hello
            .supported_versions
            .iter()
            .any(|version| version.major == 1)
        {
            self.state = HandshakeState::Closed;
            let error = schema::RpcError {
                code: schema::ErrorCode::IncompatibleMajor,
                message: "NRPC 1.x is required".to_owned(),
            };
            return Frame::new(
                0,
                RESP | ERR,
                message(MESSAGE_ERROR, schema::encode_error(&error)),
            );
        }
        self.maximum = negotiated_maximum(self.maximum, hello.max_payload as usize);
        self.state = HandshakeState::Established;
        Frame::new(
            0,
            RESP,
            message(
                MESSAGE_HELLO_ACK,
                schema::encode_hello_ack(schema::HelloAck {
                    selected_version: schema::Version { major: 1, minor: 0 },
                    role: self.role,
                    max_payload: self.maximum as u32,
                }),
            ),
        )
    }

    pub fn accept_hello_ack(&mut self, frame: Frame) -> Result<(), ProtocolError> {
        if self.side != Side::Connector
            || self.state != HandshakeState::Awaiting
            || frame.stream_id != 0
            || frame.flags & RESP == 0
        {
            self.state = HandshakeState::Closed;
            return Err(ProtocolError::InvalidHandshake);
        }
        if frame.flags & ERR != 0 {
            let error = schema::decode_error(message_body(&frame.payload, MESSAGE_ERROR)?)?;
            self.state = HandshakeState::Closed;
            return match error.code {
                schema::ErrorCode::IncompatibleMajor => Err(ProtocolError::IncompatibleMajor),
                _ => Err(ProtocolError::InvalidHandshake),
            };
        }
        let ack = schema::decode_hello_ack(message_body(&frame.payload, MESSAGE_HELLO_ACK)?)?;
        if ack.role == self.role {
            self.state = HandshakeState::Closed;
            return Err(ProtocolError::RoleMismatch);
        }
        if ack.selected_version.major != 1 {
            self.state = HandshakeState::Closed;
            return Err(ProtocolError::IncompatibleMajor);
        }
        self.maximum = negotiated_maximum(self.maximum, ack.max_payload as usize);
        self.state = HandshakeState::Established;
        Ok(())
    }

    pub fn open_request(&mut self, payload: Vec<u8>) -> Result<Frame, ProtocolError> {
        self.require_established()?;
        if self.local_open.len() >= MAX_OPEN_REQUESTS {
            return Err(ProtocolError::QueueFull);
        }
        let stream = self.next_stream;
        self.next_stream = self
            .next_stream
            .checked_add(2)
            .ok_or(ProtocolError::StreamIdExhausted)?;
        self.local_allocated.insert(stream);
        self.local_open.insert(stream);
        Frame::new(stream, REQ, payload)
    }

    pub fn cancel(&mut self, stream: u32) -> Result<Frame, ProtocolError> {
        self.require_established()?;
        if !self.local_allocated.contains(&stream) {
            return Err(ProtocolError::InvalidStream(stream));
        }
        self.local_open.remove(&stream);
        self.local_cancelled.insert(stream);
        Frame::new(stream, CANCEL, Vec::new())
    }

    pub fn accept_frame(&mut self, frame: Frame) -> Result<Option<AcceptedFrame>, ProtocolError> {
        self.require_established()?;
        if frame.stream_id == 0 {
            if frame.flags & PUSH == 0 {
                return Err(ProtocolError::Lifecycle("stream 0 is control-only"));
            }
            let sequence =
                schema::decode_heartbeat(message_body(&frame.payload, MESSAGE_HEARTBEAT)?)?;
            return Ok(Some(AcceptedFrame::Heartbeat(sequence)));
        }
        if frame.flags & REQ != 0 {
            self.accept_new_remote(frame.stream_id, false)?;
            self.remote_open.insert(frame.stream_id);
            return Ok(Some(AcceptedFrame::Request(frame)));
        }
        if frame.flags & PUSH != 0 {
            if !self.remote_open.contains(&frame.stream_id) {
                self.accept_new_remote(frame.stream_id, true)?;
            }
            return Ok(Some(AcceptedFrame::Push(frame)));
        }
        if frame.flags & CANCEL != 0 {
            if !self.remote_allocated.contains(&frame.stream_id) {
                return Err(ProtocolError::InvalidStream(frame.stream_id));
            }
            self.remote_open.remove(&frame.stream_id);
            self.remote_cancelled.insert(frame.stream_id);
            return Ok(Some(AcceptedFrame::Cancel(frame.stream_id)));
        }

        if self.local_open.remove(&frame.stream_id) {
            self.local_terminal.insert(frame.stream_id);
            return Ok(Some(AcceptedFrame::Response(frame)));
        }
        if self.local_terminal.contains(&frame.stream_id)
            && self.local_cancelled.contains(&frame.stream_id)
        {
            return Ok(None);
        }
        if self.local_cancelled.contains(&frame.stream_id) {
            self.local_terminal.insert(frame.stream_id);
            return Ok(None);
        }
        Err(ProtocolError::InvalidStream(frame.stream_id))
    }

    pub fn response(&mut self, stream: u32, payload: Vec<u8>) -> Result<Frame, ProtocolError> {
        self.require_established()?;
        self.finish_remote(stream)?;
        Frame::new(stream, RESP, payload)
    }

    pub fn error_response(
        &mut self,
        stream: u32,
        error: schema::RpcError,
    ) -> Result<Frame, ProtocolError> {
        self.require_established()?;
        self.finish_remote(stream)?;
        Frame::new(
            stream,
            RESP | ERR,
            message(MESSAGE_ERROR, schema::encode_error(&error)),
        )
    }

    pub fn heartbeat(sequence: u64) -> Result<Frame, ProtocolError> {
        Frame::new(
            0,
            PUSH,
            message(MESSAGE_HEARTBEAT, schema::encode_heartbeat(sequence)),
        )
    }

    fn accept_new_remote(&mut self, stream: u32, terminal: bool) -> Result<(), ProtocolError> {
        let expected_parity = if self.side == Side::Connector { 0 } else { 1 };
        if stream % 2 != expected_parity || stream <= self.remote_high_water {
            return Err(ProtocolError::InvalidStream(stream));
        }
        if !terminal && self.remote_open.len() >= MAX_OPEN_REQUESTS {
            return Err(ProtocolError::QueueFull);
        }
        self.remote_high_water = stream;
        self.remote_allocated.insert(stream);
        Ok(())
    }

    fn finish_remote(&mut self, stream: u32) -> Result<(), ProtocolError> {
        if self.remote_terminal.contains(&stream)
            || !self.remote_open.remove(&stream) && !self.remote_cancelled.contains(&stream)
        {
            return Err(ProtocolError::InvalidStream(stream));
        }
        self.remote_terminal.insert(stream);
        Ok(())
    }

    fn require_established(&self) -> Result<(), ProtocolError> {
        if self.state == HandshakeState::Established {
            Ok(())
        } else {
            Err(ProtocolError::HandshakeRequired)
        }
    }
}

fn negotiated_maximum(local: usize, remote: usize) -> usize {
    match (local.min(MAX_PAYLOAD), remote.min(MAX_PAYLOAD)) {
        (0, 0) => MAX_PAYLOAD,
        (0, remote) => remote,
        (local, 0) => local,
        (local, remote) => local.min(remote),
    }
}

fn message(kind: u8, body: Vec<u8>) -> Vec<u8> {
    let mut payload = Vec::with_capacity(body.len() + 1);
    payload.push(kind);
    payload.extend(body);
    payload
}

pub fn application_message(kind: u8, body: Vec<u8>) -> Vec<u8> {
    message(kind, body)
}

pub fn application_body(payload: &[u8], expected: u8) -> Result<&[u8], ProtocolError> {
    message_body(payload, expected)
}

fn message_body(payload: &[u8], expected: u8) -> Result<&[u8], ProtocolError> {
    match payload.split_first() {
        Some((&kind, body)) if kind == expected => Ok(body),
        _ => Err(ProtocolError::InvalidHandshake),
    }
}

#[derive(Default)]
pub struct BoundedQueue {
    state: Mutex<QueueState>,
    available: Condvar,
}

#[derive(Default)]
struct QueueState {
    frames: VecDeque<Frame>,
    bytes: usize,
}

impl BoundedQueue {
    pub fn push(&self, frame: Frame) -> Result<(), ProtocolError> {
        self.push_before(frame, Instant::now() + QUEUE_WAIT)
    }

    fn push_before(&self, frame: Frame, deadline: Instant) -> Result<(), ProtocolError> {
        let frame_bytes = HEADER_LEN
            .checked_add(frame.payload.len())
            .ok_or(ProtocolError::QueueFull)?;
        let mut state = self.lock_state();
        loop {
            let next_bytes = state
                .bytes
                .checked_add(frame_bytes)
                .ok_or(ProtocolError::QueueFull)?;
            if state.frames.len() < MAX_QUEUE_FRAMES && next_bytes <= MAX_QUEUE_BYTES {
                state.bytes = next_bytes;
                state.frames.push_back(frame);
                return Ok(());
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Err(ProtocolError::QueueFull);
            };
            let (next_state, timeout) = match self.available.wait_timeout(state, remaining) {
                Ok(result) => result,
                Err(poisoned) => poisoned.into_inner(),
            };
            state = next_state;
            if timeout.timed_out() {
                return Err(ProtocolError::QueueFull);
            }
        }
    }

    pub fn pop(&self) -> Option<Frame> {
        let mut state = self.lock_state();
        let frame = state.frames.pop_front()?;
        state.bytes -= HEADER_LEN + frame.payload.len();
        self.available.notify_one();
        Some(frame)
    }

    pub fn len(&self) -> usize {
        self.lock_state().frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock_state().frames.is_empty()
    }

    fn lock_state(&self) -> MutexGuard<'_, QueueState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

pub struct Client<S> {
    stream: S,
    session: Session,
}

impl<S: Read + Write> Client<S> {
    pub fn connect(mut stream: S, role: schema::Role) -> Result<Self, ProtocolError> {
        let deadline = Instant::now() + QUEUE_WAIT;
        let mut session = Session::new(Side::Connector, role, MAX_PAYLOAD);
        session
            .start_handshake()?
            .write_to_before(&mut stream, deadline)?;
        flush_before(&mut stream, deadline)?;
        let reply = read_frame_before(&mut stream, MAX_PAYLOAD, deadline)?
            .ok_or(ProtocolError::TruncatedFrame)?;
        session.accept_hello_ack(reply)?;
        Ok(Self { stream, session })
    }

    pub fn edit(
        &mut self,
        request: &schema::EditRequest,
    ) -> Result<schema::ViewportSnapshot, ProtocolError> {
        self.edit_before(request, Instant::now() + QUEUE_WAIT)
    }

    pub fn edit_before(
        &mut self,
        request: &schema::EditRequest,
        deadline: Instant,
    ) -> Result<schema::ViewportSnapshot, ProtocolError> {
        let frame = self.session.open_request(application_message(
            MESSAGE_EDIT,
            schema::encode_edit(request),
        ))?;
        let stream_id = frame.stream_id;
        frame.write_to_before(&mut self.stream, deadline)?;
        flush_before(&mut self.stream, deadline)?;
        let reply =
            match read_frame_before(&mut self.stream, self.session.maximum_payload(), deadline) {
                Err(ProtocolError::Io(error))
                    if matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    let cancellation = self.session.cancel(stream_id)?;
                    let _ = cancellation.write_once(&mut self.stream);
                    return Err(ProtocolError::RequestTimeout(stream_id));
                }
                result => result?,
            }
            .ok_or(ProtocolError::TruncatedFrame)?;
        let accepted = self
            .session
            .accept_frame(reply)?
            .ok_or(ProtocolError::Lifecycle("terminal response was ignored"))?;
        let result = match accepted {
            AcceptedFrame::Response(frame) if frame.flags & ERR == 0 => Ok(
                schema::decode_viewport(application_body(&frame.payload, MESSAGE_VIEWPORT)?)?,
            ),
            AcceptedFrame::Response(frame) => {
                let error = schema::decode_error(application_body(&frame.payload, MESSAGE_ERROR)?)?;
                Err(ProtocolError::Lifecycle(match error.code {
                    schema::ErrorCode::IncompatibleMajor => "incompatible major",
                    schema::ErrorCode::MalformedRequest => "malformed request",
                    schema::ErrorCode::UnknownMethod => "unknown method",
                    schema::ErrorCode::InvalidArgument => "invalid argument",
                    schema::ErrorCode::Internal => "core failure",
                }))
            }
            _ => Err(ProtocolError::Lifecycle("expected edit response")),
        };
        if Instant::now() >= deadline {
            return Err(ProtocolError::RequestTimeout(stream_id));
        }
        result
    }

    pub fn heartbeat(&mut self, sequence: u64) -> Result<(), ProtocolError> {
        self.heartbeat_before(sequence, Instant::now() + QUEUE_WAIT)
    }

    pub fn heartbeat_before(
        &mut self,
        sequence: u64,
        deadline: Instant,
    ) -> Result<(), ProtocolError> {
        Session::heartbeat(sequence)?.write_to_before(&mut self.stream, deadline)?;
        flush_before(&mut self.stream, deadline)?;
        let reply = read_frame_before(&mut self.stream, self.session.maximum_payload(), deadline)?
            .ok_or(ProtocolError::TruncatedFrame)?;
        match self.session.accept_frame(reply)? {
            Some(AcceptedFrame::Heartbeat(received)) if received == sequence => Ok(()),
            _ => Err(ProtocolError::Lifecycle("heartbeat sequence mismatch")),
        }
    }
}

pub fn serve<S, F>(mut stream: S, mut apply_edit: F) -> Result<(), ProtocolError>
where
    S: Read + Write,
    F: FnMut(schema::EditRequest) -> Result<schema::ViewportSnapshot, schema::RpcError>,
{
    let handshake_deadline = Instant::now() + QUEUE_WAIT;
    let mut session = Session::new(Side::Listener, schema::Role::Core, MAX_PAYLOAD);
    let hello = read_frame_before(&mut stream, MAX_PAYLOAD, handshake_deadline)?
        .ok_or(ProtocolError::TruncatedFrame)?;
    let reply = session.accept_hello(hello)?;
    reply.write_to_before(&mut stream, handshake_deadline)?;
    flush_before(&mut stream, handshake_deadline)?;
    if !session.is_established() {
        return Err(ProtocolError::IncompatibleMajor);
    }

    while let Some(frame) = read_frame(&mut stream, session.maximum_payload())? {
        match session.accept_frame(frame)? {
            Some(AcceptedFrame::Request(frame)) => {
                let response = match application_body(&frame.payload, MESSAGE_EDIT) {
                    Err(_) => session.error_response(
                        frame.stream_id,
                        schema::RpcError {
                            code: schema::ErrorCode::UnknownMethod,
                            message: "unknown Phase 0 method".to_owned(),
                        },
                    )?,
                    Ok(body) => {
                        let edit = schema::decode_edit(body)?;
                        match apply_edit(edit) {
                            Ok(viewport) => session.response(
                                frame.stream_id,
                                application_message(
                                    MESSAGE_VIEWPORT,
                                    schema::encode_viewport(&viewport),
                                ),
                            )?,
                            Err(error) => session.error_response(frame.stream_id, error)?,
                        }
                    }
                };
                response.write_to(&mut stream)?;
                stream.flush()?;
            }
            Some(AcceptedFrame::Heartbeat(sequence)) => {
                Session::heartbeat(sequence)?.write_to(&mut stream)?;
                stream.flush()?;
            }
            Some(_) | None => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SlowReader {
        bytes: Vec<u8>,
        offset: usize,
    }

    impl Read for SlowReader {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            std::thread::sleep(Duration::from_millis(50));
            if self.offset >= self.bytes.len() || output.is_empty() {
                return Ok(0);
            }
            output[0] = self.bytes[self.offset];
            self.offset += 1;
            Ok(1)
        }
    }

    struct StalledWriter;

    impl Write for StalledWriter {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::WouldBlock, "stalled"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn frame_codec_rejects_bad_input() -> Result<(), ProtocolError> {
        let frame = Frame::new(3, REQ, b"hello".to_vec())?;
        assert_eq!(
            read_frame(frame.encoded()?.as_slice(), MAX_PAYLOAD)?,
            Some(frame)
        );
        assert!(matches!(
            read_frame(&[1, 2][..], MAX_PAYLOAD),
            Err(ProtocolError::TruncatedFrame)
        ));

        let mut oversized = vec![0; HEADER_LEN];
        oversized[0..4].copy_from_slice(&((MAX_PAYLOAD as u32) + 1).to_le_bytes());
        assert!(matches!(
            read_frame(oversized.as_slice(), MAX_PAYLOAD),
            Err(ProtocolError::Oversized { .. })
        ));
        assert!(matches!(
            Frame::new(1, REQ | RESP, Vec::new()),
            Err(ProtocolError::InvalidFlags(_))
        ));
        assert!(matches!(
            Frame::new(1, REQ | 0x8000, Vec::new()),
            Err(ProtocolError::UnknownFlags(_))
        ));
        assert!(matches!(
            Frame::new(1, COMPRESSED | REQ, Vec::new()),
            Err(ProtocolError::CompressionUnsupported)
        ));
        Ok(())
    }

    #[test]
    fn frame_deadlines_cover_the_aggregate_operation() -> Result<(), ProtocolError> {
        let frame = Frame::new(1, REQ, b"slow".to_vec())?;
        let mut reader = SlowReader {
            bytes: frame.encoded()?,
            offset: 0,
        };
        let started = Instant::now();
        assert!(matches!(
            read_frame_before(
                &mut reader,
                MAX_PAYLOAD,
                Instant::now() + Duration::from_millis(100)
            ),
            Err(ProtocolError::Io(ref error)) if error.kind() == io::ErrorKind::TimedOut
        ));
        assert!(started.elapsed() < Duration::from_millis(500));

        let mut writer = StalledWriter;
        assert!(matches!(
            write_all_before(
                &mut writer,
                b"frame",
                Instant::now() + Duration::from_millis(100)
            ),
            Err(ProtocolError::Io(ref error)) if error.kind() == io::ErrorKind::TimedOut
        ));
        Ok(())
    }

    #[test]
    fn handshake_negotiates_limit_and_compatible_minor() -> Result<(), ProtocolError> {
        let mut connector = Session::new(Side::Connector, schema::Role::Ui, 4096);
        let mut listener = Session::new(Side::Listener, schema::Role::Core, 2048);
        let mut hello = schema::decode_hello(application_body(
            &connector.start_handshake()?.payload,
            MESSAGE_HELLO,
        )?)?;
        hello.supported_versions = vec![schema::Version { major: 1, minor: 9 }];
        let reply = listener.accept_hello(Frame::new(
            0,
            REQ,
            application_message(MESSAGE_HELLO, schema::encode_hello(&hello)?),
        )?)?;
        connector.accept_hello_ack(reply)?;
        assert!(connector.is_established() && listener.is_established());
        assert_eq!(connector.maximum_payload(), 2048);
        Ok(())
    }

    #[test]
    fn incompatible_major_returns_error_and_closes() -> Result<(), ProtocolError> {
        let mut listener = Session::new(Side::Listener, schema::Role::Core, MAX_PAYLOAD);
        let hello = schema::Hello {
            supported_versions: vec![schema::Version { major: 2, minor: 0 }],
            role: schema::Role::Ui,
            max_payload: MAX_PAYLOAD as u32,
        };
        let reply = listener.accept_hello(Frame::new(
            0,
            REQ,
            application_message(MESSAGE_HELLO, schema::encode_hello(&hello)?),
        )?)?;
        assert_eq!(reply.flags, RESP | ERR);
        assert!(!listener.is_established());
        Ok(())
    }

    #[test]
    fn streams_enforce_parity_branching_and_cancellation() -> Result<(), ProtocolError> {
        let (mut connector, mut listener) = connected_sessions()?;
        let request = connector.open_request(vec![MESSAGE_EDIT])?;
        assert_eq!(request.stream_id, 1);
        assert!(matches!(
            listener.accept_frame(request.clone())?,
            Some(AcceptedFrame::Request(_))
        ));
        let response = listener.response(request.stream_id, vec![MESSAGE_VIEWPORT])?;
        assert!(matches!(
            connector.accept_frame(response)?,
            Some(AcceptedFrame::Response(_))
        ));

        let second = connector.open_request(vec![MESSAGE_EDIT])?;
        listener.accept_frame(second.clone())?;
        listener.accept_frame(connector.cancel(second.stream_id)?)?;
        let raced = listener.response(second.stream_id, Vec::new())?;
        assert!(listener.response(second.stream_id, Vec::new()).is_err());
        assert!(connector.accept_frame(raced)?.is_none());
        assert!(listener
            .accept_frame(Frame::new(4, REQ, Vec::new())?)
            .is_err());
        Ok(())
    }

    #[test]
    fn generated_headers_never_panic() {
        let mut state = 11_u64;
        for length in 0..256 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let mut bytes = state.to_le_bytes().repeat(length / 8 + 1);
            bytes.truncate(length);
            let _ = read_frame(bytes.as_slice(), 4096);
        }
    }

    #[test]
    fn bounded_queue_waits_and_times_out() -> Result<(), ProtocolError> {
        let queue = std::sync::Arc::new(BoundedQueue::default());
        for stream in 1..=MAX_QUEUE_FRAMES {
            queue.push(Frame::new(stream as u32, PUSH, Vec::new())?)?;
        }
        assert!(matches!(
            queue.push_before(
                Frame::new((MAX_QUEUE_FRAMES + 1) as u32, PUSH, Vec::new())?,
                Instant::now()
            ),
            Err(ProtocolError::QueueFull)
        ));

        let producer = std::sync::Arc::clone(&queue);
        let waiting = std::thread::spawn(move || {
            producer.push(Frame::new((MAX_QUEUE_FRAMES + 2) as u32, PUSH, Vec::new())?)
        });
        std::thread::sleep(Duration::from_millis(10));
        assert!(queue.pop().is_some());
        waiting
            .join()
            .map_err(|_| ProtocolError::Lifecycle("queue thread panicked"))??;
        assert_eq!(queue.len(), MAX_QUEUE_FRAMES);
        Ok(())
    }

    #[test]
    fn request_timeout_sends_cancel() -> Result<(), ProtocolError> {
        let reply = Frame::new(
            0,
            RESP,
            application_message(
                MESSAGE_HELLO_ACK,
                schema::encode_hello_ack(schema::HelloAck {
                    selected_version: schema::Version { major: 1, minor: 0 },
                    role: schema::Role::Core,
                    max_payload: MAX_PAYLOAD as u32,
                }),
            ),
        )?
        .encoded()?;
        let mut client = Client::connect(TimeoutStream::new(reply), schema::Role::Ui)?;
        let started = Instant::now();
        assert!(matches!(
            client.edit_before(
                &schema::EditRequest {
                    trace_id: 1,
                    expected_version: 0,
                    char_offset: 0,
                    text: "x".to_owned(),
                    dispatch_ns: 1,
                },
                Instant::now() + Duration::from_millis(20)
            ),
            Err(ProtocolError::RequestTimeout(1))
        ));
        assert!(started.elapsed() < Duration::from_millis(500));

        let mut written = client.stream.written.as_slice();
        assert_eq!(
            read_frame(&mut written, MAX_PAYLOAD)?.map(|frame| frame.stream_id),
            Some(0)
        );
        assert_eq!(
            read_frame(&mut written, MAX_PAYLOAD)?.map(|frame| frame.flags),
            Some(REQ)
        );
        assert_eq!(
            read_frame(&mut written, MAX_PAYLOAD)?.map(|frame| frame.flags),
            Some(CANCEL)
        );
        Ok(())
    }

    #[test]
    fn edit_deadline_is_shared_across_write_and_read() -> Result<(), ProtocolError> {
        let (session, _) = connected_sessions()?;
        let stream = SlowTransactionStream {
            written: Vec::new(),
            writes: 0,
        };
        let mut client = Client { stream, session };
        let started = Instant::now();
        assert!(matches!(
            client.edit_before(
                &schema::EditRequest {
                    trace_id: 1,
                    expected_version: 0,
                    char_offset: 0,
                    text: "x".to_owned(),
                    dispatch_ns: 1,
                },
                Instant::now() + Duration::from_millis(30)
            ),
            Err(ProtocolError::RequestTimeout(1))
        ));
        assert!(started.elapsed() < Duration::from_millis(500));
        let mut written = client.stream.written.as_slice();
        assert_eq!(
            read_frame(&mut written, MAX_PAYLOAD)?.map(|frame| frame.flags),
            Some(REQ)
        );
        assert_eq!(
            read_frame(&mut written, MAX_PAYLOAD)?.map(|frame| frame.flags),
            Some(CANCEL)
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn real_stream_roundtrip_uses_generated_messages() -> Result<(), Box<dyn std::error::Error>> {
        let (client_stream, server_stream) = nvide_platform::LocalStream::pair()?;
        let server = std::thread::spawn(move || {
            serve(server_stream, |edit| {
                Ok(schema::ViewportSnapshot {
                    trace_id: edit.trace_id,
                    version: edit.expected_version + 1,
                    text: edit.text,
                    core_received_ns: edit.dispatch_ns + 1,
                    version_increment_ns: edit.dispatch_ns + 2,
                    viewport_emit_ns: edit.dispatch_ns + 3,
                })
            })
        });
        let mut client = Client::connect(client_stream, schema::Role::Ui)?;
        client.heartbeat(4)?;
        let viewport = client.edit(&schema::EditRequest {
            trace_id: 9,
            expected_version: 0,
            char_offset: 0,
            text: "x".to_owned(),
            dispatch_ns: 1,
        })?;
        assert_eq!(viewport.version, 1);
        drop(client);
        server.join().map_err(|_| "server thread panicked")??;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn malformed_schema_closes_real_stream() -> Result<(), Box<dyn std::error::Error>> {
        let (client_stream, server_stream) = nvide_platform::LocalStream::pair()?;
        let server = std::thread::spawn(move || serve(server_stream, |_| unreachable!()));
        let mut client = Client::connect(client_stream, schema::Role::Ui)?;
        client
            .session
            .open_request(application_message(MESSAGE_EDIT, vec![0xff]))?
            .write_to(&mut client.stream)?;
        client.stream.flush()?;
        let result = server.join().map_err(|_| "server thread panicked")?;
        assert!(matches!(result, Err(ProtocolError::Schema(_))));
        assert!(read_frame(&mut client.stream, MAX_PAYLOAD)?.is_none());
        Ok(())
    }

    struct TimeoutStream {
        reply: std::io::Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    struct SlowTransactionStream {
        written: Vec<u8>,
        writes: usize,
    }

    impl Read for SlowTransactionStream {
        fn read(&mut self, _bytes: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "stalled response",
            ))
        }
    }

    impl Write for SlowTransactionStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.writes < 4 {
                std::thread::sleep(Duration::from_millis(4));
            }
            self.writes += 1;
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl TimeoutStream {
        fn new(reply: Vec<u8>) -> Self {
            Self {
                reply: std::io::Cursor::new(reply),
                written: Vec::new(),
            }
        }
    }

    impl Read for TimeoutStream {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if self.reply.position() < self.reply.get_ref().len() as u64 {
                self.reply.read(bytes)
            } else {
                Err(io::Error::new(io::ErrorKind::TimedOut, "test timeout"))
            }
        }
    }

    impl Write for TimeoutStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn connected_sessions() -> Result<(Session, Session), ProtocolError> {
        let mut connector = Session::new(Side::Connector, schema::Role::Ui, MAX_PAYLOAD);
        let mut listener = Session::new(Side::Listener, schema::Role::Core, MAX_PAYLOAD);
        let reply = listener.accept_hello(connector.start_handshake()?)?;
        connector.accept_hello_ack(reply)?;
        Ok((connector, listener))
    }
}
