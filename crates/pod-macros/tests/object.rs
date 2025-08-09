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
    use protocol::id::{AudioFormat, FormatKey, MediaSubType, MediaType, ObjectType, Param};

    #[derive(Debug, PartialEq, Readable, Writable)]
    #[pod(object(type = ObjectType::FORMAT, id = Param::FORMAT))]
    struct RawFormat {
        #[pod(property(key = FormatKey::MEDIA_TYPE))]
        media_type: MediaType,
        #[pod(property(key = FormatKey::MEDIA_SUB_TYPE))]
        media_sub_type: MediaSubType,
        #[pod(property(key = FormatKey::AUDIO_FORMAT))]
        audio_format: AudioFormat,
        #[pod(property(key = FormatKey::AUDIO_CHANNELS))]
        channels: u32,
        #[pod(property = FormatKey::AUDIO_RATE)]
        audio_rate: u32,
    }

    roundtrip!(RawFormat {
        media_type: MediaType::AUDIO,
        media_sub_type: MediaSubType::DSP,
        audio_format: AudioFormat::F32,
        channels: 2,
        audio_rate: 44100
    })?;
    Ok(())
}

#[test]
fn empty_object() -> Result<(), Error> {
    use pod::{Readable, Writable};
    use protocol::id::{ObjectType, Param};

    #[derive(Debug, PartialEq, Readable, Writable)]
    #[pod(object(type = ObjectType::FORMAT, id = Param::FORMAT))]
    struct RawObject {}

    roundtrip!(RawObject {})?;
    Ok(())
}

#[test]
fn choice_field() -> Result<(), Error> {
    use pod::{Readable, Writable};
    use protocol::id::{FormatKey, MediaType, ObjectType, Param};

    #[derive(Debug, PartialEq, Readable, Writable)]
    #[pod(object(type = ObjectType::FORMAT, id = Param::FORMAT))]
    struct RawFormat {
        #[pod(property(key = FormatKey::MEDIA_TYPE))]
        media_type: MediaType,
    }

    let mut pod = pod::array();

    pod.as_mut()
        .write_object(ObjectType::FORMAT, Param::FORMAT, |obj| {
            obj.property(FormatKey::MEDIA_TYPE)
                .write_choice(ChoiceType::NONE, Type::ID, |choice| {
                    choice.write((MediaType::AUDIO, MediaType::VIDEO, MediaType::APPLICATION))
                })
        })?;

    let read = pod.as_ref().read::<RawFormat>()?;

    assert_eq!(
        read,
        RawFormat {
            media_type: MediaType::AUDIO,
        }
    );
    Ok(())
}
