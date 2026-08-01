use nvide_ipc::schema;
use std::{error::Error, io::Write, process::Command};

#[test]
fn core_failure_restarts_and_rebinds_over_local_transport() -> Result<(), Box<dyn Error>> {
    let endpoint = test_endpoint("restart")?;
    let listener = nvide_ipc::LocalListener::bind(&endpoint)?;

    let (mut first, mut first_client) = spawn(&listener, &endpoint)?;
    first_client.heartbeat(1)?;
    first.kill()?;
    first.wait()?;
    assert!(first_client.heartbeat(2).is_err());
    drop(first_client);

    let (mut second, mut second_client) = spawn(&listener, &endpoint)?;
    second_client.heartbeat(3)?;
    let viewport = second_client.edit(&schema::EditRequest {
        trace_id: 7,
        expected_version: 0,
        char_offset: 0,
        text: "restart-ok".to_owned(),
    })?;
    assert_eq!((viewport.trace_id, viewport.version), (7, 1));
    assert_eq!(viewport.text, "restart-ok");
    second.kill()?;
    second.wait()?;
    Ok(())
}

#[test]
fn core_process_enforces_version_handshake() -> Result<(), Box<dyn Error>> {
    let endpoint = test_endpoint("compatible-version")?;
    let listener = nvide_ipc::LocalListener::bind(&endpoint)?;
    let (mut child, mut stream) = spawn_raw(&listener, &endpoint)?;
    let reply = send_hello(&mut stream, 1, 9)?;
    assert_eq!(reply.flags, nvide_ipc::RESP);
    let ack = schema::decode_hello_ack(nvide_ipc::application_body(
        &reply.payload,
        nvide_ipc::MESSAGE_HELLO_ACK,
    )?)?;
    assert_eq!(ack.selected_version, schema::Version { major: 1, minor: 0 });
    child.kill()?;
    child.wait()?;

    let endpoint = test_endpoint("incompatible-version")?;
    let listener = nvide_ipc::LocalListener::bind(&endpoint)?;
    let (mut child, mut stream) = spawn_raw(&listener, &endpoint)?;
    let reply = send_hello(&mut stream, 2, 0)?;
    assert_eq!(reply.flags, nvide_ipc::RESP | nvide_ipc::ERR);
    let error = schema::decode_error(nvide_ipc::application_body(
        &reply.payload,
        nvide_ipc::MESSAGE_ERROR,
    )?)?;
    assert_eq!(error.code, schema::ErrorCode::IncompatibleMajor);
    assert!(!child.wait()?.success());
    Ok(())
}

#[test]
fn core_process_closes_on_malformed_oversized_and_truncated_frames() -> Result<(), Box<dyn Error>> {
    let endpoint = test_endpoint("malformed")?;
    let listener = nvide_ipc::LocalListener::bind(&endpoint)?;
    let (child, mut stream) = spawn_raw(&listener, &endpoint)?;
    establish(&mut stream)?;
    nvide_ipc::Frame::new(
        1,
        nvide_ipc::REQ,
        nvide_ipc::application_message(nvide_ipc::MESSAGE_EDIT, vec![0xff]),
    )?
    .write_to(&mut stream)?;
    stream.flush()?;
    drop(stream);
    assert_failed(child)?;

    let endpoint = test_endpoint("oversized")?;
    let listener = nvide_ipc::LocalListener::bind(&endpoint)?;
    let (child, mut stream) = spawn_raw(&listener, &endpoint)?;
    establish(&mut stream)?;
    let mut header = [0_u8; nvide_ipc::HEADER_LEN];
    header[..4].copy_from_slice(&((nvide_ipc::MAX_PAYLOAD as u32) + 1).to_le_bytes());
    header[4..8].copy_from_slice(&1_u32.to_le_bytes());
    header[8..].copy_from_slice(&nvide_ipc::REQ.to_le_bytes());
    stream.write_all(&header)?;
    stream.flush()?;
    drop(stream);
    assert_failed(child)?;

    let endpoint = test_endpoint("truncated")?;
    let listener = nvide_ipc::LocalListener::bind(&endpoint)?;
    let (child, mut stream) = spawn_raw(&listener, &endpoint)?;
    establish(&mut stream)?;
    stream.write_all(&[1, 0])?;
    stream.flush()?;
    drop(stream);
    assert_failed(child)?;
    Ok(())
}

fn spawn(
    listener: &nvide_ipc::LocalListener,
    endpoint: &str,
) -> Result<
    (
        std::process::Child,
        nvide_ipc::Client<nvide_ipc::LocalStream>,
    ),
    Box<dyn Error>,
> {
    let child = Command::new(env!("CARGO_BIN_EXE_nvide-core"))
        .arg("--endpoint")
        .arg(endpoint)
        .spawn()?;
    let stream = listener.accept()?;
    let client = nvide_ipc::Client::connect(stream, schema::Role::Ui)?;
    Ok((child, client))
}

fn spawn_raw(
    listener: &nvide_ipc::LocalListener,
    endpoint: &str,
) -> Result<(std::process::Child, nvide_ipc::LocalStream), Box<dyn Error>> {
    let child = Command::new(env!("CARGO_BIN_EXE_nvide-core"))
        .arg("--endpoint")
        .arg(endpoint)
        .spawn()?;
    Ok((child, listener.accept()?))
}

fn send_hello(
    stream: &mut nvide_ipc::LocalStream,
    major: u16,
    minor: u16,
) -> Result<nvide_ipc::Frame, Box<dyn Error>> {
    nvide_ipc::Frame::new(
        0,
        nvide_ipc::REQ,
        nvide_ipc::application_message(
            nvide_ipc::MESSAGE_HELLO,
            schema::encode_hello(&schema::Hello {
                supported_versions: vec![schema::Version { major, minor }],
                role: schema::Role::Ui,
                max_payload: nvide_ipc::MAX_PAYLOAD as u32,
            })?,
        ),
    )?
    .write_to(&mut *stream)?;
    stream.flush()?;
    nvide_ipc::read_frame(stream, nvide_ipc::MAX_PAYLOAD)?
        .ok_or_else(|| "core closed during handshake".into())
}

fn establish(stream: &mut nvide_ipc::LocalStream) -> Result<(), Box<dyn Error>> {
    let reply = send_hello(stream, 1, 0)?;
    assert_eq!(reply.flags, nvide_ipc::RESP);
    Ok(())
}

fn assert_failed(mut child: std::process::Child) -> Result<(), Box<dyn Error>> {
    if child.wait()?.success() {
        Err("core accepted a malformed NRPC connection".into())
    } else {
        Ok(())
    }
}

fn test_endpoint(label: &str) -> Result<String, Box<dyn Error>> {
    let name = format!("nvide-core-test-{}-{label}", std::process::id());
    if cfg!(windows) {
        Ok(name)
    } else {
        Ok(std::env::temp_dir()
            .join(format!("{name}.sock"))
            .into_os_string()
            .into_string()
            .map_err(|_| "test IPC path is not UTF-8")?)
    }
}
