fn main() {
    if let Err(error) = run() {
        eprintln!("nvide: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--phase0-core") {
        return run_phase0_core();
    }

    #[cfg(feature = "xtask")]
    if std::env::args()
        .nth(1)
        .as_deref()
        .is_some_and(|arg| arg == "schema" || arg == "evidence")
    {
        return nvide_rpc_schema::xtask::run(std::env::args().skip(1));
    }

    nvide_ui::run().map_err(Into::into)
}

fn run_phase0_core() -> Result<(), Box<dyn std::error::Error>> {
    use nvide_buffer::{Buffer, CursorSnapshot, RopeBuffer};
    use nvide_ipc::schema;

    let mut args = std::env::args().skip(2);
    let mut endpoint = None;
    while let Some(argument) = args.next() {
        if argument == "--endpoint" {
            endpoint = args.next();
            break;
        }
    }
    let endpoint = endpoint.ok_or("missing Phase 0 core endpoint")?;
    let stream = nvide_ipc::LocalStream::connect(&endpoint)?;
    let mut buffer = RopeBuffer::default();
    nvide_ipc::serve(stream, move |request| {
        if request.expected_version != buffer.version() {
            return Err(rpc_invalid(format!(
                "stale buffer version {}; expected {}",
                request.expected_version,
                buffer.version()
            )));
        }
        let at = usize::try_from(request.char_offset)
            .map_err(|_| rpc_invalid("character offset exceeds this platform".to_owned()))?;
        let after = at
            .checked_add(request.text.chars().count())
            .ok_or_else(|| rpc_invalid("cursor offset overflow".to_owned()))?;
        let outcome = buffer
            .insert(
                at,
                request.text,
                CursorSnapshot {
                    anchor: at,
                    head: at,
                },
                CursorSnapshot {
                    anchor: after,
                    head: after,
                },
            )
            .map_err(|error| rpc_invalid(error.to_string()))?;
        Ok(schema::ViewportSnapshot {
            trace_id: request.trace_id,
            version: outcome.version,
            text: buffer.text(),
        })
    })?;
    Ok(())
}

fn rpc_invalid(message: String) -> nvide_ipc::schema::RpcError {
    nvide_ipc::schema::RpcError {
        code: nvide_ipc::schema::ErrorCode::InvalidArgument,
        message,
    }
}
