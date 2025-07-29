use std::mem;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;

use anyhow::{Context, Result};
use protocol::poll::{ChangeInterest, Interest, PollEvent, Token};
use protocol::{Connection, DynamicBuf, EventFd, Poll};

const CONNECTION: Token = Token::new(100);
const EVENT: Token = Token::new(200);

fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().map_err(anyhow::Error::msg)?;

    let ev = Arc::new(EventFd::new(0)?);
    let mut poll = Poll::new()?;
    let mut c = Connection::open()?;

    c.set_nonblocking(true)?;

    let mut recv = DynamicBuf::new();

    poll.add(c.as_raw_fd(), CONNECTION, c.interest())?;
    poll.add(ev.as_raw_fd(), EVENT, Interest::READ)?;

    let mut events = pod::Buf::<PollEvent, 4>::new();
    let mut state = client::State::new(c);

    let mut fds = [0; 16];

    loop {
        state.run(&mut recv)?;

        if let ChangeInterest::Changed(interest) = state.connection_mut().modified() {
            poll.modify(state.connection().as_raw_fd(), CONNECTION, interest)?;
        }

        while let Some((fd, token, interest)) = state.add_interest() {
            poll.add(fd, token, interest)?;
        }

        while let Some((fd, token, interest)) = state.modify_interest() {
            poll.modify(fd, token, interest)?;
        }

        poll.poll(&mut events)?;

        while let Some(e) = events.pop() {
            match e.token {
                CONNECTION => {
                    if e.interest.is_read() {
                        let n_fds = state
                            .connection_mut()
                            .recv_with_fds(&mut recv, &mut fds[..])
                            .context("Failed to receive file descriptors")?;

                        // SAFETY: We must trust the file descriptor we have
                        // just received.
                        let iter = fds[..n_fds]
                            .iter_mut()
                            .map(|fd| unsafe { OwnedFd::from_raw_fd(mem::take(fd)) });

                        state.add_fds(iter);
                    }

                    if e.interest.is_write() {
                        state.connection_mut().send()?;
                    }
                }
                EVENT => {
                    if let Some(value) = ev.read()? {
                        println!("Event: {value}");
                    }
                }
                token => {
                    if e.interest.is_read() {
                        state.handle_read(token)?;
                    }
                }
            }
        }
    }
}
