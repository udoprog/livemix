use core::ffi::CStr;

use alloc::format;

use crate::{Error, Rectangle};

#[test]
fn unit() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().write_struct(|st| st.write(()))?;

    let st = pod.as_ref().read_struct()?;
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn encode_ints() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().write_struct(|st| st.write((1, 2, 3)))?;

    let mut st = pod.as_ref().read_struct()?;
    assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn decode_ints() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().write_struct(|st| {
        st.field().write_sized(1i32)?;
        st.field().write_sized(2i32)?;
        st.field().write_sized(3i32)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().read_struct()?;
    assert_eq!(st.read::<(i32, i32, i32)>()?, (1, 2, 3));
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn complex_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().write_struct(|st| {
        st.field().write_sized(1i32)?;
        st.field().write_sized(2i32)?;

        st.field().write_struct(|inner| {
            inner.field().write_unsized(c"hello world")?;
            inner.field().write_sized(Rectangle::new(800, 600))?;
            inner.field().write_unsized(c"goodbye world")?;
            Ok(())
        })
    })?;

    let mut st = pod.as_ref().read_struct()?;
    assert!(!st.is_empty());
    assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    assert!(!st.is_empty());

    let mut inner = st.field()?.read_struct()?;
    assert!(!inner.is_empty());
    assert_eq!(inner.field()?.read_unsized::<CStr>()?, c"hello world");
    assert_eq!(
        inner.field()?.read_sized::<Rectangle>()?,
        Rectangle::new(800, 600)
    );
    assert_eq!(inner.field()?.read_unsized::<CStr>()?, c"goodbye world");
    assert!(inner.is_empty());

    assert!(inner.field().is_err());

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn basic_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().write_struct(|st| {
        st.field().write_sized(1i32)?;
        st.field().write_sized(2i32)?;
        st.field().write_sized(3i32)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().read_struct()?;

    assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    assert!(st.is_empty());
    Ok(())
}

#[test]
fn string_decode() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().write_struct(|st| {
        st.field().write_sized(1i32)?;
        st.field().write_unsized("foo")?;
        Ok(())
    })?;

    let mut st = pod.as_ref().read_struct()?;

    assert!(!st.is_empty());
    assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    assert_eq!(st.field()?.read_unsized::<str>()?, "foo");
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

    pod.as_mut().write_struct(|st| {
        st.field().write_unsized(factory_name)?;
        st.field().write_unsized(ty)?;
        st.field().write_sized(version)?;

        st.field().write_struct(|props| {
            for &(key, value) in PROPS {
                props.field().write_unsized(key)?;
                props.field().write_unsized(value)?;
            }

            Ok(())
        })?;

        st.field().write_sized(new_id)?;
        Ok(())
    })?;

    let mut st = pod.as_ref().read_struct()?;

    assert_eq!(st.field()?.read_unsized::<str>()?, factory_name);
    assert_eq!(st.field()?.read_unsized::<str>()?, ty);
    assert_eq!(st.field()?.read_sized::<i32>()?, version);

    let mut inner = st.field()?.read_struct()?;

    for &(k, v) in PROPS {
        let key = inner.field()?.read_unsized::<str>()?;
        let value = inner.field()?.read_unsized::<str>()?;
        assert_eq!(key, k);
        assert_eq!(value, v);
    }

    Ok(())
}

#[test]
fn write_unsized_into() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().write_struct(|st| st.write(("foo", "bar")))?;

    let mut st = pod.as_ref().read_struct()?;

    assert_eq!(st.field()?.read_unsized::<str>()?, "foo");
    assert_eq!(st.field()?.read_unsized::<str>()?, "bar");

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn decode_unsized() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().write_struct(|st| {
        st.field().write_unsized("foo")?;
        st.field().write_unsized("bar")?;
        Ok(())
    })?;

    let mut st = pod.as_ref().read_struct()?;
    let (key, value) = st.read::<(&str, &str)>()?;

    assert_eq!(key, "foo");
    assert_eq!(value, "bar");

    assert!(st.is_empty());
    Ok(())
}

#[test]
fn format() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().write_struct(|st| {
        st.field().write_sized(1i32)?;
        st.field().write_sized(2i32)?;

        st.field().write_struct(|inner| {
            inner.field().write_sized(*b"hello world")?;
            inner.field().write_sized(Rectangle::new(800, 600))?;
            inner.field().write_sized(*b"goodbye world")?;
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
    pod.as_mut().write_struct(|st| {
        st.field().write_sized(*b"a")?;
        st.field().write_sized(*b"b")?;
        Ok(())
    })?;

    let mut st = pod.as_ref().read_struct()?;
    assert_eq!(format!("{:?}", st.field()?), "b\"a\"");
    assert_eq!(format!("{:?}", st.field()?), "b\"b\"");
    assert_eq!(format!("{pod:?}"), "Struct { fields: [b\"a\", b\"b\"] }");
    Ok(())
}

#[test]
fn write_read() -> Result<(), Error> {
    let mut pod = crate::array();
    pod.as_mut().write((10i32, "hello world", [1u32, 2]))?;

    let mut pod = pod.as_ref();

    let a = pod.as_mut().into_value()?.read::<i32>()?;
    assert_eq!(a, 10i32);

    let s = pod.as_mut().into_value()?.read::<&str>()?;
    assert_eq!(s, "hello world");

    let a1 = pod.as_mut().into_value()?.read::<u32>()?;
    assert_eq!(a1, 1);

    let a2 = pod.as_mut().into_value()?.read::<u32>()?;
    assert_eq!(a2, 2);
    Ok(())
}
