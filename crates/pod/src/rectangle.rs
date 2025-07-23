/// A rectangle defined by its width and height.
///
/// # Examples
///
/// ```
/// use pod::Rectangle;
///
/// let rect1 = Rectangle::new(10, 20);
/// let rect2 = Rectangle::new(20, 10);
/// assert_eq!(rect1, rect1);
/// assert_ne!(rect1, rect2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Rectangle {
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    /// Construct a new rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Rectangle;
    ///
    /// let rect1 = Rectangle::new(10, 20);
    /// let rect2 = Rectangle::new(20, 10);
    /// assert_eq!(rect1, rect1);
    /// assert_ne!(rect1, rect2);
    /// ```
    #[inline]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}
