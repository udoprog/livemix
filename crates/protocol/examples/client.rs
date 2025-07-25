use protocol::{Buf, Connection, Error};

fn main() -> Result<(), Error> {
    let mut recv = Buf::new();
    let mut c = Connection::open()?;

    c.hello()?;

    while let Some(header) = c.recv(&mut recv)? {
        let Some(frame) = recv.frame(header) else {
            break;
        };

        println!("{:?}", frame.header);
        println!("{:?}", frame.pod);
    }

    Ok(())
}
