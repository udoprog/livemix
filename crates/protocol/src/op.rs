//! Op codes.

/// Get client information updates. This is emitted when binding to a client or
/// when the client info is updated late
pub const CLIENT_INFO: u8 = 0;

/// Is used to send an error to a client.
pub const CLIENT_ERROR: u8 = 1;

/// Is used to update the properties of a client.
pub const CLIENT_UPDATE_PROPERTIES: u8 = 2;

/// The first message sent by a client is the Hello message and contains the
/// version number of the client.
pub const CORE_HELLO: u8 = 1;

/// The Sync message will result in a Done event from the server. When the Done
/// event is received, the client can be sure that all operations before the
/// Sync method have been completed.
pub const CORE_SYNC: u8 = 2;

/// Is sent from the client to the server when the server emits the Ping event.
/// The id and seq should be copied from the Ping event.
pub const CORE_PONG: u8 = 3;

/// Emitted by the server upon connection with the more information about the
/// server.
pub const CORE_INFO_EVENT: u8 = 0;

/// Emitted as a result of a client Sync method.
pub const CORE_DONE_EVENT: u8 = 1;

/// Is sent from the client to the server when the server emits the Ping event.
/// The id and seq should be copied from the Ping event.
pub const CORE_PING_EVENT: u8 = 2;

/// The error event is sent out when a fatal (non-recoverable) error has
/// occurred. The id argument is the proxy object where the error occurred, most
/// often in response to a request to that object. The message is a brief
/// description of the error, for (debugging) convenience.
pub const CORE_ERROR_EVENT: u8 = 3;

/// A client requests to bind to the registry object and list the available
/// objects on the server.
pub const CORE_GET_REGISTRY: u8 = 5;

/// Create a new object from a factory of a certain type.
pub const CORE_CREATE_OBJECT: u8 = 6;

/// This event is used internally by the object ID management logic. When a
/// client deletes an object, the server will send this event to acknowledge
/// that it has seen the delete request. When the client receives this event, it
/// will know that it can safely reuse the object ID.
pub const CORE_REMOVE_ID_EVENT: u8 = 4;

/// This event is emitted when a local object ID is bound to a global ID. It is
/// emitted before the global becomes visible in the registry. This event is
/// deprecated, the BoundProps event should be used because it also contains
/// extra properties.
pub const CORE_BOUND_ID_EVENT: u8 = 5;

/// Memory is given to a client as fd of a certain memory type. Further
/// references to this fd will be made with the per memory unique identifier id.
pub const CORE_ADD_MEM_EVENT: u8 = 6;

/// Notify a client about a new global object.
pub const REGISTRY_GLOBAL_EVENT: u8 = 0;

/// The server will allocate the activation record and eventfd for the node and
/// transfer this to the client with the Transport event.
pub const CLIENT_NODE_TRANSPORT_EVENT: u8 = 0;

/// Set an IO area on the node.
pub const CLIENT_NODE_SET_IO_EVENT: u8 = 2;

/// Notify the client of the activation record of a peer node. This activation
/// record should be triggered when this node finishes processing.
pub const CLIENT_NODE_SET_ACTIVATION_EVENT: u8 = 10;
