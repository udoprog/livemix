#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod client;
use self::client::Client;

mod state;
pub use self::state::State;

pub mod ffi;

mod memory;
use self::memory::{Memory, Region};

mod buffer;
use self::buffer::Buffers;

mod ports;
use self::ports::{Port, Ports};
