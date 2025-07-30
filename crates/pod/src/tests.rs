use core::ffi::CStr;

use alloc::format;
use alloc::string::String;

use crate::buf::CapacityError;
use crate::error::ErrorKind;
use crate::utils::{Align, AlignableWith};
use crate::{
    ArrayBuf, AsReader, Bitmap, DynamicBuf, Error, Fraction, OwnedBitmap, Pod, Rectangle, Type,
    Writer,
};
use crate::{ChoiceType, Reader};

pub(crate) fn read<T, U>(value: T) -> U
where
    T: AlignableWith<U>,
{
    // SAFETY: The value must be word-aligned and packed.
    unsafe { Align(value).as_ptr().cast::<U>().read() }
}

#[test]
fn sandbox() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_unsized(Bitmap::new(b"hello world"))?;

    assert_eq!(
        pod.as_ref().next::<OwnedBitmap>()?.as_bytes(),
        b"hello world"
    );
    Ok(())
}

#[inline]
fn push_none() -> Result<Pod<impl AsReader<u64>>, Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_none()?;
    Ok(pod)
}

#[inline]
fn expected(expected: Type, actual: Type) -> ErrorKind {
    ErrorKind::Expected { expected, actual }
}

#[test]
fn test_push_decode_u64() -> Result<(), Error> {
    let mut buf = ArrayBuf::<u64>::new();
    buf.write(0x1234567890abcdefu64)?;

    let mut buf = buf.as_slice();

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
    let mut pod = Pod::new(ArrayBuf::<_, 1>::new());
    assert!(pod.as_mut().push_none().is_ok());

    assert_eq!(
        pod.as_mut().push_none().unwrap_err().kind(),
        ErrorKind::CapacityError(CapacityError)
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
    let buf = ArrayBuf::<u64, 3>::from_array([1, 2, 3]);
    let mut buf = buf.as_slice();

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
    let pod = push_none()?;

    assert!(pod.as_ref().next_option()?.is_none());

    assert_eq!(
        pod.as_ref().next::<bool>().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bool() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<bool>().unwrap_err().kind(),
        expected(Type::BOOL, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_int() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<i32>().unwrap_err().kind(),
        expected(Type::INT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_long() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<i64>().unwrap_err().kind(),
        expected(Type::LONG, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_float() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<f32>().unwrap_err().kind(),
        expected(Type::FLOAT, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_double() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<f64>().unwrap_err().kind(),
        expected(Type::DOUBLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_string() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next_borrowed::<CStr>().unwrap_err().kind(),
        expected(Type::STRING, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bytes() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next_borrowed::<[u8]>().unwrap_err().kind(),
        expected(Type::BYTES, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_rectangle() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<Rectangle>().unwrap_err().kind(),
        expected(Type::RECTANGLE, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_fraction() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<Fraction>().unwrap_err().kind(),
        expected(Type::FRACTION, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bitmap() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next_borrowed::<Bitmap>().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next::<OwnedBitmap>().unwrap_err().kind(),
        expected(Type::BITMAP, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_array() -> Result<(), Error> {
    let mut pod = Pod::array();

    pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
        array.child().push_unsized("foo")?;
        array.child().push_unsized("bar")?;
        array.child().push_unsized("baz")?;
        Ok(())
    })?;

    let mut array = pod.as_ref().next_array()?;

    assert_eq!(array.len(), 3);
    assert_eq!(array.next().unwrap().next_borrowed::<CStr>()?, c"foo");
    assert_eq!(array.len(), 2);
    assert_eq!(array.next().unwrap().next_borrowed::<CStr>()?, c"bar");
    assert_eq!(array.len(), 1);
    assert_eq!(array.next().unwrap().next_borrowed::<CStr>()?, c"baz");

    assert!(array.is_empty());
    assert_eq!(array.len(), 0);
    Ok(())
}

#[test]
fn test_decode_complex_struct() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_struct(|st| {
        st.field().push(1i32)?;
        st.field().push(2i32)?;

        st.field().push_struct(|inner| {
            inner.field().push_unsized(c"hello world")?;
            inner.field().push(Rectangle::new(800, 600))?;
            inner.field().push_unsized(c"goodbye world")?;
            Ok(())
        })
    })?;

    let mut st = pod.as_ref().next_struct()?;
    assert!(!st.is_empty());
    assert_eq!(st.field()?.next::<i32>()?, 1i32);
    assert_eq!(st.field()?.next::<i32>()?, 2i32);
    assert!(!st.is_empty());

    let mut inner = st.field()?.next_struct()?;
    assert!(!inner.is_empty());
    assert_eq!(inner.field()?.next_borrowed::<CStr>()?, c"hello world");
    assert_eq!(
        inner.field()?.next::<Rectangle>()?,
        Rectangle::new(800, 600)
    );
    assert_eq!(inner.field()?.next_borrowed::<CStr>()?, c"goodbye world");
    assert!(inner.is_empty());

    assert!(inner.field().is_err());

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn test_decode_struct() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_struct(|st| {
        st.field().push(1i32)?;
        st.field().push(2i32)?;
        st.field().push(3i32)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;

    assert!(!st.is_empty());
    assert_eq!(st.field()?.next::<i32>()?, 1i32);
    assert_eq!(st.field()?.next::<i32>()?, 2i32);
    assert_eq!(st.field()?.next::<i32>()?, 3i32);
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn test_format_struct() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_struct(|st| {
        st.field().push(1i32)?;
        st.field().push(2i32)?;

        st.field().push_struct(|inner| {
            inner.field().push(*b"hello world")?;
            inner.field().push(Rectangle::new(800, 600))?;
            inner.field().push(*b"goodbye world")?;
            Ok(())
        })
    })?;

    assert_eq!(
        format!("{pod:?}"),
        "Struct { fields: [1, 2, Struct { fields: [b\"hello world\", Rectangle { width: 800, height: 600 }, b\"goodbye world\"] }] }"
    );
    Ok(())
}

#[test]
fn test_format_object() -> Result<(), Error> {
    let mut pod = Pod::array();

    pod.as_mut().push_object(10, 20, |obj| {
        obj.property_with_flags(1, 0b100)?.push(1i32)?;
        obj.property_with_flags(2, 0b010)?.push(2i32)?;

        obj.property_with_flags(3, 0b001)?.push_struct(|st| {
            st.field().push(*b"hello world")?;
            st.field().push(Rectangle::new(800, 600))?;
            st.field().push(*b"goodbye world")?;
            Ok(())
        })
    })?;

    assert_eq!(
        format!("{pod:?}"),
        "Object { \
            object_type: 10, \
            object_id: 20, \
            properties: [\
                Property { key: 1, flags: 4, value: 1 }, \
                Property { key: 2, flags: 2, value: 2 }, \
                Property { \
                    key: 3, \
                    flags: 1, \
                    value: Struct { \
                        fields: [\
                            b\"hello world\", \
                            Rectangle { width: 800, height: 600 }, \
                            b\"goodbye world\"\
                        ] \
                    } \
                }\
            ] \
        }"
    );
    Ok(())
}

#[test]
fn test_format_array() -> Result<(), Error> {
    let mut pod = Pod::array();

    pod.as_mut().push_array(Type::INT, |array| {
        array.child().push(1i32)?;
        array.child().push(2i32)?;
        array.child().push(3i32)?;
        Ok(())
    })?;

    assert_eq!(
        format!("{pod:?}"),
        "Array { child_type: Int, entries: [1, 2, 3] }"
    );
    Ok(())
}

#[test]
fn test_format_l1_struct() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_struct(|st| {
        st.field().push(*b"a")?;
        st.field().push(*b"b")?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;
    assert_eq!(format!("{:?}", st.field()?), "b\"a\"");
    assert_eq!(format!("{:?}", st.field()?), "b\"b\"");
    assert_eq!(format!("{pod:?}"), "Struct { fields: [b\"a\", b\"b\"] }");
    Ok(())
}

#[test]
fn test_format_choice() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut()
        .push_choice(ChoiceType::RANGE, Type::INT, |choice| {
            choice.child().push(10i32)?;
            choice.child().push(0i32)?;
            choice.child().push(30i32)?;
            Ok(())
        })?;

    assert_eq!(
        format!("{pod:?}"),
        "Choice { type: Range, child_type: Int, entries: [10, 0, 30] }"
    );
    Ok(())
}

#[test]
fn test_format_buggy() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut()
        .push_choice(ChoiceType::RANGE, Type::INT, |choice| {
            choice.child().push(10i32)?;
            choice.child().push(30i32)?;
            choice.child().push(0i32)?;
            Ok(())
        })?;

    let mut array = pod.into_buf();

    array.as_slice_mut()[2] = u64::MAX; // Corrupt the pod.

    let pod = Pod::new(array.as_slice());
    assert_eq!(
        format!("{pod:?}"),
        "Choice { type: Range, child_type: Unknown(4294967295), entries: [] }"
    );
    Ok(())
}

#[test]
fn test_array_drop() -> Result<(), Error> {
    let mut array = ArrayBuf::<String>::new();
    array.push(String::from("foo"))?;
    array.push(String::from("bar"))?;
    array.push(String::from("baz"))?;
    Ok(())
}

#[test]
fn test_struct_decoding() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_struct(|st| st.encode((1, 2, 3)))?;

    let mut st = pod.as_ref().next_struct()?;
    assert_eq!(st.decode::<(i32, i32, i32)>()?, (1, 2, 3));
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn test_struct_unit() -> Result<(), Error> {
    let mut pod = Pod::array();
    pod.as_mut().push_struct(|st| st.encode(()))?;

    let st = pod.as_ref().next_struct()?;
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn test_realloc() -> Result<(), Error> {
    let mut buf = DynamicBuf::<u64>::new();

    for n in 0..128 {
        buf.extend_from_words(&[n])?;
    }

    Ok(())
}
