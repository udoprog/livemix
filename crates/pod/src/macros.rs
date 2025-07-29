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

            #[doc = concat!(" Encode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.push(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = $crate::Type::ID;
                const SIZE: usize = <u32 as $crate::Encode>::SIZE;

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    $crate::Id(*self).write_content(writer)
                }
            }

            impl $crate::EncodeInto for $ty {
                #[inline]
                fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                    pod.push(self)
                }
            }

            impl<'de> $crate::DecodeFrom<'de> for $ty {
                #[inline]
                fn decode_from(pod: $crate::Pod<impl $crate::Reader<'de, u64>, impl $crate::PodKind>) -> Result<Self, $crate::Error> {
                    pod.next()
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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

        #[cfg(test)]
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
macro_rules! __consts {
    (
        $(
            $(#[doc = $doc:literal])*
            #[example = $example:ident]
            #[module = $module:path]
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
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.push(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = <$repr as $crate::Encode>::TYPE;
                const SIZE: usize = <$repr as $crate::Encode>::SIZE;

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    <$repr as $crate::Encode>::write_content(&self.0, writer)
                }
            }

            impl $crate::EncodeInto for $ty {
                #[inline]
                fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                    pod.push(self)
                }
            }

            impl<'de> $crate::DecodeFrom<'de> for $ty {
                #[inline]
                fn decode_from(pod: $crate::Pod<impl $crate::Reader<'de, u64>, impl $crate::PodKind>) -> Result<Self, $crate::Error> {
                    pod.next()
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            ///
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let flag = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flag, ", stringify!($ty), "::", stringify!($example), ");")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.as_mut().push(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let flag = pod.as_ref().next::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(flag, ", stringify!($ty), "::", stringify!($example), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            #[doc = concat!(" Unknown identifiers will be decoded as the default value ", stringify!($default), ".")]
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            /// pod.as_mut().push(u32::MAX / 2)?;
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
            #[module = $module:path]
            $vis:vis struct $ty:ident($repr:ty) {
                $none_vis:vis const $none:ident;

                $(
                    $(#[doc = $field_doc:literal])*
                    $(#[constant = $flag_mod:ident :: $flag_constant:ident])?
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
                    $(#[doc = $field_doc])*
                    $(
                        #[doc = ""]
                        #[doc = concat!("Equivalent to `", stringify!($flag_constant), "`.")]
                    )*
                    $flag_vis const $flag: Self = Self($value);
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

            #[doc = concat!(" Encode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.push(", stringify!($ty), "::", stringify!($example0), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = <$repr as $crate::Encode>::TYPE;
                const SIZE: usize = <$repr as $crate::Encode>::SIZE;

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    <$repr as $crate::Encode>::write_content(&self.0, writer)
                }
            }

            impl $crate::EncodeInto for $ty {
                #[inline]
                fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                    pod.push(self)
                }
            }

            impl<'de> $crate::DecodeFrom<'de> for $ty {
                #[inline]
                fn decode_from(pod: $crate::Pod<impl $crate::Reader<'de, u64>, impl $crate::PodKind>) -> Result<Self, $crate::Error> {
                    pod.next()
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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
            #[doc = concat!(" use ", stringify!($module), "::", stringify!($ty), ";")]
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

        #[cfg(test)]
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
        impl<$($tt)*> $crate::EncodeInto for $ty
        $(where $($where)*)*
        {
            #[inline]
            fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                pod.push(self)
            }
        }
    };

    ($ty:ty) => {
        impl $crate::EncodeInto for $ty {
            #[inline]
            fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                pod.push(self)
            }
        }
    };
}

pub(crate) use __encode_into_sized as encode_into_sized;

macro_rules! __decode_from_sized {
    (impl [$($tt:tt)*] $ty:ty $(where $($where:tt)*)?) => {
        impl<'de, $($tt)*> $crate::DecodeFrom<'de> for $ty
        $(where $($where)*)*
        {
            #[inline]
            fn decode_from(pod: $crate::Pod<impl $crate::Reader<'de, u64>, impl $crate::PodKind>) -> Result<Self, $crate::Error> {
                pod.next()
            }
        }
    };

    ($ty:ty) => {
        impl<'de> $crate::DecodeFrom<'de> for $ty {
            #[inline]
            fn decode_from(pod: $crate::Pod<impl $crate::Reader<'de, u64>, impl $crate::PodKind>) -> Result<Self, $crate::Error> {
                pod.next()
            }
        }
    };
}

pub(crate) use __decode_from_sized as decode_from_sized;

macro_rules! __decode_from_borrowed {
    ($ty:ty) => {
        impl<'de> $crate::DecodeFrom<'de> for &'de $ty {
            #[inline]
            fn decode_from(
                pod: $crate::Pod<impl $crate::Reader<'de, u64>, impl $crate::PodKind>,
            ) -> Result<Self, $crate::Error> {
                pod.next_borrowed()
            }
        }
    };
}

pub(crate) use __decode_from_borrowed as decode_from_borrowed;

macro_rules! __encode_into_unsized {
    (impl [$($tt:tt)*] $ty:ty $(where $t_key:ident: $($t_bound:tt)*)?) => {
        impl<$($tt)*> $crate::EncodeInto for $ty
        $(
            where
                $t_key: $($t_bound)*
        )*
        {
            #[inline]
            fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                pod.push_unsized(self)
            }
        }
    };

    ($ty:ty) => {
        impl $crate::EncodeInto for $ty {
            #[inline]
            fn encode_into(&self, pod: $crate::Pod<impl $crate::Writer<u64>, impl $crate::PodKind>) -> Result<(), $crate::Error> {
                pod.push_unsized(self)
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
