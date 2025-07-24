use core::fmt;

macro_rules! declare {
    ($ty_vis:vis struct $ty:ident {
        $(
            #[name = $name:literal, size = $size:expr]
            $vis:vis const $ident:ident = $value:expr;
        )*
    }) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[repr(transparent)]
        $ty_vis struct $ty(u32);

        impl $ty {
            $(
                #[doc = concat!(" The `", $name, "` type.")]
                $vis const $ident: Self = Self($value);
            )*

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

            /// Get the size of the type.
            #[inline]
            pub(crate) fn size(&self) -> Option<usize> {
                match *self {
                    $(Self::$ident => $size,)*
                    _ => None,
                }
            }
        }

        impl fmt::Display for $ty {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match *self {
                    $(Self::$ident => write!(f, $name),)*
                    _ => write!(f, "Unknown({})", self.0),
                }
            }
        }

        impl fmt::Debug for $ty {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }
    };
}

declare! {
    pub struct Type {
        #[name = "None", size = Some(0)]
        pub const NONE = 1;
        #[name = "Bool", size = Some(4)]
        pub const BOOL = 2;
        #[name = "Id", size = Some(4)]
        pub const ID = 3;
        #[name = "Int", size = Some(4)]
        pub const INT = 4;
        #[name = "Long", size = Some(8)]
        pub const LONG = 5;
        #[name = "Float", size = Some(4)]
        pub const FLOAT = 6;
        #[name = "Double", size = Some(8)]
        pub const DOUBLE = 7;
        #[name = "String", size = None]
        pub const STRING = 8;
        #[name = "Bytes", size = None]
        pub const BYTES = 9;
        #[name = "Rectangle", size = Some(8)]
        pub const RECTANGLE = 10;
        #[name = "Fraction", size = Some(8)]
        pub const FRACTION = 11;
        #[name = "Bitmap", size = None]
        pub const BITMAP = 12;
        #[name = "Array", size = None]
        pub const ARRAY = 13;
        #[name = "Struct", size = None]
        pub const STRUCT = 14;
        #[name = "Object", size = None]
        pub const OBJECT = 15;
        #[name = "Sequence", size = None]
        pub const SEQUENCE = 16;
        #[name = "Pointer", size = Some(16)]
        pub const POINTER = 17;
    }
}
