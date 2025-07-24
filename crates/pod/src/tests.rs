#![cfg(feature = "alloc")]

use core::ffi::CStr;

use super::Reader;
use super::error::ErrorKind;
use super::{ArrayBuf, Bitmap, Error, Fraction, OwnedBitmap, Pod, Rectangle, Slice, Type, Writer};

#[inline]
fn encode_none() -> Result<Pod<impl Reader<'static>>, Error> {
    let mut buf = ArrayBuf::new();
    let mut pod = Pod::new(&mut buf);
    pod.encode_none()?;
    Ok(Pod::new(buf))
}

#[inline]
fn expected(expected: Type, actual: Type) -> ErrorKind {
    ErrorKind::Expected { expected, actual }
}

#[test]
fn test_encode_decode_u64() -> Result<(), Error> {
    fn u64_from_array(values: [u32; 2]) -> u64 {
        // SAFETY: The u64 can inhabit all bit patterns presented.
        unsafe { (&values as *const [u32; 2]).cast::<u64>().read() }
    }

    let mut buf = ArrayBuf::new();
    buf.write(&0x1234567890abcdefu64)?;

    let &[a, b] = buf.as_slice() else {
        panic!();
    };

    if cfg!(target_endian = "little") {
        assert_eq!(a, 0x90abcdef);
        assert_eq!(b, 0x12345678);
        assert_eq!(0x1234567890abcdefu64, u64_from_array([a, b]));
    } else {
        assert_eq!(b, 0x90abcdef);
        assert_eq!(a, 0x12345678);
        assert_eq!(0x1234567890abcdefu64, u64_from_array([b, a]));
    }

    assert_eq!(buf.read::<u64>()?, 0x1234567890abcdef);
    Ok(())
}

#[test]
fn test_write_overflow() -> Result<(), Error> {
    let mut buf = ArrayBuf::<2>::with_size();
    let mut pod = Pod::new(&mut buf);

    assert!(pod.encode_none().is_ok());
    assert_eq!(
        pod.encode_none().unwrap_err().kind(),
        ErrorKind::BufferOverflow
    );
    Ok(())
}

#[test]
fn test_slice_underflow() -> Result<(), Error> {
    let mut buf = Slice::new(&[1, 2, 3]);
    assert_eq!(buf.array::<1>()?, [1]);
    assert_eq!(buf.array::<1>()?, [2]);
    assert_eq!(
        buf.read::<u64>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.array::<1>()?, [3]);
    assert_eq!(
        buf.array::<1>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_array_underflow() -> Result<(), Error> {
    let mut buf = ArrayBuf::<3>::from_array([1, 2, 3]);
    assert_eq!(buf.array::<1>()?, [1]);
    assert_eq!(buf.array::<1>()?, [2]);
    assert_eq!(
        buf.read::<u64>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.array::<1>()?, [3]);
    assert_eq!(
        buf.array::<1>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_none() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert!(pod.decode_option()?.is_none());

    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<bool>().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bool() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<bool>().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_int() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<i32>().unwrap_err().kind(),
        expected(Type::INT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_long() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<i64>().unwrap_err().kind(),
        expected(Type::LONG, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_float() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<f32>().unwrap_err().kind(),
        expected(Type::FLOAT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_double() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<f64>().unwrap_err().kind(),
        expected(Type::DOUBLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_string() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode_borrowed::<CStr>().unwrap_err().kind(),
        expected(Type::STRING, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bytes() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode_borrowed::<[u8]>().unwrap_err().kind(),
        expected(Type::BYTES, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_rectangle() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<Rectangle>().unwrap_err().kind(),
        expected(Type::RECTANGLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_fraction() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<Fraction>().unwrap_err().kind(),
        expected(Type::FRACTION, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bitmap() -> Result<(), Error> {
    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode_borrowed::<Bitmap>().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    let mut pod = encode_none()?;

    assert_eq!(
        pod.decode::<OwnedBitmap>().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_array() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let mut pod = Pod::new(&mut buf);
    let mut array = pod.encode_unsized_array(Type::STRING, 4)?;

    array.encode_unsized("foo")?;
    array.encode_unsized("bar")?;
    array.encode_unsized("baz")?;

    array.close()?;

    let mut pod = Pod::new(buf.as_reader_slice());
    let mut array = pod.decode_array()?;

    assert_eq!(array.len(), 3);
    assert_eq!(array.decode_borrowed::<CStr>()?, c"foo");
    assert_eq!(array.len(), 2);
    assert_eq!(array.decode_borrowed::<CStr>()?, c"bar");
    assert_eq!(array.len(), 1);
    assert_eq!(array.decode_borrowed::<CStr>()?, c"baz");

    assert!(array.is_empty());
    assert_eq!(array.len(), 0);
    Ok(())
}
