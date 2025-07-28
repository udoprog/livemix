#[macro_export]
macro_rules! __id {
    (
        $(
            $(#[doc = $doc:literal])*
            #[example = $example:ident]
            $ty_vis:vis struct $ty:ident {
                $default:ident
                $(,
                    $(#[$($field_meta:meta)*])*
                    $field:ident = $field_value:expr
                )* $(,)?
            }
        )*
    ) => {
        $(
            $(#[doc = $doc])*
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            $ty_vis struct $ty(u32);

            impl $ty {
                $(
                    $(#[$($field_meta)*])*
                    $ty_vis const $field: Self = Self($field_value);
                )*
            }

            #[doc = concat!(" Encode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.push(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = $crate::Type::ID;

                #[inline]
                fn size(&self) -> usize {
                    4
                }

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    $crate::Id(*self).write_content(writer)
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            ///
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            #[doc = concat!(" Unknown identifiers will be decoded as the default value ", stringify!($default), ".")]
            ///
            /// ```
            /// use pod::{Pod, Id};
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            /// pod.as_mut().push(Id(u32::MAX / 2))?;
            ///
            #[doc = concat!(" let id = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            /// assert!(id.is_invalid());
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::Decode<'de> for $ty {
                const TYPE: $crate::Type = $crate::Type::ID;

                #[inline]
                fn read_content(reader: impl $crate::Reader<'de, u64>, len: usize) -> Result<Self, $crate::Error> {
                    let $crate::Id(id) = $crate::Id::<$ty>::read_content(reader, len)?;
                    Ok(id)
                }
            }

            impl $ty {
                /// Test if the identifier is invalid.
                pub fn is_invalid(&self) -> bool {
                    match self.0 {
                        $($field_value => false,)*
                        _ => true,
                    }
                }

                /// Get the identifier value.
                #[inline]
                pub fn into_id(self) -> u32 {
                    self.0
                }

                /// Convert an identifier value into the type.
                #[inline]
                pub fn from_id(value: u32) -> Self {
                    match value {
                        $($field_value => Self::$field,)*
                        _ => Self(value),
                    }
                }
            }

            impl $crate::RawId for $ty {
                #[inline]
                fn into_id(self) -> u32 {
                    <$ty>::into_id(self)
                }

                #[inline]
                fn from_id(value: u32) -> Self {
                    <$ty>::from_id(value)
                }
            }

            impl core::fmt::Debug for $ty {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self.0 {
                        $($field_value => write!(f, "{}", stringify!($field)),)*
                        _ => write!(f, "{}({})", stringify!($default), self.0),
                    }
                }
            }
        )*
    };
}

pub use __id as id;

#[macro_export]
macro_rules! __consts {
    (
        $(
            $(#[doc = $doc:literal])*
            #[example = $example:ident]
            $ty_vis:vis struct $ty:ident($repr:ty) {
                $default:ident
                $(,
                    $(#[$($field_meta:meta)*])*
                    $field:ident = $field_value:expr
                )* $(,)?
            }
        )*
    ) => {
        $(
            $(#[doc = $doc])*
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[repr(transparent)]
            $ty_vis struct $ty($repr);

            impl $ty {
                $(
                    $(#[$($field_meta)*])*
                    $ty_vis const $field: Self = Self($field_value);
                )*
            }

            #[doc = concat!(" Encode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.push(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = <$repr as $crate::Encode>::TYPE;

                #[inline]
                fn size(&self) -> usize {
                    <$repr as $crate::Encode>::size(&self.0)
                }

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    <$repr as $crate::Encode>::write_content(&self.0, writer)
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            ///
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            #[doc = concat!(" Unknown identifiers will be decoded as the default value ", stringify!($default), ".")]
            ///
            /// ```
            /// use pod::{Pod, Id};
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            /// pod.as_mut().push(Id(u32::MAX / 2))?;
            ///
            #[doc = concat!(" let id = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            /// assert!(id.is_invalid());
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::Decode<'de> for $ty {
                const TYPE: $crate::Type = <$repr as $crate::Decode<'de>>::TYPE;

                #[inline]
                fn read_content(reader: impl $crate::Reader<'de, u64>, len: usize) -> Result<Self, $crate::Error> {
                    Ok(Self(<$repr as $crate::Decode<'de>>::read_content(reader, len)?))
                }
            }

            impl $ty {
                /// Test if the identifier is invalid.
                pub fn is_invalid(&self) -> bool {
                    match self.0 {
                        $($field_value => false,)*
                        _ => true,
                    }
                }

                /// Get the identifier value.
                #[inline]
                pub fn into_raw(self) -> $repr {
                    self.0
                }

                /// Convert an identifier value into the type.
                #[inline]
                pub fn from_raw(value: $repr) -> Self {
                    match value {
                        $($field_value => Self::$field,)*
                        _ => Self(value),
                    }
                }
            }

            impl core::fmt::Debug for $ty {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self.0 {
                        $($field_value => write!(f, "{}", stringify!($field)),)*
                        _ => write!(f, "{}({})", stringify!($default), self.0),
                    }
                }
            }
        )*
    };
}

pub use __consts as consts;

#[macro_export]
macro_rules! __flags {
    (
        $(
            #[examples = [$example0:ident $(, $example:ident)* $(,)?]]
            #[not_set = [$($not_set:ident),* $(,)?]]
            $vis:vis struct $ty:ident($repr:ty) {
                $none_vis:vis const $none:ident;

                $(
                    $(#[$($meta:meta)*])*
                    $flag_vis:vis const $flag:ident = $value:expr;
                )*
            }
        )*
    ) => {
        $(
            #[derive(Clone, Copy, PartialEq, Eq)]
            #[repr(transparent)]
            $vis struct $ty($repr);

            impl $ty {
                /// Empty flags.
                $none_vis const $none: Self = Self(0);

                $(
                    #[doc = concat!("Flag with value `", stringify!($value), "`.")]
                    ///
                    $(#[$($meta)*])*
                    $flag_vis const $flag: Self = Self($value);
                )*

                /// Test if the set contains another set.
                ///
                /// # Examples
                ///
                /// ```
                #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
                ///
                #[doc = concat!(" let flags = ", stringify!($ty), "::", stringify!($example0) $(," | ", stringify!($ty), "::", stringify!($example))*, ";")]
                #[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example0), "));")]
                $(#[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example), "));")])*
                $(#[doc = concat!(" assert!(!flags.contains(", stringify!($ty), "::", stringify!($not_set), "));")])*
                /// ```
                #[inline]
                pub fn contains(self, other: Self) -> bool {
                    (self.0 & other.0) == other.0
                }

                #[doc = concat!(" Convert the flags to a raw ", stringify!($repr), " value.")]
                #[inline]
                $vis fn into_raw(self) -> $repr {
                    self.0
                }

                #[doc = concat!(" Create a new `StreamFlags` from a raw ", stringify!($repr), " value.")]
                #[inline]
                $vis fn from_raw(value: $repr) -> Self {
                    Self(value)
                }

                #[doc = concat!(" Access unknown bits in the flag which carry no meaning.")]
                #[inline]
                $vis fn unknown_bits(&self) -> $repr {
                    self.0 $(& !$value)*
                }
            }

            #[doc = concat!(" Encode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.push(", stringify!($ty), "::", stringify!($example0), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = <$repr as $crate::Encode>::TYPE;

                #[inline]
                fn size(&self) -> usize {
                    <$repr as $crate::Encode>::size(&self.0)
                }

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    <$repr as $crate::Encode>::write_content(&self.0, writer)
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            ///
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example0), ")?;")]
            ///
            #[doc = concat!(" let flags = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flags, ", stringify!($ty), "::", stringify!($example0), ");")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example0), ")?;")]
            ///
            #[doc = concat!(" let flags = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flags, ", stringify!($ty), "::", stringify!($example0), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            /// Unknown representations will be preserved but carry no meaning.
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.as_mut().push(1 | (1 as ", stringify!($repr), ").rotate_right(1))?;")]
            ///
            #[doc = concat!(" let flags = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flags.unknown_bits(), (1 as ", stringify!($repr), ").rotate_right(1));")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::Decode<'de> for $ty {
                const TYPE: $crate::Type = <$repr as $crate::Decode<'de>>::TYPE;

                #[inline]
                fn read_content(reader: impl $crate::Reader<'de, u64>, len: usize) -> Result<Self, $crate::Error> {
                    Ok(Self(<$repr as $crate::Decode<'de>>::read_content(reader, len)?))
                }
            }

            /// Combine two flags with a bitwise or operation.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let flags = ", stringify!($ty), "::", stringify!($example0) $(," | ", stringify!($ty), "::", stringify!($example))*, ";")]
            #[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example0), "));")]
            $(#[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example), "));")])*
            $(#[doc = concat!(" assert!(!flags.contains(", stringify!($ty), "::", stringify!($not_set), "));")])*
            /// ```
            impl core::ops::BitOr for $ty {
                type Output = Self;

                #[inline]
                fn bitor(self, rhs: Self) -> Self::Output {
                    Self(self.0 | rhs.0)
                }
            }

            /// Test if the flags contain another set.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let flags = ", stringify!($ty), "::", stringify!($example0) $(," | ", stringify!($ty), "::", stringify!($example))*, ";")]
            #[doc = concat!(" assert!(flags & ", stringify!($ty), "::", stringify!($example0), ");")]
            $(#[doc = concat!(" assert!(flags & ", stringify!($ty), "::", stringify!($example), ");")])*
            $(#[doc = concat!(" assert!(!(flags & ", stringify!($ty), "::", stringify!($not_set), "));")])*
            /// ```
            impl core::ops::BitAnd for $ty {
                type Output = bool;

                #[inline]
                fn bitand(self, rhs: Self) -> Self::Output {
                    self.contains(rhs)
                }
            }

            /// Assign value to the flags with a bitwise or assign operation.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let mut flags = ", stringify!($ty), "::", stringify!($example0), ";")]
            #[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example0), "));")]
            $(
                #[doc = concat!(" assert!(!flags.contains(", stringify!($ty), "::", stringify!($example), "));")]
                #[doc = concat!(" flags |= ", stringify!($ty), "::", stringify!($example), ");")]
                #[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example), "));")]
            )*
            $(#[doc = concat!(" assert!(!flags.contains(", stringify!($ty), "::", stringify!($not_set), "));")])*
            /// ```
            impl core::ops::BitOrAssign for $ty {
                #[inline]
                fn bitor_assign(&mut self, rhs: Self) {
                    self.0 |= rhs.0;
                }
            }

            #[doc = concat!(" Debug implkementation for `", stringify!($ty), "`.")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let flags = ", stringify!($ty), "::", stringify!($example0) $(, " | ", stringify!($ty), "::", stringify!($example))*, ";")]
            #[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example0), "));")]
            $(#[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example), "));")])*
            $(#[doc = concat!(" assert!(!flags.contains(", stringify!($ty), "::", stringify!($not_set), "));")])*
            ///
            /// let string = format!("{flags:?}");
            #[doc = concat!(" let expected = ", stringify!(concat!(stringify!($ty), "(", stringify!($example0) $(, " | ", stringify!($example))*, ")")), ";")]
            /// assert_eq!(string, expected);
            /// ```
            impl core::fmt::Debug for $ty {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}(", stringify!($ty))?;

                    let mut first = true;
                    let mut value = self.0;

                    let mut write = |flag: &'static str| {
                        if !first {
                            write!(f, " | ")?;
                        }

                        write!(f, "{flag}")?;
                        first = false;
                        Ok(())
                    };

                    $(
                        if value & $value != 0 {
                            write(stringify!($flag))?;
                            value &= !$value;
                        }
                    )*

                    if value > 0 {
                        if !first {
                            write!(f, " | ")?;
                        }

                        write!(f, "0x{:x}", value)?;
                    }

                    write!(f, ")")?;
                    Ok(())
                }
            }
        )*
    }
}

pub use __flags as flags;
