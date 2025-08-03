use crate::{ChoiceType, Error, Id, Type};

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

    std::dbg!(&obj);

    assert_eq!(obj.object_type(), 10);
    assert_eq!(obj.object_id(), 20);
    let p = obj.property()?;
    assert_eq!(p.key(), 1);
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
    assert_eq!(p.key(), 3);
    std::dbg!(p.value().read::<Id<u32>>()?);
    Ok(())
}
