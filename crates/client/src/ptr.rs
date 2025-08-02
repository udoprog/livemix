use core::ptr::NonNull;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use protocol::{consts, flags};

/// A trait for atomic operations on types.
pub(crate) trait AtomicOps<T> {
    fn store(&self, value: T);

    fn swap(&self, value: T) -> T;

    fn load(&self) -> T;

    fn compare_exchange(&self, current: T, new: T) -> bool;

    fn sub(&self, value: T) -> T;
}

macro_rules! atomic {
    ($($atomic:ident, $repr:ty),* $(,)?) => {
        $(impl AtomicOps<$repr> for $atomic {
            #[inline]
            fn store(&self, value: $repr) {
                $atomic::store(self, value, Ordering::SeqCst);
            }

            #[inline]
            fn swap(&self, value: $repr) -> $repr {
                $atomic::swap(self, value, Ordering::SeqCst)
            }

            #[inline]
            fn load(&self) -> $repr {
                $atomic::load(self, Ordering::SeqCst)
            }

            #[inline]
            fn compare_exchange(&self, current: $repr, new: $repr) -> bool {
                $atomic::compare_exchange(self, current, new, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            }

            #[inline]
            fn sub(&self, value: $repr) -> $repr {
                $atomic::fetch_sub(self, value, Ordering::SeqCst)
            }
        })*
    };
}

atomic! {
    AtomicU32, u32,
    AtomicI32, i32,
}

/// Helper trait to convert values into ones compatible with atomic operations.
pub(crate) trait IntoAtomic {
    type Repr;
    type Atomic: AtomicOps<Self::Repr>;

    fn into_repr(self) -> Self::Repr;

    fn from_repr(repr: Self::Repr) -> Self;
}

impl IntoAtomic for consts::ActivationStatus {
    type Repr = u32;
    type Atomic = AtomicU32;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self.into_raw()
    }

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        Self::from_raw(repr)
    }
}

impl IntoAtomic for flags::Status {
    type Repr = i32;
    type Atomic = AtomicI32;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self.into_raw()
    }

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        Self::from_raw(repr)
    }
}

impl IntoAtomic for u32 {
    type Repr = u32;
    type Atomic = AtomicU32;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self
    }

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        repr
    }
}

/// A pointer to an atomic field.
pub(crate) struct Atomic<T>
where
    T: IntoAtomic,
{
    ptr: NonNull<T::Atomic>,
}

impl<T> Atomic<T>
where
    T: IntoAtomic,
{
    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *const T) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr.cast_mut().cast()) },
        }
    }

    #[inline]
    pub(crate) fn store(&self, value: T) {
        // SAFETY: We are assuming that the pointer is valid and aligned.
        unsafe { (*self.ptr.as_ptr()).store(T::into_repr(value)) }
    }

    #[inline]
    pub(crate) fn swap(&self, value: T) -> T {
        // SAFETY: We are assuming that the pointer is valid and aligned.
        unsafe { T::from_repr((*self.ptr.as_ptr()).swap(T::into_repr(value))) }
    }

    #[inline]
    pub(crate) fn sub(&self, value: T) -> T {
        // SAFETY: We are assuming that the pointer is valid and aligned.
        unsafe { T::from_repr((*self.ptr.as_ptr()).sub(T::into_repr(value))) }
    }

    #[inline]
    pub(crate) fn load(&self) -> T {
        // SAFETY: We are assuming that the pointer is valid and aligned.
        unsafe { T::from_repr((*self.ptr.as_ptr()).load()) }
    }

    #[inline]
    pub(crate) fn compare_exchange(&self, current: T, new: T) -> bool {
        // SAFETY: We are assuming that the pointer is valid and aligned.
        unsafe { (*self.ptr.as_ptr()).compare_exchange(T::into_repr(current), T::into_repr(new)) }
    }
}

/// A field that can be volatilely read.
pub(crate) struct Volatile<T> {
    ptr: NonNull<T>,
}

impl<T> Volatile<T> {
    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *const T) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr.cast_mut()) },
        }
    }

    /// Read the value.
    pub(crate) fn read(&self) -> T {
        // SAFETY: We are assuming that the field pointer is valid.
        unsafe { self.ptr.as_ptr().read_volatile() }
    }

    /// Write a value.
    pub(crate) fn write(&self, value: T) {
        // SAFETY: We are assuming that the field pointer is valid.
        unsafe { self.ptr.as_ptr().write_volatile(value) };
    }

    /// Replace a value and return the old one.
    pub(crate) fn replace(&self, value: T) -> T {
        // SAFETY: We are assuming that the field pointer is valid.
        unsafe {
            let old = self.ptr.as_ptr().read_volatile();
            self.ptr.as_ptr().write_volatile(value);
            old
        }
    }
}

macro_rules! __volatile {
    ($this:expr, $($tt:tt)*) => {
        unsafe {
            $crate::ptr::Volatile::new_unchecked(core::ptr::addr_of!((*$this.as_ptr()).$($tt)*))
        }
    };
}

pub(crate) use __volatile as volatile;

macro_rules! __atomic {
    ($this:expr, $($tt:tt)*) => {
        // SAFETY: We assume that the pointer is valid and aligned.
        unsafe { $crate::ptr::Atomic::new_unchecked(core::ptr::addr_of!((*$this.as_ptr()).$($tt)*)) }
    };
}

pub(crate) use __atomic as atomic;
