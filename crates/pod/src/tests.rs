#![cfg(feature = "alloc")]

use super::Reader;
use super::error::ErrorKind;
use super::ty::Type;
use super::utils::Align;
use super::{ArrayBuf, Decoder, Encoder, Error, Slice, Writer};

#[inline]
fn encode_none() -> Result<Decoder<impl Reader<'static>>, Error> {
    let mut buf = ArrayBuf::new();
    let mut en = Encoder::new(&mut buf);
    en.encode_none()?;
    Ok(Decoder::new(buf))
}

#[inline]
fn expected(expected: Type, actual: Type) -> ErrorKind {
    ErrorKind::Expected { expected, actual }
}

#[test]
fn test_encode_decode_u64() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    buf.write_u64(0x1234567890abcdef)?;

    let &[a, b] = buf.as_slice() else {
        panic!();
    };

    if cfg!(target_endian = "little") {
        assert_eq!(a, 0x90abcdef);
        assert_eq!(b, 0x12345678);
        let align = Align::new([a, b]);
        assert_eq!(0x1234567890abcdefu64, align.read());
    } else {
        assert_eq!(b, 0x90abcdef);
        assert_eq!(a, 0x12345678);
        let align = Align::new([b, a]);
        assert_eq!(0x1234567890abcdefu64, align.read());
    }

    assert_eq!(buf.read_u64()?, 0x1234567890abcdef);
    Ok(())
}

#[test]
fn test_write_overflow() -> Result<(), Error> {
    let mut buf = ArrayBuf::<2>::with_size();
    let mut en = Encoder::new(&mut buf);

    assert!(en.encode_none().is_ok());
    assert_eq!(
        en.encode_none().unwrap_err().kind(),
        ErrorKind::BufferOverflow
    );
    Ok(())
}

#[test]
fn test_slice_underflow() -> Result<(), Error> {
    let mut buf = Slice::new(&[1, 2, 3]);
    assert_eq!(buf.read_u32()?, 1);
    assert_eq!(buf.read_u32()?, 2);
    assert_eq!(
        buf.read_u64().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.read_u32()?, 3);
    assert_eq!(
        buf.read_u32().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_array_underflow() -> Result<(), Error> {
    let mut buf = ArrayBuf::<3>::from_array([1, 2, 3]);
    assert_eq!(buf.read_u32()?, 1);
    assert_eq!(buf.read_u32()?, 2);
    assert_eq!(
        buf.read_u64().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.read_u32()?, 3);
    assert_eq!(
        buf.read_u32().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_none() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert!(de.decode_option()?.is_none());

    let mut de = encode_none()?;

    assert_eq!(
        de.decode_bool().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bool() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_bool().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_int() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_int().unwrap_err().kind(),
        expected(Type::INT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_long() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_long().unwrap_err().kind(),
        expected(Type::LONG, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_float() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_float().unwrap_err().kind(),
        expected(Type::FLOAT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_double() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_double().unwrap_err().kind(),
        expected(Type::DOUBLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_string() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_borrowed_c_str().unwrap_err().kind(),
        expected(Type::STRING, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bytes() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_borrowed_bytes().unwrap_err().kind(),
        expected(Type::BYTES, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_rectangle() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_rectangle().unwrap_err().kind(),
        expected(Type::RECTANGLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_fraction() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_fraction().unwrap_err().kind(),
        expected(Type::FRACTION, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bitmap() -> Result<(), Error> {
    let mut de = encode_none()?;

    assert_eq!(
        de.decode_borrowed_bitmap().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    let mut de = encode_none()?;

    assert_eq!(
        de.decode_owned_bitmap().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    Ok(())
}
