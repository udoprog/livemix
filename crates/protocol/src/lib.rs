#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub(crate) mod macros;

pub(crate) mod error;
pub use self::error::Error;

#[cfg(feature = "std")]
mod connection;
#[cfg(feature = "std")]
pub use self::connection::Connection;

pub mod types;

mod events;

pub mod poll;
pub use self::poll::Poll;

mod event_fd;
pub use self::event_fd::EventFd;

mod timer_fd;
pub use self::timer_fd::TimerFd;

pub mod consts;
pub mod op;

#[cfg(feature = "alloc")]
pub mod ids;

pub mod flags;
pub mod id;

#[cfg(feature = "alloc")]
pub mod buf;

pub mod prop;
pub use self::prop::Prop;

mod properties;
pub use self::properties::Properties;

pub mod ffi;
