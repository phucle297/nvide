//! OS abstractions for local IPC endpoints.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Return a unique path suitable for a local IPC endpoint (Unix domain socket).
pub fn temp_ipc_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("nvide-{}-{}-{}.sock", prefix, std::process::id(), nanos);
    std::env::temp_dir().join(name)
}

/// Remove a socket path if it exists (best-effort).
pub fn remove_ipc_path(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(unix)]
pub mod unix_socket {
    use std::io;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;
    use std::time::Duration;

    use super::{remove_ipc_path, PlatformError};

    pub fn bind(path: &Path) -> Result<UnixListener, PlatformError> {
        remove_ipc_path(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(UnixListener::bind(path)?)
    }

    pub fn connect(path: &Path) -> Result<UnixStream, PlatformError> {
        Ok(UnixStream::connect(path)?)
    }

    pub fn connect_with_retry(
        path: &Path,
        attempts: u32,
        delay: Duration,
    ) -> Result<UnixStream, PlatformError> {
        let mut last = None;
        for _ in 0..attempts {
            match UnixStream::connect(path) {
                Ok(s) => return Ok(s),
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(delay);
                }
            }
        }
        Err(PlatformError::Io(last.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::ConnectionRefused, "connect retry exhausted")
        })))
    }
}

#[cfg(windows)]
pub mod windows_pipe {
    // Named pipes will land with Windows CI; Phase 0 prototype uses Unix sockets
    // on this host. The module exists so the crate compiles on Windows matrix.
}
