use core::mem;
use core::mem::MaybeUninit;
use core::ptr;

use std::env;
use std::io;
use std::io::Write;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use pod::{AsReader, Pod, Reader};
use tracing::Level;

use crate::Error;
use crate::buf::{RecvBuf, SendBuf};
use crate::error::ErrorKind;
use crate::poll::{ChangeInterest, Interest};
use crate::types::Header;

const ENVIRONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET: &str = "pipewire-0";

const MAX_SEND_SIZE: usize = 4096;

impl AsRawFd for Connection {
    #[inline]
    fn as_raw_fd(&self) -> i32 {
        self.socket.as_raw_fd()
    }
}

/// A connection to a local pipewire server.
#[derive(Debug)]
pub struct Connection {
    socket: UnixStream,
    message_sequence: u32,
    outgoing: SendBuf,
    interest: Interest,
    modified: ChangeInterest,
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
            message_sequence: 0,
            outgoing: SendBuf::new(),
            interest: Interest::READ | Interest::HUP | Interest::ERROR,
            modified: ChangeInterest::Unchanged,
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

    /// Return modified interest, if any.
    #[inline]
    pub fn modified(&mut self) -> ChangeInterest {
        self.modified.take()
    }

    /// Send data to the server.
    ///
    /// If this method returns `true`, the interest for the connection has been
    /// changed and should be updated with the main loop.
    pub fn send(&mut self) -> Result<(), Error> {
        // Keep track of how much we've sent to limit the amount of time we
        // spend sending.
        let mut sent = MAX_SEND_SIZE;

        loop {
            if self.outgoing.is_empty() {
                self.modified |= self.interest.unset(Interest::WRITE);
                return Ok(());
            }

            let bytes = self.outgoing.as_bytes();
            let bytes = bytes.get(..bytes.len().min(sent)).unwrap_or_default();
            let remaining_before = bytes.len();

            match self.socket.write(bytes) {
                Ok(0) => {
                    return Err(Error::new(ErrorKind::RemoteClosed));
                }
                Ok(n) => {
                    debug_assert!(
                        n <= bytes.len(),
                        "Socket write returned more bytes than available in the buffer"
                    );

                    // SAFETY: We trust the returned value `n` as the number of
                    // bytes read constained by the number of bytes available.
                    unsafe {
                        self.outgoing.advance_read_bytes(n);
                    }

                    let remaining = self.outgoing.remaining_bytes();

                    tracing::trace!(bytes = n, remaining_before, remaining, "sent");

                    sent -= n;

                    if sent == 0 {
                        return Ok(());
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(Error::new(ErrorKind::SendFailed(e)));
                }
            }
        }
    }

    /// Receive file descriptors from the server.
    pub fn recv_with_fds(&mut self, recv: &mut RecvBuf, fds: &mut [RawFd]) -> Result<usize, Error> {
        const {
            assert!(mem::align_of::<MaybeUninit<[u64; 16]>>() >= mem::align_of::<libc::cmsghdr>());
        }

        let fd_len = mem::size_of::<RawFd>() * fds.len();
        let size = unsafe { libc::CMSG_SPACE(fd_len as u32) as usize };

        let mut buf = MaybeUninit::<[u64; 16]>::uninit();
        assert!(mem::size_of_val(&buf) >= size);

        let mut iov = libc::iovec {
            iov_base: ptr::null_mut(),
            iov_len: 0,
        };

        let mut msghdr = unsafe { mem::zeroed::<libc::msghdr>() };

        loop {
            unsafe {
                // SAFETY: This is the only point which writes to the buffer, all
                // subsequent reads are aligned which only depends on the read cursor.
                let remaining_before = recv.remaining_bytes();
                let bytes = recv.as_bytes_mut()?;

                iov.iov_base = bytes.as_mut_ptr().cast();
                iov.iov_len = bytes.len();

                msghdr.msg_name = ptr::null_mut();
                msghdr.msg_namelen = 0;
                msghdr.msg_iov = &mut iov;
                msghdr.msg_iovlen = 1;
                msghdr.msg_control = &mut buf as *mut _ as *mut libc::c_void;
                msghdr.msg_controllen = size;

                let n = libc::recvmsg(self.socket.as_raw_fd(), &mut msghdr as *mut _, 0);

                if n < 0 {
                    match io::Error::last_os_error() {
                        e if e.kind() == io::ErrorKind::WouldBlock => {
                            return Ok(0);
                        }
                        e => {
                            return Err(Error::new(ErrorKind::ReceiveFailed(e)));
                        }
                    }
                }

                let n = n as usize;

                debug_assert!(
                    n <= bytes.len(),
                    "Socket read returned more bytes than available in the buffer"
                );

                // SAFETY: We trust the returned value `n` as the number of bytes
                // read and therefore written into the buffer.
                recv.advance_written_bytes(n);

                tracing::trace!(
                    bytes = n,
                    remaining_before,
                    remaining = recv.remaining_bytes(),
                    "received"
                );

                // Walk the ancillary data buffer and copy the raw descriptors
                // from it into the output buffer.
                let mut n_fds = 0usize;
                let mut cur = libc::CMSG_FIRSTHDR(&mut msghdr as *mut _);

                while let Some(c) = cur.as_ref() {
                    if c.cmsg_level == libc::SOL_SOCKET && c.cmsg_type == libc::SCM_RIGHTS {
                        let data_ptr = libc::CMSG_DATA(c);
                        let data_offset = data_ptr.offset_from((c as *const libc::cmsghdr).cast());

                        debug_assert!(data_offset >= 0);

                        let data_byte_count = c.cmsg_len as usize - data_offset as usize;

                        debug_assert!(c.cmsg_len as isize >= data_offset);
                        debug_assert!(data_byte_count % mem::size_of::<RawFd>() == 0);

                        let rawfd_count = (data_byte_count / mem::size_of::<RawFd>()) as usize;
                        let fd_ptr = data_ptr.cast::<RawFd>();

                        for i in 0..rawfd_count {
                            fds[n_fds] = ptr::read_unaligned(fd_ptr.add(i));
                            n_fds += 1;
                        }
                    }

                    cur = libc::CMSG_NXTHDR(&mut msghdr as *mut _, cur);
                }

                if n_fds > 0 {
                    return Ok(n_fds);
                }

                if n == 0 {
                    return Err(Error::new(ErrorKind::RemoteClosed));
                }
            }
        }
    }

    /// Send an outgoing request.
    ///
    /// This will write the request to the outgoing buffer.
    #[tracing::instrument(skip(self, pod), fields(remaining = self.outgoing.len()), ret(level = Level::DEBUG))]
    pub fn request(&mut self, id: u32, op: u8, pod: Pod<impl AsReader>) -> Result<(), Error> {
        let pod = pod.as_ref();
        let buf = pod.as_buf();

        let Ok(size) = u32::try_from(buf.bytes_len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let message_sequence = self.message_sequence;
        self.message_sequence = self.message_sequence.wrapping_add(1);

        let Some(header) = Header::new(id, op, size, message_sequence, 0) else {
            return Err(Error::new(ErrorKind::HeaderSizeOverflow { size }));
        };

        self.outgoing.push_bytes(&header)?;
        self.outgoing.extend_from_words(buf.as_slice())?;
        self.modified |= self.interest.set(Interest::WRITE);
        Ok(())
    }
}
