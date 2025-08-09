use core::{borrow::Borrow, cmp::Ordering};

use alloc::string::String;

use pod::{PodSink, UnsizedWritable, Writable};

macro_rules! properties {
    ($($name:ident = $value:literal;)*) => {
        $(
            #[doc = concat!(" A property with the value `", stringify!($value), "`.`")]
            pub const $name: &Prop = Prop::new($value);
        )*

        impl Prop {
            /// Lookup property.
            pub fn get(name: &str) -> Option<&'static Self> {
                match name {
                    $($value => Some($name),)*
                    _ => None,
                }
            }
        }
    };
}

properties! {
    APPLICATION_NAME = "application.name";
    NODE_NAME = "node.name";
    NODE_DESCRIPTION = "node.description";
    MEDIA_CLASS = "media.class";
    MEDIA_TYPE = "media.type";
    MEDIA_CATEGORY = "media.category";
    MEDIA_ROLE = "media.role";
    PORT_NAME = "port.name";
    FORMAT_DSP = "format.dsp";
}

/// The key of a property.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct Prop(str);

impl Prop {
    /// Construct a new property.
    pub(crate) const fn new(name: &str) -> &Self {
        // SAFETY: A property is repr transparent over a `str`.
        unsafe { &*(name as *const str as *const Prop) }
    }

    /// Get the string of the property.
    pub(crate) const fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for Prop {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        &self.0 == other
    }
}

impl PartialOrd<str> for Prop {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        Some(self.as_str().cmp(other))
    }
}

impl AsRef<Prop> for Prop {
    #[inline]
    fn as_ref(&self) -> &Prop {
        self
    }
}

impl Borrow<str> for Prop {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<Prop> for String {
    #[inline]
    fn borrow(&self) -> &Prop {
        Prop::new(self.as_str())
    }
}

impl AsRef<Prop> for String {
    #[inline]
    fn as_ref(&self) -> &Prop {
        Prop::new(self.as_str())
    }
}

impl AsRef<Prop> for str {
    #[inline]
    fn as_ref(&self) -> &Prop {
        Prop::new(self)
    }
}

impl UnsizedWritable for Prop {
    const TYPE: pod::Type = str::TYPE;

    #[inline]
    fn size(&self) -> Option<usize> {
        str::size(&self.0)
    }

    #[inline]
    fn write_unsized(&self, writer: impl pod::Writer) -> Result<(), pod::Error> {
        str::write_unsized(&self.0, writer)
    }
}

impl Writable for Prop {
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), pod::Error> {
        str::write_into(&self.0, pod)
    }
}
