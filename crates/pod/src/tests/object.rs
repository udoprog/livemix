use crate::{Error, Pod, Type};

#[test]
fn read_written() -> Result<(), Error> {
    let mut pod = crate::array();

    let obj = pod.as_mut().write_object(10, 20, |obj| {
        obj.property(1).write(1i32)?;
        obj.property(2).write(2i32)?;
        obj.property(3).write(3i32)?;
        Ok(())
    })?;

    let mut obj = obj.as_ref();

    assert_eq!(obj.object_type(), 10);
    assert_eq!(obj.object_id(), 20);
    let mut p = obj.property()?;
    Ok(())
}
