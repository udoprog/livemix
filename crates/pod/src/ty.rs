use core::fmt;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Type(u32);

macro_rules! declare {
    (impl Type {
        $(
            #[name = $name:literal]
            $vis:vis const $ident:ident = $value:expr;
        )*
    }) => {
        impl Type {
            $(
                #[doc = concat!(" The `", $name, "` type.")]
                $vis const $ident: Self = Self($value);
            )*
        }

        impl fmt::Display for Type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match *self {
                    $(Self::$ident => write!(f, $name),)*
                    _ => write!(f, "Unknown({})", self.0),
                }
            }
        }
    };
}

declare! {
    impl Type {
        #[name = "None"]
        pub(crate) const NONE = 1;
        #[name = "Bool"]
        pub(crate) const BOOL = 2;
        #[name = "Id"]
        pub(crate) const ID = 3;
        #[name = "Int"]
        pub(crate) const INT = 4;
        #[name = "Long"]
        pub(crate) const LONG = 5;
        #[name = "Float"]
        pub(crate) const FLOAT = 6;
        #[name = "Double"]
        pub(crate) const DOUBLE = 7;
        #[name = "String"]
        pub(crate) const STRING = 8;
        #[name = "Bytes"]
        pub(crate) const BYTES = 9;
        #[name = "Rectangle"]
        pub(crate) const RECTANGLE = 10;
        #[name = "Fraction"]
        pub(crate) const FRACTION = 11;
        #[name = "Bitmap"]
        pub(crate) const BITMAP = 12;
    }
}

impl Type {
    /// Construct a new type.
    #[inline]
    pub(crate) const fn new(ty: u32) -> Self {
        Self(ty)
    }

    /// Convert the type to a `u32`.
    #[inline]
    pub(crate) const fn into_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for Type {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
