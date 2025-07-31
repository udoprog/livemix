use core::ffi::CStr;

use alloc::format;

use crate::{Error, Rectangle};

#[test]
fn unit() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().push_struct(|st| st.encode(()))?;

    let st = pod.as_ref().next_struct()?;
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn encode_ints() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().push_struct(|st| st.encode((1, 2, 3)))?;

    let mut st = pod.as_ref().next_struct()?;
    assert_eq!(st.field()?.next::<i32>()?, 1i32);
    assert_eq!(st.field()?.next::<i32>()?, 2i32);
    assert_eq!(st.field()?.next::<i32>()?, 3i32);
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn decode_ints() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().push_struct(|st| {
        st.field().push(1i32)?;
        st.field().push(2i32)?;
        st.field().push(3i32)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;
    assert_eq!(st.decode::<(i32, i32, i32)>()?, (1, 2, 3));
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn complex_decode() -> Result<(), Error> {
    let mut pod = crate::array();

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
    assert_eq!(inner.field()?.next_unsized::<CStr>()?, c"hello world");
    assert_eq!(
        inner.field()?.next::<Rectangle>()?,
        Rectangle::new(800, 600)
    );
    assert_eq!(inner.field()?.next_unsized::<CStr>()?, c"goodbye world");
    assert!(inner.is_empty());

    assert!(inner.field().is_err());

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn basic_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_struct(|st| {
        st.field().push(1i32)?;
        st.field().push(2i32)?;
        st.field().push(3i32)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;

    assert_eq!(st.field()?.next::<i32>()?, 1i32);
    assert_eq!(st.field()?.next::<i32>()?, 2i32);
    assert_eq!(st.field()?.next::<i32>()?, 3i32);
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn string_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_struct(|st| {
        st.field().push(1i32)?;
        st.field().push_unsized("foo")?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;

    assert!(!st.is_empty());
    assert_eq!(st.field()?.next::<i32>()?, 1i32);
    assert_eq!(st.field()?.next_unsized::<str>()?, "foo");
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn build_struct() -> Result<(), Error> {
    let factory_name = "client-node";
    let ty = "audio";
    let version = 1;
    let new_id = 2;

    const PROPS: &[(&str, &str)] = &[
        ("node.description", "livemix"),
        ("node.name", "livemix_node"),
        ("media.class", "Audio/Duplex"),
        ("media.type", "Audio"),
        ("media.category", "Duplex"),
        ("media.role", "DSP"),
    ];

    let mut pod = crate::array();

    pod.as_mut().push_struct(|st| {
        st.field().push_unsized(factory_name)?;
        st.field().push_unsized(ty)?;
        st.field().push(version)?;

        st.field().push_struct(|props| {
            for &(key, value) in PROPS {
                props.field().push_unsized(key)?;
                props.field().push_unsized(value)?;
            }

            Ok(())
        })?;

        st.field().push(new_id)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;

    assert_eq!(st.field()?.next_unsized::<str>()?, factory_name);
    assert_eq!(st.field()?.next_unsized::<str>()?, ty);
    assert_eq!(st.field()?.next::<i32>()?, version);

    let mut inner = st.field()?.next_struct()?;

    for &(k, v) in PROPS {
        let key = inner.field()?.next_unsized::<str>()?;
        let value = inner.field()?.next_unsized::<str>()?;
        assert_eq!(key, k);
        assert_eq!(value, v);
    }

    Ok(())
}

#[test]
fn encode_unsized() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_struct(|st| st.encode(("foo", "bar")))?;

    let mut st = pod.as_ref().next_struct()?;

    assert_eq!(st.field()?.next_unsized::<str>()?, "foo");
    assert_eq!(st.field()?.next_unsized::<str>()?, "bar");

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn decode_unsized() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().push_struct(|st| {
        st.field().push_unsized("foo")?;
        st.field().push_unsized("bar")?;
        Ok(())
    })?;

    let mut st = pod.as_ref().next_struct()?;
    let (key, value) = st.decode::<(&str, &str)>()?;

    assert_eq!(key, "foo");
    assert_eq!(value, "bar");

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn format() -> Result<(), Error> {
    let mut pod = crate::array();
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
fn format_l1_struct() -> Result<(), Error> {
    let mut pod = crate::array();
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
