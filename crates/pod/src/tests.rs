use core::ffi::CStr;

use alloc::format;

use crate::Reader;
use crate::error::ErrorKind;
use crate::utils::{Align, WordSized};
use crate::{
    ArrayBuf, Bitmap, Error, Fraction, OwnedBitmap, Pod, Rectangle, Type, TypedPod, Writer,
};

pub(crate) fn read<T, U>(value: T) -> U
where
    T: WordSized,
    U: WordSized,
{
    // SAFETY: The value must be word-aligned and packed.
    unsafe { Align(value).as_ptr().cast::<U>().read() }
}

#[inline]
fn encode_none() -> Result<Pod<impl Reader<'static>>, Error> {
    let mut buf = ArrayBuf::new();
    Pod::new(&mut buf).encode_none()?;
    Ok(Pod::new(buf))
}

#[inline]
fn expected(expected: Type, actual: Type) -> ErrorKind {
    ErrorKind::Expected { expected, actual }
}

#[test]
fn test_encode_decode_u64() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    buf.write(0x1234567890abcdefu64)?;

    let Ok([a, b]) = buf.peek::<[u32; 2]>() else {
        panic!();
    };

    if cfg!(target_endian = "little") {
        assert_eq!(a, 0x90abcdef);
        assert_eq!(b, 0x12345678);
        assert_eq!(0x1234567890abcdefu64, read([a, b]));
    } else {
        assert_eq!(b, 0x90abcdef);
        assert_eq!(a, 0x12345678);
        assert_eq!(0x1234567890abcdefu64, read([b, a]));
    }

    assert_eq!(buf.read::<u64>()?, 0x1234567890abcdef);
    Ok(())
}

#[test]
fn test_write_overflow() -> Result<(), Error> {
    let mut buf = ArrayBuf::<1>::with_size();

    let pod = Pod::new(&mut buf);
    assert!(pod.encode_none().is_ok());

    let pod = Pod::new(&mut buf);
    assert_eq!(
        pod.encode_none().unwrap_err().kind(),
        ErrorKind::BufferOverflow
    );
    Ok(())
}

#[test]
fn test_slice_underflow() -> Result<(), Error> {
    let mut buf: &[u64] = &[1, 2, 3];
    assert_eq!(buf.read::<u64>()?, 1);
    assert_eq!(buf.read::<u64>()?, 2);
    assert_eq!(
        buf.read::<[u64; 2]>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.read::<u64>()?, 3);
    assert_eq!(
        buf.read::<u64>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_array_underflow() -> Result<(), Error> {
    let mut buf = ArrayBuf::<3>::from_array([1, 2, 3]);
    assert_eq!(buf.read::<u64>()?, 1);
    assert_eq!(buf.read::<u64>()?, 2);
    assert_eq!(
        buf.read::<[u64; 2]>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.read::<u64>()?, 3);
    assert_eq!(
        buf.read::<u64>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_none() -> Result<(), Error> {
    let pod = encode_none()?;

    assert!(pod.decode_option()?.is_none());

    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<bool>().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bool() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<bool>().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_int() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<i32>().unwrap_err().kind(),
        expected(Type::INT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_long() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<i64>().unwrap_err().kind(),
        expected(Type::LONG, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_float() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<f32>().unwrap_err().kind(),
        expected(Type::FLOAT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_double() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<f64>().unwrap_err().kind(),
        expected(Type::DOUBLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_string() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode_borrowed::<CStr>().unwrap_err().kind(),
        expected(Type::STRING, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bytes() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode_borrowed::<[u8]>().unwrap_err().kind(),
        expected(Type::BYTES, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_rectangle() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<Rectangle>().unwrap_err().kind(),
        expected(Type::RECTANGLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_fraction() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<Fraction>().unwrap_err().kind(),
        expected(Type::FRACTION, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bitmap() -> Result<(), Error> {
    let pod = encode_none()?;

    assert_eq!(
        pod.decode_borrowed::<Bitmap>().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    let pod = encode_none()?;

    assert_eq!(
        pod.decode::<OwnedBitmap>().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_array() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let pod = Pod::new(&mut buf);
    let mut array = pod.encode_unsized_array(Type::STRING, 4)?;

    array.encode_unsized("foo")?;
    array.encode_unsized("bar")?;
    array.encode_unsized("baz")?;

    array.close()?;

    let pod = Pod::new(buf.as_slice());
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

#[test]
fn test_format_array() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let pod = Pod::new(&mut buf);
    let mut st = pod.encode_array(Type::INT)?;

    st.encode(1i32)?;
    st.encode(2i32)?;
    st.encode(3i32)?;

    st.close()?;

    let pod = Pod::new(buf.as_slice());
    assert_eq!(format!("{pod:?}"), "Array[Int](1, 2, 3)");
    Ok(())
}

#[test]
fn test_format_l1_struct() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let pod = Pod::new(&mut buf);
    let mut st = pod.encode_struct()?;

    st.field()?.encode(*b"a")?;
    st.field()?.encode(*b"b")?;
    st.close()?;

    let pod = Pod::new(buf.as_slice());

    let mut st = pod.decode_struct()?;
    assert_eq!(format!("{:?}", st.field()?), "Bytes(b\"a\")");
    assert_eq!(format!("{:?}", st.field()?), "Bytes(b\"b\")");

    // assert_eq!(format!("{pod:?}"), "Struct(Int: 1, Int: 2, Int: 3)");
    Ok(())
}

#[test]
fn test_format_struct() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let pod = Pod::new(&mut buf);
    let mut st = pod.encode_struct()?;

    st.field()?.encode(1i32)?;
    st.field()?.encode(2i32)?;

    let mut inner = st.field()?.encode_struct()?;
    inner.field()?.encode(*b"hello world")?;
    inner.field()?.encode(Rectangle::new(800, 600))?;
    inner.field()?.encode(*b"goodbye world")?;
    inner.close()?;

    st.close()?;

    let pod = Pod::new(buf.as_slice());
    assert_eq!(
        format!("{pod:?}"),
        "Struct(Int: 1, Int: 2, Struct: Struct(Bytes: b\"hello world\", Rectangle: Rectangle { width: 800, height: 600 }, Bytes: b\"goodbye world\"))"
    );
    Ok(())
}

#[test]
fn test_decode_complex_struct() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let pod = Pod::new(&mut buf);
    let mut st = pod.encode_struct()?;

    st.field()?.encode(1i32)?;
    st.field()?.encode(2i32)?;

    let mut inner = st.field()?.encode_struct()?;
    inner.field()?.encode(c"hello world")?;
    inner.field()?.encode(Rectangle::new(800, 600))?;
    inner.field()?.encode(c"goodbye world")?;
    inner.close()?;

    st.close()?;

    let pod = Pod::new(buf.as_slice());

    let mut st = pod.decode_struct()?;
    assert!(!st.is_empty());
    assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    assert!(!st.is_empty());

    let mut inner = st.field()?.decode_struct()?;
    assert!(!inner.is_empty());
    assert_eq!(inner.field()?.decode_borrowed::<CStr>()?, c"hello world");
    assert_eq!(
        inner.field()?.decode::<Rectangle>()?,
        Rectangle::new(800, 600)
    );
    assert_eq!(inner.field()?.decode_borrowed::<CStr>()?, c"goodbye world");
    assert!(inner.is_empty());

    assert!(inner.field().is_err());

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn test_decode_struct() -> Result<(), Error> {
    let mut buf = ArrayBuf::new();
    let pod = Pod::new(&mut buf);
    let mut st = pod.encode_struct()?;

    st.field()?.encode(1i32)?;
    st.field()?.encode(2i32)?;
    st.field()?.encode(3i32)?;

    st.close()?;

    let pod = TypedPod::from_reader(buf.as_slice())?;
    let mut st = pod.decode_struct()?;

    assert!(!st.is_empty());
    assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    assert!(st.is_empty());
    Ok(())
}
