#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub(crate) mod error;
pub use self::error::Error;

#[cfg(feature = "std")]
mod connection;
#[cfg(feature = "std")]
pub use self::connection::Connection;

#[cfg(feature = "alloc")]
pub(crate) mod dynamic_buf;
#[cfg(feature = "alloc")]
pub use self::dynamic_buf::DynamicBuf;

pub mod types;

mod events;

pub mod poll;
pub use self::poll::Poll;

mod event_fd;
pub use self::event_fd::EventFd;

pub mod consts;
pub mod op;

#[cfg(feature = "alloc")]
pub mod ids;

pub mod flags;
pub mod id;
