use core::ffi::CStr;

use pod::{PodSink, UnsizedWritable, Writable};

/// The key of a property.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct Prop(CStr);

impl Prop {
    /// Create a new property.
    pub(crate) const fn new(name: &CStr) -> &Self {
        // SAFETY: A property is repr transparent over a `CStr`.
        unsafe { &*(name as *const CStr as *const Prop) }
    }
}

macro_rules! properties {
    ($($name:ident = $value:literal;)*) => {
        $(
            #[doc = concat!(" A property with the value `", stringify!($value), "`.`")]
            pub const $name: &Prop = Prop::new($value);
        )*

        impl Prop {
            /// Lookup property.
            pub fn get(name: &CStr) -> Option<&'static Self> {
                $(
                    if name == $value {
                        return Some($name);
                    }
                )*

                None
            }
        }
    };
}

impl UnsizedWritable for Prop {
    const TYPE: pod::Type = CStr::TYPE;

    #[inline]
    fn size(&self) -> Option<usize> {
        CStr::size(&self.0)
    }

    #[inline]
    fn write_content(&self, writer: impl pod::Writer) -> Result<(), pod::Error> {
        CStr::write_content(&self.0, writer)
    }
}

impl Writable for Prop {
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), pod::Error> {
        CStr::write_into(&self.0, pod)
    }
}

properties! {
    APPLICATION_NAME = c"application.name";
    NODE_NAME = c"node.name";
}
