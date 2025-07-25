use std::env;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use pod::Pod;

use crate::Error;
use crate::buf::Buf;
use crate::error::ErrorKind;
use crate::types::Header;

const ENVIRONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET: &str = "pipewire-0";

const VERSION: i32 = 3;

/// A connection to a local pipewire server.
pub struct Connection {
    socket: UnixStream,
    seq: u32,
    send: Buf,
    header: Option<Header>,
}

impl Connection {
    /// Open a connection to a local pipewire server.
    pub fn open() -> Result<Self, Error> {
        let socket = 'socket: {
            for environ in ENVIRONS.iter().copied() {
                let Some(path) = env::var_os(environ) else {
                    continue;
                };

                let mut path = PathBuf::from(path);
                path.push(SOCKET);

                match UnixStream::connect(&path) {
                    Ok(socket) => break 'socket socket,
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
        })
    }

    /// Send core hello.
    pub fn hello(&mut self) -> Result<(), Error> {
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

        let bytes = self.send.take_bytes();
        self.socket.write_all(bytes).map_err(ErrorKind::SendError)?;
        Ok(())
    }

    pub fn recv(&mut self, recv: &mut Buf) -> Result<Option<Header>, Error> {
        loop {
            // SAFETY: This is the only point which writes to the buffer, all
            // subsequent reads are aligned which only depends on the read cursor.
            let n = self
                .socket
                .read(unsafe { recv.as_bytes_mut() })
                .map_err(ErrorKind::RecvError)?;

            if n == 0 && recv.is_empty() {
                break;
            }

            // SAFETY: We trust the returned value `n` as the number of bytes
            // read and therefore written into the buffer.
            unsafe {
                recv.set_written(n);
            }

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

        Ok(None)
    }
}
