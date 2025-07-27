use core::fmt;

/// The type of a choice.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ChoiceType(u32);

impl ChoiceType {
    /// Only `child1` is an valid option.
    pub const NONE: Self = Self(0);
    /// In a range, `child1` is a default value, options are between `child2`
    /// and `child3` in the value array.
    pub const RANGE: Self = Self(1);
    /// In a step, `child1` is a default value, options are between `child2` and
    /// `child3`, in steps of child4 in the value array.
    pub const STEP: Self = Self(2);
    /// In an enum, `child1` is a default value, options are any value from the
    /// value array, preferred values come first.
    pub const ENUM: Self = Self(3);
    /// In flags, `child1` is a default value, options are any value from the
    /// value array, preferred values come first.
    pub const FLAGS: Self = Self(4);
}

impl ChoiceType {
    /// Convert the choice into a `u32`.
    #[inline]
    pub(crate) fn into_u32(self) -> u32 {
        self.0
    }

    /// Convert a `u32` into a choice.
    #[inline]
    pub(crate) fn from_u32(value: u32) -> Self {
        ChoiceType(value)
    }
}

impl fmt::Debug for ChoiceType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => write!(f, "None"),
            1 => write!(f, "Range"),
            2 => write!(f, "Step"),
            3 => write!(f, "Enum"),
            4 => write!(f, "Flags"),
            _ => write!(f, "Unknown({})", self.0),
        }
    }
}
