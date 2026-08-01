use nvide_buffer::{Buffer, CursorSnapshot, RopeBuffer};
use nvide_ipc::schema;
use std::{env, error::Error};

fn main() {
    if let Err(error) = run() {
        eprintln!("nvide-core: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let endpoint = endpoint_from_args().ok_or("missing --endpoint or NVIDE_NRPC_ENDPOINT")?;
    let stream = nvide_ipc::LocalStream::connect(&endpoint)?;
    let mut core = CoreState::default();
    nvide_ipc::serve(stream, move |request| core.apply(request))?;
    Ok(())
}

fn endpoint_from_args() -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--endpoint" {
            return args.next();
        }
    }
    env::var(nvide_platform_endpoint()).ok()
}

fn nvide_platform_endpoint() -> &'static str {
    "NVIDE_NRPC_ENDPOINT"
}

#[derive(Default)]
struct CoreState {
    buffer: RopeBuffer,
}

impl CoreState {
    fn apply(
        &mut self,
        request: schema::EditRequest,
    ) -> Result<schema::ViewportSnapshot, schema::RpcError> {
        let core_received_ns = nvide_platform_clock()?;
        if request.expected_version != self.buffer.version() {
            return Err(invalid_argument(format!(
                "stale buffer version {}; expected {}",
                request.expected_version,
                self.buffer.version()
            )));
        }
        let at = usize::try_from(request.char_offset)
            .map_err(|_| invalid_argument("character offset exceeds this platform".to_owned()))?;
        let after = at
            .checked_add(request.text.chars().count())
            .ok_or_else(|| invalid_argument("cursor offset overflow".to_owned()))?;
        let outcome = self
            .buffer
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
            .map_err(|error| invalid_argument(error.to_string()))?;
        let version_increment_ns = nvide_platform_clock()?;
        Ok(schema::ViewportSnapshot {
            trace_id: request.trace_id,
            version: outcome.version,
            text: self.buffer.text(),
            core_received_ns,
            version_increment_ns,
            viewport_emit_ns: nvide_platform_clock()?,
        })
    }
}

fn nvide_platform_clock() -> Result<u64, schema::RpcError> {
    nvide_ipc::platform_monotonic_ns().map_err(|error| schema::RpcError {
        code: schema::ErrorCode::Internal,
        message: error.to_string(),
    })
}

fn invalid_argument(message: String) -> schema::RpcError {
    schema::RpcError {
        code: schema::ErrorCode::InvalidArgument,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_increment_once_and_reject_stale_versions() -> Result<(), schema::RpcError> {
        let mut core = CoreState::default();
        let first = core.apply(schema::EditRequest {
            trace_id: 1,
            expected_version: 0,
            char_offset: 0,
            text: "a".to_owned(),
            dispatch_ns: 1,
        })?;
        assert_eq!((first.version, first.text.as_str()), (1, "a"));
        assert!(core
            .apply(schema::EditRequest {
                trace_id: 2,
                expected_version: 0,
                char_offset: 1,
                text: "b".to_owned(),
                dispatch_ns: 2,
            })
            .is_err());
        Ok(())
    }
}
