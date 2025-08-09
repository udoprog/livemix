use crate::{ChoiceType, Error, Id, Readable, Type};

#[test]
fn embed_object() -> Result<(), Error> {
    let mut pod = crate::array();

    let obj = pod.as_mut().embed_object(10, 20, |obj| {
        obj.property(1).write(1i32)?;
        obj.property(2).write(2i32)?;
        obj.property(3).write(3i32)?;
        Ok(())
    })?;

    let mut obj = obj.as_ref();

    assert_eq!(obj.object_type::<u32>(), 10);
    assert_eq!(obj.object_id::<u32>(), 20);
    let p = obj.property()?;
    assert_eq!(p.key::<u32>(), 1);
    assert_eq!(p.value().read_sized::<i32>()?, 1);
    Ok(())
}

#[test]
fn stream_decode_choice() -> Result<(), Error> {
    let mut pod = crate::array();

    pod.as_mut().write_object(1, 2, |obj| {
        obj.property(3)
            .write_choice(ChoiceType::NONE, Type::ID, |choice| {
                choice.write((Id(77), Id(0), Id(1)))
            })
    })?;

    let mut obj = pod.as_ref().read_object()?;

    let p = obj.property()?;
    assert_eq!(p.key::<u32>(), 3);
    assert_eq!(p.value().read::<Id<u32>>()?, Id(77));
    Ok(())
}

#[test]
fn contents_decode() -> Result<(), Error> {
    #[derive(Readable)]
    #[pod(crate, object(type = 10u32, id = 20u32))]
    struct Contents {
        #[pod(property = 100u32)]
        value: u32,
    }

    let mut pod = crate::array();
    let obj = pod
        .as_mut()
        .embed_object(10u32, 20u32, |obj| obj.property(100u32).write(200))?;

    let c = obj.as_ref().read::<Contents>()?;

    assert_eq!(c.value, 200);
    Ok(())
}
