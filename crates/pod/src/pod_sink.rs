use crate::{BuildPod, Builder, Error, Writer};

/// A trait for reading pods as a stream.
pub trait PodSink {
    /// The buffer being written to.
    type Writer<'this>: Writer
    where
        Self: 'this;

    /// The flavor of pod being built.
    type BuildPod: BuildPod;

    /// Get the next pod from the stream.
    fn next(&mut self) -> Result<Builder<Self::Writer<'_>, Self::BuildPod>, Error>;
}
