use protocol::poll::{Interest, Polled, Token};
use protocol::{Buf, Connection, EventFd, Poll};

use anyhow::Result;

const CONNECTION: Token = Token::new(100);
const EVENT: Token = Token::new(200);

fn main() -> Result<()> {
    let ev = EventFd::new(0)?;

    let mut poll = Poll::new()?;
    let mut c = Connection::open()?;

    c.set_nonblocking(true)?;

    let mut recv = Buf::new();

    poll.add(&c, CONNECTION, c.interest())?;
    poll.add(&ev, EVENT, Interest::READ)?;

    if let Polled::Changed(interest) = c.hello()? {
        println!("New connection interest: {interest:?}");
        poll.modify(&c, CONNECTION, interest)?;
    }

    let ev2 = ev.clone();

    std::thread::spawn(move || {
        let mut value = 1;

        loop {
            // Simulate an event after some time.
            std::thread::sleep(std::time::Duration::from_secs(1));
            ev2.write(value).unwrap();
            value += 1;
        }
    });

    let mut events = Vec::new();

    loop {
        poll.poll(&mut events)?;

        for e in events.drain(..) {
            match e.token {
                CONNECTION => {
                    if e.interest.is_read() {
                        if let Some(header) = c.recv(&mut recv)? {
                            let Some(frame) = recv.frame(header) else {
                                break;
                            };

                            println!("{:?}", frame.header);
                            println!("{:?}", frame.pod);
                        }
                    }

                    if e.interest.is_write() {
                        if let Polled::Changed(interest) = c.send()? {
                            poll.modify(&c, CONNECTION, interest)?;
                        }
                    }
                }
                EVENT => {
                    if let Some(value) = ev.read()? {
                        println!("Event: {value}");
                    }
                }
                other => {
                    println!("Unknown token: {other:?}");
                }
            }
        }
    }
}
