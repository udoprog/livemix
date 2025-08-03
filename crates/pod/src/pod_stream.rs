use crate::{Error, Object, Readable, SizedReadable, Slice, Struct, UnsizedReadable};

/// The protocol for an item from a pod stream.
pub trait PodItem<'de>
where
    Self: Sized + PodStream<'de>,
{
    /// Read the item as a [`Readable`].
    fn read<T>(self) -> Result<T, Error>
    where
        T: Readable<'de>;

    /// The the next sized the item.
    fn read_sized<T>(self) -> Result<T, Error>
    where
        T: SizedReadable<'de>;

    /// The the next unsized the item.
    fn read_unsized<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>;

    /// The the next struct the item.
    fn read_struct(self) -> Result<Struct<Slice<'de>>, Error>;

    /// The the next object the item.
    fn read_object(self) -> Result<Object<Slice<'de>>, Error>;

    /// The the next optional pod the item.
    fn read_option(self) -> Result<Option<Self>, Error>;
}

/// A trait for reading pods as a stream.
pub trait PodStream<'de> {
    /// The returned stream item.
    type Item: PodItem<'de>;

    /// Get the next pod from the stream.
    fn next(&mut self) -> Result<Self::Item, Error>;
}
