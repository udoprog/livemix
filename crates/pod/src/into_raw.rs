/// Convert a type into a raw representation.
pub trait IntoRaw<T> {
    /// Convert the value into a raw representation.
    fn into_raw(self) -> T;
}
