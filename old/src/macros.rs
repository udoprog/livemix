macro_rules! __bitflags {
    ($vis:vis struct $name:ident($ty:ty) {
        $(
            $(#[$($meta:meta)*])*
            $flag_vis:vis const $flag:ident = $value:expr;
        )*
    }) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[repr(transparent)]
        $vis struct $name($ty);

        impl $name {
            $(
                $(#[$($meta)*])*
                $flag_vis const $flag: Self = Self($value);
            )*

            /// Convert the flags to a raw u32 value.
            #[inline]
            $vis fn into_raw(self) -> u32 {
                self.0
            }

            /// Create a new `StreamFlags` from a raw u32 value.
            #[inline]
            $vis fn from_raw(value: u32) -> Self {
                Self(value)
            }
        }

        impl core::ops::BitOr for $name {
            type Output = Self;

            #[inline]
            fn bitor(self, rhs: Self) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl fmt::Debug for StreamFlags {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.0 == 0 {
                    return write!(f, "NONE");
                }

                let mut first = true;

                let mut write = |flag: &'static str| {
                    if !first {
                        write!(f, " | ")?;
                    }

                    flag.fmt(f)?;
                    first = false;
                    Ok(())
                };

                $(
                    if self.0 & Self::$flag.0 != 0 {
                        write(stringify!($flag))?;
                    }
                )*

                Ok(())
            }
        }
    }
}

pub(crate) use __bitflags as bitflags;

macro_rules! __decl_enum {
    (
        $(
            #[repr($repr:ty)]
            $vis:vis enum $name:ident {
                $(#[$first_meta:meta])*
                $first_variant:ident = $first_value:path,
                $(
                    $(#[$meta:meta])*
                    $variant:ident = $value:path,
                )*
            }
        )*
    ) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[repr($repr)]
            $vis enum $name {
                $(#[$first_meta])*
                $first_variant = $first_value,
                $(
                    $(#[$meta])*
                    $variant = $value,
                )*
            }

            impl $name {
                #[inline]
                $vis fn into_raw(self) -> $repr {
                    self as $repr
                }

                #[doc = concat!(" Convert a raw ", stringify!($repr), " value to a `", stringify!($name), "`.")]
                #[inline]
                $vis fn from_raw(value: $repr) -> Self {
                    match value {
                        $($value => Self::$variant,)*
                        _ => Self::$first_variant,
                    }
                }
            }
        )*
    }
}

pub(crate) use __decl_enum as decl_enum;

macro_rules! __endian {
    ($a:expr, $b:expr) => {
        if cfg!(target_endian = "little") {
            $a
        } else {
            $b
        }
    };
}

pub(crate) use __endian as endian;
