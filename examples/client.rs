use std::mem;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;

use anyhow::{Context, Result};
use pod::buf::ArrayVec;
use protocol::buf::RecvBuf;
use protocol::poll::{ChangeInterest, Interest, PollEvent};
use protocol::{Connection, Poll, TimerFd};

fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().map_err(anyhow::Error::msg)?;

    let mut poll = Poll::new()?;
    let mut c = Connection::open()?;
    c.set_nonblocking(true)?;

    let timer = TimerFd::new()?;
    timer.set_nonblocking(true)?;

    let mut recv = RecvBuf::new();
    let mut state = client::State::new(c)?;

    let conn_token = state.token()?;
    let timer_token = state.token()?;

    poll.add(
        state.connection().as_raw_fd(),
        conn_token,
        state.connection().interest(),
    )?;
    poll.add(timer.as_raw_fd(), timer_token, Interest::READ)?;

    timer.set_interval(Duration::from_secs(10))?;

    let mut events = ArrayVec::<PollEvent, 4>::new();

    loop {
        state.run(&mut recv)?;

        if let ChangeInterest::Changed(interest) = state.connection_mut().modified() {
            poll.modify(state.connection().as_raw_fd(), conn_token, interest)?;
        }

        while let Some((fd, token, interest)) = state.add_interest() {
            poll.add(fd, token, interest)?;
        }

        while let Some((fd, token, interest)) = state.modify_interest() {
            poll.modify(fd, token, interest)?;
        }

        poll.poll(&mut events)?;

        while let Some(e) = events.pop() {
            if e.token == conn_token {
                tracing::trace!(?e.interest, "connection");

                if e.interest.is_read() {
                    let mut fds = [0; 16];

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

                continue;
            }

            if e.token == timer_token {
                if e.interest.is_read() {
                    if matches!(timer.read()?, Some(v) if v > 0) {
                        state.tick()?;
                    }
                }

                continue;
            }

            if e.interest.is_read() {
                state.handle_read(e.token)?;
            }
        }
    }
}
