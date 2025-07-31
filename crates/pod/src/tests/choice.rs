use crate::{ChoiceType, Type};

#[test]
fn choice_read() -> Result<(), crate::Error> {
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
    assert_eq!(choice.len(), 3);
    let a = choice.read::<i32>()?;
    // let (a, b, c) = choice.read::<(i32, i32, i32)>()?;

    assert_eq!(a, 10);
    // assert_eq!(b, 0);
    // assert_eq!(c, 30);
    Ok(())
}
