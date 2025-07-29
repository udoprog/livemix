//! Well-known node identifiers used in the protocol.

/// The current version of the native protocol.
pub const VERSION: u32 = 3;

/// The fixed identifier for the core id.
pub const CORE_ID: u32 = 0;

/// The fixed identifier for the client id.
pub const CLIENT_ID: u32 = 1;

/// The current registry version.
pub const REGISTRY_VERSION: u32 = 3;

/// The type of interface factories.
pub const INTERFACE_FACTORY: &str = "PipeWire:Interface:Factory";

/// The type of interface client.
pub const INTERFACE_CLIENT: &str = "PipeWire:Interface:Client";

/// The type of interface node.
pub const INTERFACE_NODE: &str = "PipeWire:Interface:Node";

/// The type of interface port.
pub const INTERFACE_PORT: &str = "PipeWire:Interface:Port";

/// The type of interface link.
pub const INTERFACE_LINK: &str = "PipeWire:Interface:Link";

pod::macros::consts! {
    /// The direction of a port.
    #[example = OUTPUT]
    #[module = protocol::consts]
    pub struct Direction(u32) {
        UNKNOWN;
        INPUT = 0;
        OUTPUT = 1;
    }

    /// Describes `PW_NODE_ACTIVATION_*`.
    #[example = NOT_TRIGGERED]
    #[module = protocol::consts]
    pub struct ActivationStatus(u32) {
        UNKNOWN;
        NOT_TRIGGERED = 0;
        TRIGGERED = 1;
        AWAKE = 2;
        FINISHED = 3;
        INACTIVE = 4;
    }
}
