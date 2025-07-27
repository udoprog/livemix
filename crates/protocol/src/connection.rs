use core::mem;
use core::mem::MaybeUninit;
use core::ptr;
use std::env;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use pod::Pod;
use pod::Reader;

use crate::consts;
use crate::error::ErrorKind;
use crate::flags;
use crate::id;
use crate::op;
use crate::poll::{ChangeInterest, Interest};
use crate::types::Header;
use crate::{DynamicBuf, Error};

const ENVIRONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET: &str = "pipewire-0";

const VERSION: i32 = 3;
const MAX_SEND_SIZE: usize = 4096;

impl AsRawFd for Connection {
    #[inline]
    fn as_raw_fd(&self) -> i32 {
        self.socket.as_raw_fd()
    }
}

/// A connection to a local pipewire server.
pub struct Connection {
    socket: UnixStream,
    message_sequence: u32,
    sync_sequence: u32,
    outgoing: DynamicBuf,
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
            sync_sequence: 1,
            outgoing: DynamicBuf::new(),
            interest: Interest::READ,
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

    /// Send client hello.
    pub fn core_hello(&mut self) -> Result<(), Error> {
        let mut pod = Pod::array();
        pod.as_mut()
            .encode_struct(|st| st.field()?.encode(VERSION))?;

        self.request(consts::CORE_ID, op::CORE_HELLO, pod)?;
        Ok(())
    }

    /// Get registry.
    pub fn core_get_registry(&mut self, new_id: u32) -> Result<(), Error> {
        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(consts::REGISTRY_VERSION as i32)?;
            st.field()?.encode(new_id)?;
            Ok(())
        })?;

        self.request(consts::CORE_ID, op::CORE_GET_REGISTRY, pod)?;
        Ok(())
    }

    /// Synchronize.
    pub fn core_sync(&mut self, id: u32) -> Result<u32, Error> {
        let sync_sequence = self.sync_sequence;
        self.sync_sequence = self.sync_sequence.wrapping_add(1);

        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(id)?;
            st.field()?.encode(sync_sequence)?;
            Ok(())
        })?;

        self.request(consts::CORE_ID, op::CORE_SYNC, pod)?;
        Ok(sync_sequence)
    }

    /// Send a pong response to a ping.
    pub fn core_pong(&mut self, id: u32, seq: u32) -> Result<(), Error> {
        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(id)?;
            st.field()?.encode(seq)?;
            Ok(())
        })?;

        self.request(consts::CORE_ID, op::CORE_PONG, pod)?;
        Ok(())
    }

    /// Create an object.
    pub fn core_create_object(
        &mut self,
        factory_name: &str,
        ty: &str,
        version: u32,
        new_id: u32,
    ) -> Result<(), Error> {
        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode_unsized(factory_name)?;
            st.field()?.encode_unsized(ty)?;
            st.field()?.encode(version)?;

            st.field()?.encode_struct(|props| {
                props.field()?.encode(6)?;

                props.field()?.encode("node.description")?;
                props.field()?.encode("livemix")?;

                props.field()?.encode("node.name")?;
                props.field()?.encode("livemix")?;

                props.field()?.encode("media.class")?;
                props.field()?.encode("Audio/Duplex")?;

                props.field()?.encode("media.type")?;
                props.field()?.encode("Audio")?;

                props.field()?.encode("media.category")?;
                props.field()?.encode("Duplex")?;

                props.field()?.encode("media.role")?;
                props.field()?.encode("DSP")?;
                Ok(())
            })?;

            st.field()?.encode(new_id)?;
            Ok(())
        })?;

        self.request(consts::CORE_ID, op::CORE_CREATE_OBJECT, pod)?;
        Ok(())
    }

    /// Update client properties.
    pub fn client_update_properties(&mut self) -> Result<(), Error> {
        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode_struct(|st| {
                st.field()?.encode(2)?;

                st.field()?.encode("application.name")?;
                st.field()?.encode("livemix")?;

                st.field()?.encode("node.name")?;
                st.field()?.encode("livemix")?;
                Ok(())
            })
        })?;

        self.request(consts::CLIENT_ID, op::CLIENT_UPDATE_PROPERTIES, pod)?;
        Ok(())
    }

    /// Update the client.
    pub fn client_node_get_node(
        &mut self,
        id: u32,
        version: u32,
        new_id: u32,
    ) -> Result<(), Error> {
        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(version)?;
            st.field()?.encode(new_id)?;
            Ok(())
        })?;

        self.request(id, op::CLIENT_NODE_GET_NODE, pod)?;
        Ok(())
    }

    /// Update client node.
    pub fn client_node_update(&mut self, id: u32) -> Result<(), Error> {
        let mut pod = Pod::array();

        let mut change_mask = flags::ClientNodeUpdate::NONE;
        change_mask |= flags::ClientNodeUpdate::PARAMS;
        change_mask |= flags::ClientNodeUpdate::INFO;

        let max_input_ports = 2u32;
        let max_output_ports = 2u32;

        let mut node_change_mask = flags::NodeChangeMask::FLAGS;
        node_change_mask |= flags::NodeChangeMask::PROPS;
        node_change_mask |= flags::NodeChangeMask::PARAMS;

        let node_flags = flags::Node::IN_DYNAMIC_PORTS | flags::Node::OUT_DYNAMIC_PORTS;

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(change_mask)?;

            st.field()?.encode(2)?;

            st.field()?
                .encode_object(id::ObjectType::FORMAT, id::Param::ENUM_FORMAT, |obj| {
                    obj.property(id::Format::MEDIA_TYPE, 0)?
                        .encode(id::MediaType::AUDIO)?;
                    obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                        .encode(id::MediaSubType::RAW)?;
                    obj.property(id::Format::AUDIO_FORMAT, 0)?
                        .encode(id::AudioFormat::S16)?;
                    obj.property(id::Format::AUDIO_CHANNELS, 0)?.encode(1u32)?;
                    obj.property(id::Format::AUDIO_RATE, 0)?.encode(44100u32)?;
                    Ok(())
                })?;

            st.field()?
                .encode_object(id::ObjectType::FORMAT, id::Param::FORMAT, |obj| {
                    obj.property(id::Format::MEDIA_TYPE, 0)?
                        .encode(id::MediaType::AUDIO)?;
                    obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                        .encode(id::MediaSubType::RAW)?;
                    obj.property(id::Format::AUDIO_FORMAT, 0)?
                        .encode(id::AudioFormat::S16)?;
                    obj.property(id::Format::AUDIO_CHANNELS, 0)?.encode(1u32)?;
                    obj.property(id::Format::AUDIO_RATE, 0)?.encode(44100u32)?;
                    Ok(())
                })?;

            if change_mask & flags::ClientNodeUpdate::INFO {
                st.field()?.encode_struct(|st| {
                    st.field()?.encode(max_input_ports)?;
                    st.field()?.encode(max_output_ports)?;
                    st.field()?.encode(node_change_mask)?;
                    st.field()?.encode(node_flags)?;

                    st.field()?.encode(1u32)?;
                    st.field()?.encode("node.name")?;
                    st.field()?.encode_unsized("livemix2")?;

                    st.field()?.encode(4u32)?;
                    st.field()?.encode(id::Param::PROP_INFO)?;
                    st.field()?.encode(flags::Param::NONE)?;

                    st.field()?.encode(id::Param::PROPS)?;
                    st.field()?.encode(flags::Param::WRITE)?;

                    st.field()?.encode(id::Param::ENUM_FORMAT)?;
                    st.field()?.encode(flags::Param::READ)?;

                    st.field()?.encode(id::Param::FORMAT)?;
                    st.field()?.encode(flags::Param::WRITE)?;
                    Ok(())
                })?;
            } else {
                st.field()?.encode_none()?;
            }

            Ok(())
        })?;

        self.request(id, op::CLIENT_NODE_UPDATE, pod)?;
        Ok(())
    }

    /// Update client node port.
    pub fn client_node_port_update(
        &mut self,
        id: u32,
        direction: consts::Direction,
        port_id: u32,
    ) -> Result<(), Error> {
        let mut pod = Pod::array();

        let mut change_mask = flags::ClientNodePortUpdate::NONE;
        change_mask |= flags::ClientNodePortUpdate::PARAMS;
        change_mask |= flags::ClientNodePortUpdate::INFO;

        let mut port_change_mask = flags::PortChangeMask::NONE;
        port_change_mask |= flags::PortChangeMask::FLAGS;
        port_change_mask |= flags::PortChangeMask::PROPS;
        port_change_mask |= flags::PortChangeMask::PARAMS;

        let port_flags = flags::Port::NONE;

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(direction as u32)?;
            st.field()?.encode(port_id)?;
            st.field()?.encode(change_mask)?;

            // Parameters.
            st.field()?.encode(2u32)?;

            st.field()?
                .encode_object(id::ObjectType::FORMAT, id::Param::ENUM_FORMAT, |obj| {
                    obj.property(id::Format::MEDIA_TYPE, 0)?
                        .encode(id::MediaType::AUDIO)?;
                    obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                        .encode(id::MediaSubType::RAW)?;
                    obj.property(id::Format::AUDIO_FORMAT, 0)?
                        .encode(id::AudioFormat::S16)?;
                    obj.property(id::Format::AUDIO_CHANNELS, 0)?.encode(1u32)?;
                    obj.property(id::Format::AUDIO_RATE, 0)?.encode(44100u32)?;
                    Ok(())
                })?;

            st.field()?
                .encode_object(id::ObjectType::FORMAT, id::Param::FORMAT, |obj| {
                    obj.property(id::Format::MEDIA_TYPE, 0)?
                        .encode(id::MediaType::AUDIO)?;
                    obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                        .encode(id::MediaSubType::RAW)?;
                    obj.property(id::Format::AUDIO_FORMAT, 0)?
                        .encode(id::AudioFormat::S16)?;
                    obj.property(id::Format::AUDIO_CHANNELS, 0)?.encode(1u32)?;
                    obj.property(id::Format::AUDIO_RATE, 0)?.encode(44100u32)?;
                    Ok(())
                })?;

            if change_mask & flags::ClientNodePortUpdate::INFO {
                st.field()?.encode_struct(|st| {
                    st.field()?.encode(port_change_mask)?;
                    st.field()?.encode(port_flags)?;

                    // Rate num / denom
                    st.field()?.encode(0u32)?;
                    st.field()?.encode(0u32)?;

                    // Properties.
                    st.field()?.encode(2u32)?;
                    st.field()?.encode("port.name")?;
                    st.field()?.encode_unsized("livemix_port0")?;

                    st.field()?.encode("format.dsp")?;
                    st.field()?.encode_unsized("32 bit float mono audio")?;

                    // Parameters.
                    st.field()?.encode(2u32)?;
                    st.field()?.encode(id::Param::ENUM_FORMAT)?;
                    st.field()?.encode(flags::Param::READ)?;

                    st.field()?.encode(id::Param::FORMAT)?;
                    st.field()?.encode(flags::Param::WRITE)?;

                    /*
                    st.field()?.encode(id::Param::META)?;
                    st.field()?.encode(flags::Param::READ)?;

                    st.field()?.encode(id::Param::IO)?;
                    st.field()?.encode(flags::Param::READ)?;

                    st.field()?.encode(id::Param::BUFFERS)?;
                    st.field()?.encode(flags::Param::NONE)?;
                    */

                    Ok(())
                })?;
            } else {
                st.field()?.encode_none()?;
            }

            Ok(())
        })?;

        self.request(id, op::CLIENT_NODE_PORT_UPDATE, pod)?;
        Ok(())
    }

    /// Update the client.
    pub fn client_node_set_active(&mut self, id: u32, active: bool) -> Result<(), Error> {
        let mut pod = Pod::array();

        pod.as_mut().encode_struct(|st| {
            st.field()?.encode(active)?;
            Ok(())
        })?;

        self.request(id, op::CLIENT_NODE_SET_ACTIVE, pod)?;
        Ok(())
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
                        self.outgoing.advance_read(n);
                    }

                    tracing::trace!(bytes = n, remaining = self.outgoing.remaining(), "sent");
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

    /// Receive data from the server.
    pub fn recv(&mut self, recv: &mut DynamicBuf) -> Result<(), Error> {
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
                        recv.advance_written(n);
                    }

                    tracing::trace!(bytes = n, remaining = recv.remaining(), "received");
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(Error::new(ErrorKind::ReceiveFailed(e)));
                }
            };
        }
    }

    /// Receive file descriptors from the server.
    pub fn recv_with_fds(
        &mut self,
        recv: &mut DynamicBuf,
        fds: &mut [RawFd],
    ) -> Result<usize, Error> {
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
                let bytes = recv.as_bytes_mut();

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
                recv.advance_written(n);

                tracing::trace!(bytes = n, remaining = recv.remaining(), "received");

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

    /// Serialize an outgoing message.
    fn request<'de>(
        &mut self,
        id: u32,
        op: u8,
        pod: Pod<impl Reader<'de, u64>>,
    ) -> Result<(), Error> {
        let buf = pod.as_buf();

        let Ok(size) = u32::try_from(buf.remaining_bytes()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Some(header) = Header::new(id, op, size, self.message_sequence, 0) else {
            return Err(Error::new(ErrorKind::HeaderSizeOverflow { size }));
        };

        let remaining_before = self.outgoing.remaining();

        self.outgoing.write(header);
        self.outgoing.extend_from_words(buf.as_slice());
        self.message_sequence = self.message_sequence.wrapping_add(1);
        self.modified |= self.interest.set(Interest::WRITE);

        tracing::trace!(?header, ?remaining_before, remaining = ?self.outgoing.remaining(), "Sending");
        Ok(())
    }
}
