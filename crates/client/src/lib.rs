#![no_std]
// TODO: REMOVE THIS ONCE THE CRATE IS READY
#![allow(unused)]
#![allow(clippy::enum_variant_names)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod client;
use self::client::Client;

mod stream;
pub use self::stream::Stream;

pub mod memory;
use self::memory::{Memory, Region};

mod buffer;
use self::buffer::Buffers;

mod client_node;
pub use self::client_node::{ClientNode, ClientNodeId, ClientNodes};

mod ports;
pub use self::ports::{MixId, Port, PortId, PortParam, Ports};

mod activation;
pub use self::activation::PeerActivation;

pub mod events;
pub mod ptr;
pub mod utils;

mod stats;
pub use self::stats::Stats;

mod parameters;
pub use self::parameters::Parameters;

mod id;
pub use self::id::{GlobalId, LocalId};
