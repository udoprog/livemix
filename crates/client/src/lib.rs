#![no_std]
// TODO: REMOVE THIS ONCE THE CRATE IS READY
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod client;
use self::client::Client;

mod state;
pub use self::state::State;

mod memory;
use self::memory::{Memory, Region};

mod buffer;
use self::buffer::Buffers;

mod ports;
use self::ports::{Port, Ports};

mod activation;
pub use self::activation::Activation;

mod ptr;
