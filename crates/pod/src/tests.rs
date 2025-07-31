mod choice;
mod struct_;

use core::ffi::CStr;

use alloc::format;
use alloc::string::String;

use crate::buf::{ArrayVec, CapacityError};
use crate::error::ErrorKind;
use crate::{
    ArrayBuf, AsSlice, Bitmap, Builder, DynamicBuf, Error, Fraction, OwnedBitmap, Pod, Rectangle,
    Type, Writer,
};
use crate::{ChoiceType, Reader};

pub(crate) fn read(value: [u32; 2]) -> u64 {
    // SAFETY: Same size, same supported bit patterns.
    unsafe { (&[value] as *const [u32; 2]).cast::<u64>().read_unaligned() }
}

#[test]
fn sandbox() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().push_unsized(Bitmap::new(b"hello world"))?;

    assert_eq!(
        pod.as_ref().next::<OwnedBitmap>()?.as_bytes(),
        b"hello world"
    );
    Ok(())
}

#[inline]
fn push_none() -> Result<Pod<impl AsSlice>, Error> {
    let mut pod = crate::array();
    pod.as_mut().push_none()?;
    Ok(pod.into_pod())
}

#[inline]
fn expected(expected: Type, actual: Type) -> ErrorKind {
    ErrorKind::Expected { expected, actual }
}

#[test]
fn test_push_decode_u64() -> Result<(), Error> {
    let mut buf = ArrayBuf::<128>::new();
    buf.write(&[0x1234567890abcdefu64])?;

    let mut buf = crate::buf::slice(buf.as_bytes());

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
    let mut pod = Builder::new(ArrayBuf::<8>::new());
    assert!(pod.as_mut().push_none().is_ok());

    assert_eq!(
        pod.as_mut().push_none().unwrap_err().kind(),
        ErrorKind::CapacityError(CapacityError)
    );
    Ok(())
}

#[test]
fn test_slice_underflow() -> Result<(), Error> {
    let mut buf = crate::buf::slice(&[1, 2, 3]);

    assert_eq!(buf.read::<u8>()?, 1);
    assert_eq!(buf.read::<u8>()?, 2);
    assert_eq!(
        buf.read::<[u8; 2]>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.read::<u8>()?, 3);
    assert_eq!(
        buf.read::<u8>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    Ok(())
}

#[test]
fn test_array_underflow() -> Result<(), Error> {
    let mut buf = crate::buf::slice(&[1, 2, 3]);

    assert_eq!(buf.read::<u8>()?, 1);
    assert_eq!(buf.read::<u8>()?, 2);
    assert_eq!(
        buf.read::<[u8; 2]>().unwrap_err().kind(),
        ErrorKind::BufferUnderflow
    );
    assert_eq!(buf.read::<u8>()?, 3);
    assert_eq!(
        buf.read::<u8>().unwrap_err().kind(),
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
        pod.as_ref().next_unsized::<CStr>().unwrap_err().kind(),
        expected(Type::STRING, Type::NONE)
    );

    Ok(())
}

#[test]
fn test_bytes() -> Result<(), Error> {
    let pod = push_none()?;

    assert_eq!(
        pod.as_ref().next_unsized::<[u8]>().unwrap_err().kind(),
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
        pod.as_ref().next_unsized::<Bitmap>().unwrap_err().kind(),
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
    let mut pod = crate::array();

    pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
        array.child().push_unsized("foo")?;
        array.child().push_unsized("bar")?;
        array.child().push_unsized("baz")?;
        Ok(())
    })?;

    let mut array = pod.as_ref().next_array()?;

    assert_eq!(array.len(), 3);
    assert_eq!(array.next().unwrap().next_unsized::<CStr>()?, c"foo");
    assert_eq!(array.len(), 2);
    assert_eq!(array.next().unwrap().next_unsized::<CStr>()?, c"bar");
    assert_eq!(array.len(), 1);
    assert_eq!(array.next().unwrap().next_unsized::<CStr>()?, c"baz");

    assert!(array.is_empty());
    assert_eq!(array.len(), 0);
    Ok(())
}

#[test]
fn array_padded_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_array(Type::INT, |array| {
        array.child().push(1i32)?;
        array.child().push(2i32)?;
        array.child().push(3i32)?;
        Ok(())
    })?;

    let mut array = pod.as_ref().next_array()?;

    assert!(!array.is_empty());
    assert_eq!(array.next().unwrap().next::<i32>()?, 1i32);
    assert_eq!(array.next().unwrap().next::<i32>()?, 2i32);
    assert_eq!(array.next().unwrap().next::<i32>()?, 3i32);
    assert!(array.is_empty());
    Ok(())
}

#[test]
fn array_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_array(Type::LONG, |array| {
        array.child().push(1i64)?;
        array.child().push(2i64)?;
        array.child().push(3i64)?;
        Ok(())
    })?;

    let mut array = pod.as_ref().next_array()?;

    assert!(!array.is_empty());
    assert_eq!(array.len(), 3);
    assert_eq!(array.next().unwrap().next::<i64>()?, 1i64);
    assert_eq!(array.len(), 2);
    assert_eq!(array.next().unwrap().next::<i64>()?, 2i64);
    assert_eq!(array.len(), 1);
    assert_eq!(array.next().unwrap().next::<i64>()?, 3i64);
    assert!(array.is_empty());
    assert!(array.next().is_none());
    Ok(())
}

#[test]
fn choice_decode() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut()
        .push_choice(ChoiceType::RANGE, Type::INT, |choice| {
            choice.child().push(10i32)?;
            choice.child().push(0i32)?;
            choice.child().push(30i32)?;
            Ok(())
        })?;

    let mut choice = pod.as_ref().next_choice()?;

    assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    assert_eq!(choice.child_type(), Type::INT);
    assert_eq!(choice.next().unwrap().next::<i32>()?, 10i32);
    assert_eq!(choice.next().unwrap().next::<i32>()?, 0i32);
    assert_eq!(choice.next().unwrap().next::<i32>()?, 30i32);
    Ok(())
}

#[test]
fn object_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_object(10, 20, |obj| {
        obj.property(1).flags(0b001).push(1i32)?;
        obj.property(2).flags(0b010).push(2i32)?;
        obj.property(3).flags(0b100).push(3i32)?;
        Ok(())
    })?;

    let obj = pod.as_ref().next_object()?.to_owned()?;

    let mut obj = obj.as_ref();
    assert!(!obj.is_empty());

    let p = obj.property()?;
    assert_eq!(p.key(), 1);
    assert_eq!(p.flags(), 0b001);
    assert_eq!(p.value().next::<i32>()?, 1);

    let p = obj.property()?;
    assert_eq!(p.key(), 2);
    assert_eq!(p.flags(), 0b010);
    assert_eq!(p.value().next::<i32>()?, 2);

    let p = obj.property()?;
    assert_eq!(p.key(), 3);
    assert_eq!(p.flags(), 0b100);
    assert_eq!(p.value().next::<i32>()?, 3);

    assert!(obj.is_empty());
    Ok(())
}

#[test]
fn array_string_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
        array.child().push_unsized("foo")?;
        array.child().push_unsized("bar")?;
        array.child().push_unsized("baz")?;
        Ok(())
    })?;

    assert_eq!(pod.as_buf().len(), 32);

    let mut array = pod.as_ref().next_array()?;
    assert_eq!(array.len(), 3);
    assert_eq!(array.next().unwrap().next_unsized::<str>()?, "foo");
    assert_eq!(array.len(), 2);
    assert_eq!(array.next().unwrap().next_unsized::<str>()?, "bar");
    assert_eq!(array.len(), 1);
    assert_eq!(array.next().unwrap().next_unsized::<str>()?, "baz");
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
    assert!(array.next().is_none());
    Ok(())
}

#[test]
fn string_decode() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().push_unsized("foo")?;
    assert_eq!(pod.as_ref().next_unsized::<str>()?, "foo");
    Ok(())
}

#[test]
fn sequence_decode() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().push_sequence(|seq| {
        seq.control().push(1i32)?;
        seq.control().push(2i32)?;
        seq.control().push(3i32)?;
        Ok(())
    })?;

    let mut seq = pod.as_ref().next_sequence()?;
    assert!(!seq.is_empty());
    assert_eq!(seq.control()?.value().next::<i32>()?, 1i32);
    assert_eq!(seq.control()?.value().next::<i32>()?, 2i32);
    assert_eq!(seq.control()?.value().next::<i32>()?, 3i32);
    assert!(seq.is_empty());
    Ok(())
}

#[test]
fn test_format_object() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_object(10, 20, |obj| {
        obj.property(1).flags(0b100).push(1i32)?;
        obj.property(2).flags(0b010).push(2i32)?;

        obj.property(3).flags(0b001).push_struct(|st| {
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
    let mut pod = crate::array();

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
fn test_format_buggy() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut()
        .push_choice(ChoiceType::RANGE, Type::INT, |choice| {
            choice.child().push(10i32)?;
            choice.child().push(30i32)?;
            choice.child().push(0i32)?;
            Ok(())
        })?;

    let mut array = pod.into_buf();

    array.as_bytes_mut()[20] = u8::MAX; // Corrupt the pod.

    let pod = Pod::new(array.as_bytes());
    assert_eq!(
        format!("{pod:?}"),
        "Choice { type: Range, child_type: Unknown(255), child_size: 4, entries: [{Unknown(255)}, {Unknown(255)}, {Unknown(255)}] }"
    );
    Ok(())
}

#[test]
fn test_array_drop() -> Result<(), Error> {
    let mut array = ArrayVec::<String>::new();
    array.push(String::from("foo"))?;
    array.push(String::from("bar"))?;
    array.push(String::from("baz"))?;
    Ok(())
}

#[test]
fn test_realloc() -> Result<(), Error> {
    let mut buf = DynamicBuf::new();

    for n in 0..128 {
        buf.extend_from_words(&[n])?;
    }

    Ok(())
}

#[test]
fn choice_format() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut()
        .push_choice(ChoiceType::RANGE, Type::INT, |choice| {
            choice.child().push(10i32)?;
            choice.child().push(0i32)?;
            choice.child().push(30i32)?;
            Ok(())
        })?;

    assert_eq!(
        format!("{pod:?}"),
        "Choice { type: Range, child_type: Int, child_size: 4, entries: [10, 0, 30] }"
    );
    Ok(())
}

#[test]
fn decode_bytes_array() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_array(Type::INT, |array| {
        array.child().push(1i32)?;
        array.child().push(2i32)?;
        array.child().push(3i32)?;
        Ok(())
    })?;

    let array = pod.as_ref().next_array()?;

    let mut pod2 = crate::array();
    pod2.as_mut().write(array)?;

    let mut array = pod2.as_ref().next_array()?;

    assert!(!array.is_empty());
    assert_eq!(array.len(), 3);

    assert_eq!(array.next().unwrap().next::<i32>()?, 1i32);
    assert_eq!(array.next().unwrap().next::<i32>()?, 2i32);
    assert_eq!(array.next().unwrap().next::<i32>()?, 3i32);

    assert!(array.is_empty());
    assert_eq!(array.len(), 0);
    Ok(())
}
