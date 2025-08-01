#[macro_export]
macro_rules! __id {
    (
        $(
            $(#[doc = $doc:literal])*
            #[example = $example:ident]
            #[module = $module:path]
            $ty_vis:vis struct $ty:ident {
                $default:ident
                $(,
                    $(#[doc = $field_doc:literal])*
                    $(#[constant = $field_mod:ident :: $field_constant:ident])?
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
                    $(#[doc = $field_doc])*
                    $(
                        #[doc = ""]
                        #[doc = concat!("Equivalent to `", stringify!($field_constant), "`.")]
                    )*
                    $ty_vis const $field: Self = Self($field_value);
                )*
            }

            #[doc = concat!(" `SizedWritable` implementation for [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.write(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::SizedWritable for $ty {
                const TYPE: $crate::Type = $crate::Type::ID;
                const SIZE: usize = <u32 as $crate::SizedWritable>::SIZE;

                #[inline]
                fn write_sized(&self, writer: impl $crate::Writer) -> Result<(), $crate::Error> {
                    $crate::Id(*self).write_sized(writer)
                }
            }

            impl $crate::Writable for $ty {
                #[inline]
                fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                    pod.next()?.write_sized(self)
                }
            }

            impl<'de> $crate::Readable<'de> for $ty {
                #[inline]
                fn read_from(pod: &mut impl $crate::PodStream<'de>) -> Result<Self, $crate::Error> {
                    pod.next()?.read_sized()
                }
            }

            #[doc = concat!(" `SizedReadable` implementation for [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            ///
            #[doc = concat!(" pod.as_mut().write_sized(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.as_mut().write_sized(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            #[doc = concat!(" Unknown identifiers will be decoded as the default value ", stringify!($default), ".")]
            ///
            /// ```
            /// use pod::{Pod, Id};
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            /// pod.as_mut().write(Id(u32::MAX / 2))?;
            ///
            #[doc = concat!(" let id = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            /// assert!(id.is_invalid());
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::SizedReadable<'de> for $ty {
                #[inline]
                fn read_content(reader: impl $crate::Reader<'de>, ty: $crate::Type, len: usize) -> Result<Self, $crate::Error> {
                    let $crate::Id(id) = $crate::Id::<$ty>::read_content(reader, ty, len)?;
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

        #[cfg(all(test, feature = "test-pipewire-sys"))]
        #[test]
        fn test_constants() {
            $($(
                $(
                    assert_eq! {
                        $ty::$field.into_id(), $field_mod::$field_constant,
                        "{}::{} != {}::{}",
                        stringify!($ty), stringify!($field),
                        stringify!($field_mod), stringify!($field_constant)
                    };
                )*
            )*)*
        }
    };
}

pub use __id as id;

#[macro_export]
macro_rules! __one_of {
    ($fallback:expr, $value:expr) => {
        $value
    };

    ($fallback:expr, ) => {
        $fallback
    };
}

pub use __one_of as one_of;

#[macro_export]
macro_rules! __consts {
    (
        constants;

        $(
            $(#[doc = $doc:literal])*
            #[example = $example:ident]
            #[module = $module:path]
            $ty_vis:vis struct $ty:ident($repr:ty) {
                $default:ident;
                $(
                    $(#[doc = $field_doc:literal])*
                    $(#[display = $display:literal])?
                    $field:ident = $field_value:expr;
                )*
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
                    $(#[doc = $field_doc])*
                    $ty_vis const $field: Self = Self($field_value);
                )*
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

            impl $crate::IntoRaw<$repr> for $ty {
                #[inline]
                fn into_raw(self) -> $repr {
                    $ty::into_raw(self)
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

            impl core::fmt::Display for $ty {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self.0 {
                        $(
                            $field_value => write!(f, "{}", $crate::macros::one_of!(stringify!($field), $($display)*)),
                        )*
                        _ => write!(f, "{}({})", stringify!($default), self.0),
                    }
                }
            }
        )*
    };

    (
        readable_writable;

        $(
            $(#[doc = $doc:literal])*
            #[example = $example:ident]
            #[module = $module:path]
            $ty_vis:vis struct $ty:ident($repr:ty) {
                $default:ident;
                $(
                    $(#[$($field_meta:meta)*])*
                    $field:ident = $field_value:expr;
                )*
            }
        )*
    ) => {
        $(
            #[doc = concat!(" `SizedWritable` implementation for [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.write(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::SizedWritable for $ty {
                const TYPE: $crate::Type = <$repr as $crate::SizedWritable>::TYPE;
                const SIZE: usize = <$repr as $crate::SizedWritable>::SIZE;

                #[inline]
                fn write_sized(&self, writer: impl $crate::Writer) -> Result<(), $crate::Error> {
                    <$repr as $crate::SizedWritable>::write_sized(&self.0, writer)
                }
            }

            impl $crate::Writable for $ty {
                #[inline]
                fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                    pod.next()?.write_sized(self)
                }
            }

            impl<'de> $crate::Readable<'de> for $ty {
                #[inline]
                fn read_from(pod: &mut impl $crate::PodStream<'de>) -> Result<Self, $crate::Error> {
                    pod.next()?.read_sized()
                }
            }

            #[doc = concat!(" `SizedReadable` implementation for [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            ///
            #[doc = concat!(" pod.as_mut().write(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let flag = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flag, ", stringify!($ty), "::", stringify!($example), ");")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.as_mut().write(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let flag = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flag, ", stringify!($ty), "::", stringify!($example), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            #[doc = concat!(" Unknown identifiers will be decoded as the default value ", stringify!($default), ".")]
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            /// pod.as_mut().write(u32::MAX / 2)?;
            ///
            #[doc = concat!(" let id = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            /// assert!(id.is_invalid());
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::SizedReadable<'de> for $ty {
                #[inline]
                fn read_content(reader: impl $crate::Reader<'de>, ty: $crate::Type, len: usize) -> Result<Self, $crate::Error> {
                    Ok(Self(<$repr as $crate::SizedReadable<'de>>::read_content(reader, ty, len)?))
                }
            }
        )*
    };

    ($($tt:tt)*) => {
        $crate::macros::consts!(constants; $($tt)*);
        $crate::macros::consts!(readable_writable; $($tt)*);
    };
}

pub use __consts as consts;

#[macro_export]
macro_rules! __flags {
    (
        $(
            $(#[doc = $ty_doc:literal])*
            #[examples = [$example0:ident $(, $example:ident)* $(,)?]]
            #[not_set = [$($not_set:ident),* $(,)?]]
            #[module = $module:path]
            $vis:vis struct $ty:ident($repr:ty) {
                $(#[doc = $none_doc:literal])*
                $none:ident;

                $(
                    $(#[doc = $field_doc:literal])*
                    $(#[constant = $flag_mod:ident :: $flag_constant:ident])?
                    $flag:ident = $value:expr;
                )*
            }
        )*
    ) => {
        $(
            $(#[doc = $ty_doc])*
            #[derive(Clone, Copy, PartialEq, Eq)]
            #[repr(transparent)]
            $vis struct $ty($repr);

            impl $ty {
                /// Empty flags.
                $(#[doc = $none_doc])*
                $vis const $none: Self = Self(0);

                $(
                    #[doc = concat!("Flag with value `", stringify!($value), "`.")]
                    ///
                    $(#[doc = $field_doc])*
                    $(
                        #[doc = ""]
                        #[doc = concat!("Equivalent to `", stringify!($flag_constant), "`.")]
                    )*
                    $vis const $flag: Self = Self($value);
                )*

                /// Test if the set contains another set.
                ///
                /// # Examples
                ///
                /// ```
                #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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

            #[doc = concat!(" `SizedWritable` implementation for [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.write(", stringify!($ty), "::", stringify!($example0), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::SizedWritable for $ty {
                const TYPE: $crate::Type = <$repr as $crate::SizedWritable>::TYPE;
                const SIZE: usize = <$repr as $crate::SizedWritable>::SIZE;

                #[inline]
                fn write_sized(&self, writer: impl $crate::Writer) -> Result<(), $crate::Error> {
                    <$repr as $crate::SizedWritable>::write_sized(&self.0, writer)
                }
            }

            impl $crate::Writable for $ty {
                #[inline]
                fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                    pod.next()?.write_sized(self)
                }
            }

            impl<'de> $crate::Readable<'de> for $ty {
                #[inline]
                fn read_from(pod: &mut impl $crate::PodStream<'de>) -> Result<Self, $crate::Error> {
                    pod.next()?.read_sized()
                }
            }

            #[doc = concat!(" `SizedReadable` implementation for [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            ///
            #[doc = concat!(" pod.as_mut().write_sized(", stringify!($ty), "::", stringify!($example0), ")?;")]
            ///
            #[doc = concat!(" let flags = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flags, ", stringify!($ty), "::", stringify!($example0), ");")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.as_mut().write_sized(", stringify!($ty), "::", stringify!($example0), ")?;")]
            ///
            #[doc = concat!(" let flags = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flags, ", stringify!($ty), "::", stringify!($example0), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            /// Unknown representations will be preserved but carry no meaning.
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = pod::array();
            #[doc = concat!(" pod.as_mut().write(1 | (1 as ", stringify!($repr), ").rotate_right(1))?;")]
            ///
            #[doc = concat!(" let flags = pod.as_ref().read_sized::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flags.unknown_bits(), (1 as ", stringify!($repr), ").rotate_right(1));")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::SizedReadable<'de> for $ty {
                #[inline]
                fn read_content(reader: impl $crate::Reader<'de>, ty: $crate::Type, len: usize) -> Result<Self, $crate::Error> {
                    Ok(Self(<$repr as $crate::SizedReadable<'de>>::read_content(reader, ty, len)?))
                }
            }

            /// Combine two flags with a bitwise or operation.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let mut flags = ", stringify!($ty), "::", stringify!($example0), ";")]
            #[doc = concat!(" assert!(flags.contains(", stringify!($ty), "::", stringify!($example0), "));")]
            $(
                #[doc = concat!(" assert!(!flags.contains(", stringify!($ty), "::", stringify!($example), "));")]
                #[doc = concat!(" flags |= ", stringify!($ty), "::", stringify!($example), ";")]
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
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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
                    if self.0 == 0 {
                        return write!(f, "{}({})", stringify!($ty), stringify!($none));
                    }

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

        #[cfg(all(test, feature = "test-pipewire-sys"))]
        #[test]
        fn test_constants() {
            $($(
                $(
                    assert_eq! {
                        $ty::$flag.into_raw(), $flag_mod::$flag_constant as $repr,
                        "{}::{} != {}::{}",
                        stringify!($ty), stringify!($flag),
                        stringify!($flag_mod), stringify!($flag_constant)
                    };
                )*
            )*)*
        }
    }
}

pub use __flags as flags;

macro_rules! __encode_into_sized {
    (impl [$($tt:tt)*] $ty:ty $(where $($where:tt)*)?) => {
        impl<$($tt)*> $crate::Writable for $ty
        $(where $($where)*)*
        {
            #[inline]
            fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                pod.next()?.write_sized(self)
            }
        }
    };

    ($ty:ty) => {
        impl $crate::Writable for $ty {
            #[inline]
            fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                pod.next()?.write_sized(self)
            }
        }
    };
}

pub(crate) use __encode_into_sized as encode_into_sized;

macro_rules! __decode_from_sized {
    (impl [$($tt:tt)*] $ty:ty $(where $($where:tt)*)?) => {
        impl<'de, $($tt)*> $crate::Readable<'de> for $ty
        $(where $($where)*)*
        {
            #[inline]
            fn read_from(pod: &mut impl $crate::PodStream<'de>) -> Result<Self, $crate::Error> {
                pod.next()?.read_sized()
            }
        }
    };

    ($ty:ty) => {
        impl<'de> $crate::Readable<'de> for $ty {
            #[inline]
            fn read_from(pod: &mut impl $crate::PodStream<'de>) -> Result<Self, $crate::Error> {
                pod.next()?.read_sized()
            }
        }
    };
}

pub(crate) use __decode_from_sized as decode_from_sized;

macro_rules! __decode_from_borrowed {
    ($ty:ty) => {
        impl<'de> $crate::Readable<'de> for &'de $ty {
            #[inline]
            fn read_from(pod: &mut impl $crate::PodStream<'de>) -> Result<Self, $crate::Error> {
                pod.next()?.read_unsized()
            }
        }
    };
}

pub(crate) use __decode_from_borrowed as decode_from_borrowed;

macro_rules! __encode_into_unsized {
    (impl [$($tt:tt)*] $ty:ty $(where $t_key:ident: $($t_bound:tt)*)?) => {
        impl<$($tt)*> $crate::Writable for $ty
        $(
            where
                $t_key: $($t_bound)*
        )*
        {
            #[inline]
            fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                pod.next()?.write_unsized(self)
            }
        }
    };

    ($ty:ty) => {
        impl $crate::Writable for $ty {
            #[inline]
            fn write_into(&self, pod: &mut impl $crate::PodSink) -> Result<(), $crate::Error> {
                pod.next()?.write_unsized(self)
            }
        }
    };
}

pub(crate) use __encode_into_unsized as encode_into_unsized;

macro_rules! __repeat_tuple {
    ($macro:path) => {
        $macro!(1, A, a);
        $macro!(2, A, a, B, b);
        $macro!(3, A, a, B, b, C, c);
        $macro!(4, A, a, B, b, C, c, D, d);
        $macro!(5, A, a, B, b, C, c, D, d, E, e);
        $macro!(6, A, a, B, b, C, c, D, d, E, e, F, f);
        $macro!(7, A, a, B, b, C, c, D, d, E, e, F, f, G, g);
        $macro!(8, A, a, B, b, C, c, D, d, E, e, F, f, G, g, H, h);
        $macro!(9, A, a, B, b, C, c, D, d, E, e, F, f, G, g, H, h, I, i);
    };
}

pub(crate) use __repeat_tuple as repeat_tuple;
