use pod::{ChoiceType, Error, Readable, Type, Writable};

macro_rules! roundtrip {
    ($ty:ident $($tt:tt)*) => {{
        let mut pod = pod::array();
        pod.as_mut().write($ty $($tt)*)?;
        let read = pod.as_ref().read::<$ty>()?;
        assert_eq!(read, $ty $($tt)*);
        Ok::<_, pod::Error>(())
    }};
}

#[test]
fn object() -> Result<(), Error> {
    use pod::{Readable, Writable};
    use protocol::id;

    #[derive(Debug, PartialEq, Readable, Writable)]
    #[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
    struct RawFormat {
        #[pod(property(key = id::Format::MEDIA_TYPE))]
        media_type: id::MediaType,
        #[pod(property(key = id::Format::MEDIA_SUB_TYPE))]
        media_sub_type: id::MediaSubType,
        #[pod(property(key = id::Format::AUDIO_FORMAT))]
        audio_format: id::AudioFormat,
        #[pod(property(key = id::Format::AUDIO_CHANNELS))]
        channels: u32,
        #[pod(property = id::Format::AUDIO_RATE)]
        audio_rate: u32,
    }

    roundtrip!(RawFormat {
        media_type: id::MediaType::AUDIO,
        media_sub_type: id::MediaSubType::DSP,
        audio_format: id::AudioFormat::F32,
        channels: 2,
        audio_rate: 44100
    })?;
    Ok(())
}

#[test]
fn empty_object() -> Result<(), Error> {
    use pod::{Readable, Writable};
    use protocol::id;

    #[derive(Debug, PartialEq, Readable, Writable)]
    #[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
    struct RawObject {}

    roundtrip!(RawObject {})?;
    Ok(())
}

#[test]
fn choice_field() -> Result<(), Error> {
    use pod::{Readable, Writable};
    use protocol::id;

    #[derive(Debug, PartialEq, Readable, Writable)]
    #[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
    struct RawFormat {
        #[pod(property(key = id::Format::MEDIA_TYPE))]
        media_type: id::MediaType,
    }

    let mut pod = pod::array();

    pod.as_mut()
        .write_object(id::ObjectType::FORMAT, id::Param::FORMAT, |obj| {
            obj.property(id::Format::MEDIA_TYPE).write_choice(
                ChoiceType::NONE,
                Type::ID,
                |choice| {
                    choice.write((
                        id::MediaType::AUDIO,
                        id::MediaType::VIDEO,
                        id::MediaType::APPLICATION,
                    ))
                },
            )
        })?;

    let read = pod.as_ref().read::<RawFormat>()?;

    assert_eq!(
        read,
        RawFormat {
            media_type: id::MediaType::AUDIO,
        }
    );
    Ok(())
}
