use core::fmt;

macro_rules! id {
    (
        $($vis:vis struct $name:ident;)*
    ) => {
        $(
            /// A client node identifier.
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[repr(transparent)]
            pub struct $name(u32);

            impl $name {
                #[doc = concat!(" Create a new `", stringify!($name), "` from a `u32`.")]
                #[inline]
                pub fn new(id: u32) -> Self {
                    Self(id)
                }

                #[doc = concat!(" Convert the `", stringify!($name), "` into a `u32`.")]
                #[inline]
                pub(crate) fn into_u32(self) -> u32 {
                    self.0
                }

                /// Get the index of the client node.
                ///
                /// Since it was constructed from a `u32`, it can always be
                /// safely coerced into one.
                pub fn index(self) -> usize {
                    self.0 as usize
                }
            }

            impl fmt::Display for $name {
                #[inline]
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0.fmt(f)
                }
            }

            impl fmt::Debug for $name {
                #[inline]
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0.fmt(f)
                }
            }

            impl pod::Writable for $name {
                #[inline]
                fn write_into(&self, pod: &mut impl pod::PodSink) -> Result<(), pod::Error> {
                    pod.next()?.write(self.0)
                }
            }

            impl<'de> pod::Readable<'de> for $name {
                #[inline]
                fn read_from(pod: &mut impl pod::PodStream<'de>) -> Result<Self, pod::Error> {
                    let pod = pod.next()?;
                    Ok($name(pod::PodItem::read_sized(pod)?))
                }
            }
        )*
    }
}

id! {
    pub struct LocalId;
    pub struct GlobalId;
}
