use pod::{Error, Readable, Writable};

macro_rules! roundtrip {
    ($ty:ident $($tt:tt)*) => {{
        let mut pod = pod::array();
        pod.as_mut().write($ty $($tt)*)?;
        let read = pod.as_ref().read::<$ty>()?;
        assert_eq!(read, $ty $($tt)*);
        Ok::<_, pod::Error>(())
    }};
}

#[test]
fn basic() -> Result<(), Error> {
    #[derive(Debug, PartialEq, Readable, Writable)]
    struct Struct {
        channels: u32,
    }

    roundtrip!(Struct { channels: 40 })?;
    Ok(())
}

#[test]
fn with_lifetime() -> Result<(), Error> {
    #[derive(Debug, PartialEq, Readable, Writable)]
    struct Struct<'de> {
        a: &'de [u8],
        b: &'de [u8],
    }

    roundtrip!(Struct {
        a: &b"hello"[..],
        b: &b"world"[..],
    })?;
    Ok(())
}
