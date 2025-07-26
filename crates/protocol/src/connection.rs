use std::env;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use pod::Pod;

use crate::Error;
use crate::buf::Buf;
use crate::error::ErrorKind;
use crate::poll::Interest;
use crate::poll::Polled;
use crate::types::Header;

const ENVIRONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET: &str = "pipewire-0";

const VERSION: i32 = 3;
const MAX_SEND_SIZE: usize = 16;

impl AsRawFd for Connection {
    #[inline]
    fn as_raw_fd(&self) -> i32 {
        self.socket.as_raw_fd()
    }
}

/// A connection to a local pipewire server.
pub struct Connection {
    socket: UnixStream,
    seq: u32,
    send: Buf,
    header: Option<Header>,
    interest: Interest,
}

impl Connection {
    /// Open a connection to a local pipewire server.
    #[tracing::instrument]
    pub fn open() -> Result<Self, Error> {
        let socket = 'socket: {
            for environ in ENVIRONS.iter().copied() {
                let Some(path) = env::var_os(environ) else {
                    continue;
                };

                let mut path = PathBuf::from(path);
                path.push(SOCKET);

                match UnixStream::connect(&path) {
                    Ok(socket) => {
                        tracing::trace!("Connected to {}", path.display());
                        break 'socket socket;
                    }
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {
                        continue;
                    }
                    Err(e) => return Err(Error::new(ErrorKind::ConnectionFailed(e))),
                }
            }

            return Err(Error::new(ErrorKind::NoSocket));
        };

        Ok(Self {
            socket,
            seq: 0,
            send: Buf::new(),
            header: None,
            interest: Interest::READ,
        })
    }

    /// Set the connection to non-blocking mode.
    #[inline]
    pub fn set_nonblocking(&mut self, nonblocking: bool) -> Result<(), Error> {
        self.socket
            .set_nonblocking(nonblocking)
            .map_err(ErrorKind::SetNonBlockingFailed)?;
        Ok(())
    }

    /// Get the current interest for the connection.
    #[inline]
    pub fn interest(&self) -> Interest {
        self.interest
    }

    /// Send core hello.
    pub fn hello(&mut self) -> Result<Polled, Error> {
        let mut pod = Pod::array();
        let mut st = pod.as_mut().encode_struct()?;
        st.field()?.encode(VERSION)?;
        st.close()?;

        let buf = pod.as_buf();

        let Ok(size) = u32::try_from(buf.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        // SAFETY: No other borrows are live right now.
        self.send.write(Header::new(0, 1, size, self.seq, 0));
        self.send.extend_from_words(buf.as_slice());
        self.seq = self.seq.wrapping_add(1);
        Ok(self.interest.set(Interest::WRITE))
    }

    /// Send data to the server.
    ///
    /// If this method returns `true`, the interest for the connection has been
    /// changed and should be updated with the main loop.
    pub fn send(&mut self) -> Result<Polled, Error> {
        // Keep track of how much we've sent to limit the amount of time we
        // spend sending.
        let mut sent = MAX_SEND_SIZE;

        loop {
            if self.send.is_empty() {
                return Ok(self.interest.unset(Interest::WRITE));
            }

            let bytes = self.send.as_bytes();
            let bytes = bytes.get(..bytes.len().min(sent)).unwrap_or_default();

            match self.socket.write(bytes) {
                Ok(0) => {
                    return Err(Error::new(ErrorKind::RemoteClosed));
                }
                Ok(n) => {
                    debug_assert!(
                        n <= bytes.len(),
                        "Socket write returned more bytes than available in the buffer"
                    );

                    unsafe {
                        self.send.set_read(n);
                    }

                    sent -= n;

                    if sent == 0 {
                        return Ok(Polled::Unchanged);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(Polled::Unchanged);
                }
                Err(e) => {
                    return Err(Error::new(ErrorKind::SendFailed(e)));
                }
            }
        }
    }

    /// Receive data from the server.
    pub fn recv(&mut self, recv: &mut Buf) -> Result<Option<Header>, Error> {
        loop {
            // SAFETY: This is the only point which writes to the buffer, all
            // subsequent reads are aligned which only depends on the read cursor.
            let bytes = unsafe { recv.as_bytes_mut() };

            let result = self.socket.read(bytes);

            match result {
                Ok(0) => {
                    return Err(Error::new(ErrorKind::RemoteClosed));
                }
                Ok(n) => {
                    debug_assert!(
                        n <= bytes.len(),
                        "Socket read returned more bytes than available in the buffer"
                    );

                    // SAFETY: We trust the returned value `n` as the number of bytes
                    // read and therefore written into the buffer.
                    unsafe {
                        recv.set_written(n);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(None);
                }
                Err(e) => {
                    return Err(Error::new(ErrorKind::ReceiveFailed(e)));
                }
            };

            let Some(header) = &self.header else {
                self.header = recv.read::<Header>();
                continue;
            };

            let size = header.size() as usize;

            if size > recv.remaining() {
                continue;
            };

            let header = *header;
            self.header = None;
            return Ok(Some(header));
        }
    }
}
