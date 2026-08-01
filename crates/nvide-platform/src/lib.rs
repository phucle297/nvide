//! Local process and IPC platform primitives.

pub const NRPC_ENDPOINT_ENV: &str = "NVIDE_NRPC_ENDPOINT";

#[cfg(unix)]
mod local {
    use std::{
        fs,
        io::{self, Read, Write},
        os::unix::net::{UnixListener, UnixStream},
        path::{Path, PathBuf},
        time::Duration,
    };

    pub struct LocalListener {
        inner: UnixListener,
        path: PathBuf,
    }

    impl LocalListener {
        pub fn bind(endpoint: &str) -> io::Result<Self> {
            let path = Path::new(endpoint);
            let inner = UnixListener::bind(path)?;
            Ok(Self {
                inner,
                path: path.to_owned(),
            })
        }

        pub fn accept(&self) -> io::Result<LocalStream> {
            let (stream, _) = self.inner.accept()?;
            LocalStream::new(stream)
        }
    }

    impl Drop for LocalListener {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    pub struct LocalStream(UnixStream);

    impl LocalStream {
        pub fn connect(endpoint: &str) -> io::Result<Self> {
            Self::new(UnixStream::connect(endpoint)?)
        }

        pub fn pair() -> io::Result<(Self, Self)> {
            let (left, right) = UnixStream::pair()?;
            Ok((Self::new(left)?, Self::new(right)?))
        }

        fn new(stream: UnixStream) -> io::Result<Self> {
            let timeout = Some(Duration::from_secs(5));
            stream.set_read_timeout(timeout)?;
            stream.set_write_timeout(timeout)?;
            Ok(Self(stream))
        }
    }

    impl Read for LocalStream {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.0.read(bytes)
        }
    }

    impl Write for LocalStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod local {
    use std::{
        fs::File,
        io::{self, Read, Write},
        os::windows::io::{FromRawHandle, RawHandle},
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
            HANDLE, INVALID_HANDLE_VALUE,
        },
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
            OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        },
        System::Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
            PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
        },
    };

    pub struct LocalListener {
        name: Vec<u16>,
    }

    impl LocalListener {
        pub fn bind(endpoint: &str) -> io::Result<Self> {
            Ok(Self {
                name: pipe_name(endpoint)?,
            })
        }

        pub fn accept(&self) -> io::Result<LocalStream> {
            // SAFETY: the UTF-16 name is NUL-terminated; returned ownership is transferred to File.
            let handle = unsafe {
                CreateNamedPipeW(
                    self.name.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    64 * 1024,
                    64 * 1024,
                    5_000,
                    std::ptr::null(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: handle is a live server pipe and the overlapped pointer is intentionally null.
            let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) };
            // SAFETY: GetLastError reads thread-local OS state after ConnectNamedPipe.
            if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
                // SAFETY: handle is live and has not been transferred.
                unsafe { CloseHandle(handle) };
                return Err(io::Error::last_os_error());
            }
            Ok(LocalStream(file_from_handle(handle)))
        }
    }

    pub struct LocalStream(File);

    impl LocalStream {
        pub fn connect(endpoint: &str) -> io::Result<Self> {
            let name = pipe_name(endpoint)?;
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                // SAFETY: name is NUL-terminated and all optional pointers are null.
                let handle = unsafe {
                    CreateFileW(
                        name.as_ptr(),
                        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                        0,
                        std::ptr::null(),
                        OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL,
                        0,
                    )
                };
                if handle != INVALID_HANDLE_VALUE {
                    return Ok(Self(file_from_handle(handle)));
                }
                // SAFETY: GetLastError reads thread-local OS state immediately after CreateFileW.
                let code = unsafe { GetLastError() };
                if code != ERROR_FILE_NOT_FOUND && code != ERROR_PIPE_BUSY {
                    return Err(io::Error::from_raw_os_error(code as i32));
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out connecting to local named pipe",
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    fn pipe_name(endpoint: &str) -> io::Result<Vec<u16>> {
        if endpoint.is_empty() || endpoint.encode_utf16().any(|unit| unit == 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid pipe name",
            ));
        }
        Ok(format!(r"\\.\pipe\{endpoint}")
            .encode_utf16()
            .chain([0])
            .collect())
    }

    fn file_from_handle(handle: HANDLE) -> File {
        // SAFETY: the handle is uniquely owned here and File closes it exactly once.
        unsafe { File::from_raw_handle(handle as RawHandle) }
    }

    impl Read for LocalStream {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.0.read(bytes)
        }
    }

    impl Write for LocalStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}

pub use local::{LocalListener, LocalStream};

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn local_stream_pair_roundtrips() -> std::io::Result<()> {
        let (mut left, mut right) = LocalStream::pair()?;
        left.write_all(b"nrpc")?;
        let mut bytes = [0; 4];
        right.read_exact(&mut bytes)?;
        assert_eq!(&bytes, b"nrpc");
        Ok(())
    }
}
