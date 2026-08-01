//! Local process and IPC platform primitives.

pub const NRPC_ENDPOINT_ENV: &str = "NVIDE_NRPC_ENDPOINT";

#[cfg(unix)]
#[allow(unsafe_code)]
mod local {
    use std::{
        fs,
        io::{self, Read, Write},
        os::unix::net::{UnixListener, UnixStream},
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };

    pub struct LocalListener {
        inner: UnixListener,
        path: PathBuf,
    }

    impl LocalListener {
        pub fn bind(endpoint: &str) -> io::Result<Self> {
            let path = Path::new(endpoint);
            let inner = UnixListener::bind(path)?;
            inner.set_nonblocking(true)?;
            Ok(Self {
                inner,
                path: path.to_owned(),
            })
        }

        pub fn accept(&self) -> io::Result<LocalStream> {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match self.inner.accept() {
                    Ok((stream, _)) => return LocalStream::new(stream),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            return Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "timed out accepting local socket",
                            ));
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }
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

    pub fn monotonic_ns() -> io::Result<u64> {
        let mut value = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: value points to writable timespec storage for the duration of the call.
        if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let seconds = u64::try_from(value.tv_sec)
            .map_err(|_| io::Error::other("negative monotonic clock value"))?;
        let nanoseconds = u64::try_from(value.tv_nsec)
            .map_err(|_| io::Error::other("negative monotonic clock value"))?;
        seconds
            .checked_mul(1_000_000_000)
            .and_then(|total| total.checked_add(nanoseconds))
            .ok_or_else(|| io::Error::other("monotonic clock overflow"))
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
            CloseHandle, GetLastError, ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND, ERROR_NO_DATA,
            ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, HANDLE,
            INVALID_HANDLE_VALUE,
        },
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
            OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        },
        System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
        System::Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, SetNamedPipeHandleState, PIPE_NOWAIT,
            PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
            PIPE_UNLIMITED_INSTANCES,
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
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
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
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                // SAFETY: handle is a live server pipe and the overlapped pointer is null.
                if unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } != 0 {
                    break;
                }
                // SAFETY: GetLastError reads thread-local state immediately after ConnectNamedPipe.
                let code = unsafe { GetLastError() };
                if code == ERROR_PIPE_CONNECTED {
                    break;
                }
                if code != ERROR_PIPE_LISTENING && code != ERROR_NO_DATA {
                    // SAFETY: handle is live and has not been transferred.
                    unsafe { CloseHandle(handle) };
                    return Err(io::Error::from_raw_os_error(code as i32));
                }
                if Instant::now() >= deadline {
                    // SAFETY: handle is live and has not been transferred.
                    unsafe { CloseHandle(handle) };
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out accepting local named pipe",
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
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
                    if let Err(error) = set_nonblocking(handle) {
                        // SAFETY: handle is live and has not been transferred.
                        unsafe { CloseHandle(handle) };
                        return Err(error);
                    }
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

    fn set_nonblocking(handle: HANDLE) -> io::Result<()> {
        let mode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
        // SAFETY: handle is a connected named pipe; mode points to a live value.
        if unsafe { SetNamedPipeHandleState(handle, &mode, std::ptr::null(), std::ptr::null()) }
            == 0
        {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    impl Read for LocalStream {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match self.0.read(bytes) {
                    Ok(read) => return Ok(read),
                    Err(error) if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => {
                        return Ok(0)
                    }
                    Err(error) if is_pending(&error) => {
                        if Instant::now() >= deadline {
                            return Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "local named-pipe read timed out",
                            ));
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }

    impl Write for LocalStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.is_empty() {
                return Ok(0);
            }
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match self.0.write(bytes) {
                    Ok(0) => {}
                    Ok(written) => return Ok(written),
                    Err(error) if is_pending(&error) => {}
                    Err(error) => return Err(error),
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "local named-pipe write timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    fn is_pending(error: &io::Error) -> bool {
        matches!(
            error.raw_os_error(),
            Some(code) if code == ERROR_NO_DATA as i32 || code == ERROR_PIPE_BUSY as i32
        )
    }

    pub fn monotonic_ns() -> io::Result<u64> {
        let mut counter = 0_i64;
        let mut frequency = 0_i64;
        // SAFETY: both pointers reference writable i64 storage for these calls.
        if unsafe { QueryPerformanceCounter(&mut counter) } == 0
            || unsafe { QueryPerformanceFrequency(&mut frequency) } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if counter < 0 || frequency <= 0 {
            return Err(io::Error::other("invalid performance counter"));
        }
        let nanoseconds = (counter as u128)
            .checked_mul(1_000_000_000)
            .ok_or_else(|| io::Error::other("monotonic clock overflow"))?
            / frequency as u128;
        u64::try_from(nanoseconds).map_err(|_| io::Error::other("monotonic clock overflow"))
    }
}

pub use local::{monotonic_ns, LocalListener, LocalStream};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[cfg(unix)]
    #[test]
    fn local_stream_pair_roundtrips() -> std::io::Result<()> {
        let (mut left, mut right) = LocalStream::pair()?;
        left.write_all(b"nrpc")?;
        let mut bytes = [0; 4];
        right.read_exact(&mut bytes)?;
        assert_eq!(&bytes, b"nrpc");
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn named_pipe_read_times_out() -> std::io::Result<()> {
        let endpoint = format!("nvide-platform-read-timeout-{}", std::process::id());
        let listener = LocalListener::bind(&endpoint)?;
        let reader = std::thread::spawn(move || -> std::io::Result<()> {
            let mut stream = listener.accept()?;
            let mut byte = [0];
            let error = stream.read(&mut byte).expect_err("idle read must time out");
            assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
            Ok(())
        });
        let _client = LocalStream::connect(&endpoint)?;
        reader
            .join()
            .map_err(|_| std::io::Error::other("reader thread panicked"))??;
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn named_pipe_write_times_out() -> std::io::Result<()> {
        let endpoint = format!("nvide-platform-write-timeout-{}", std::process::id());
        let listener = LocalListener::bind(&endpoint)?;
        let idle = std::thread::spawn(move || -> std::io::Result<()> {
            let _stream = listener.accept()?;
            std::thread::sleep(std::time::Duration::from_secs(6));
            Ok(())
        });
        let mut client = LocalStream::connect(&endpoint)?;
        let payload = vec![0; 1024 * 1024];
        let error = client
            .write_all(&payload)
            .expect_err("stalled write must time out");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        idle.join()
            .map_err(|_| std::io::Error::other("writer thread panicked"))??;
        Ok(())
    }
}
