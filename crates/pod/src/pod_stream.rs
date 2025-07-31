use crate::{Error, PackedPod, Slice, TypedPod};

/// A trait for reading pods as a stream.
pub trait PodStream<'de> {
    /// Get the next pod from the stream.
    fn next(&mut self) -> Result<TypedPod<Slice<'de>, PackedPod>, Error>;
}
