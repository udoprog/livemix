/// A fraction defined by its numerator and denominator.
///
/// # Examples
///
/// ```
/// use pod::Fraction;
///
/// let rect1 = Fraction::new(10, 20);
/// let rect2 = Fraction::new(20, 10);
/// assert_eq!(rect1, rect1);
/// assert_ne!(rect1, rect2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[non_exhaustive]
pub struct Fraction {
    pub num: u32,
    pub denom: u32,
}

impl Fraction {
    /// Construct a new fraction.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Fraction;
    ///
    /// let rect1 = Fraction::new(10, 20);
    /// let rect2 = Fraction::new(20, 10);
    /// assert_eq!(rect1, rect1);
    /// assert_ne!(rect1, rect2);
    /// ```
    #[inline]
    pub fn new(num: u32, denom: u32) -> Self {
        Self { num, denom }
    }
}
