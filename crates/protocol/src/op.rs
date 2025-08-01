//! Op codes.

pod::macros::consts! {
    constants;

    #[example = GET_REGISTRY]
    #[module = protocol::consts]
    pub struct Core(u8) {
        UNKNOWN;
        /// The first message sent by a client is the Hello message and contains
        /// the version number of the client.
        #[display = "Core::Hello"]
        HELLO = 1;
        /// The Sync message will result in a Done event from the server. When
        /// the Done event is received, the client can be sure that all
        /// operations before the Sync method have been completed.
        #[display = "Core::Sync"]
        SYNC = 2;
        /// Is sent from the client to the server when the server emits the Ping
        /// event. The id and seq should be copied from the Ping event.
        #[display = "Core::Pong"]
        PONG = 3;
        /// A client requests to bind to the registry object and list the
        /// available objects on the server.
        #[display = "Core::GetRegistry"]
        GET_REGISTRY = 5;
        /// Create a new object from a factory of a certain type.
        #[display = "Core::CreateObject"]
        CREATE_OBJECT = 6;
    }

    #[example = GLOBAL]
    #[module = protocol::consts]
    pub struct CoreEvent(u8) {
        UNKNOWN;
        /// Emitted by the server upon connection with the more information
        /// about the server.
        #[display = "Core::Info"]
        INFO = 0;
        /// Emitted as a result of a client Sync method.
        #[display = "Core::Done"]
        DONE = 1;
        /// Is sent from the client to the server when the server emits the Ping
        /// event. The id and seq should be copied from the Ping event.
        #[display = "Core::Ping"]
        PING = 2;
        /// The error event is sent out when a fatal (non-recoverable) error has
        /// occurred. The id argument is the proxy object where the error
        /// occurred, most often in response to a request to that object. The
        /// message is a brief description of the error, for (debugging)
        /// convenience.
        #[display = "Core::Error"]
        ERROR = 3;
        /// This event is used internally by the object ID management logic.
        /// When a client deletes an object, the server will send this event to
        /// acknowledge that it has seen the delete request. When the client
        /// receives this event, it will know that it can safely reuse the
        /// object ID.
        #[display = "Core::RemoveIdEvent"]
        REMOVE_ID_EVENT = 4;
        /// This event is emitted when a local object ID is bound to a global
        /// ID. It is emitted before the global becomes visible in the registry.
        /// This event is deprecated, the BoundProps event should be used
        /// because it also contains extra properties.
        #[display = "Core::BoundId"]
        BOUND_ID = 5;
        /// Memory is given to a client as fd of a certain memory type. Further
        /// references to this fd will be made with the per memory unique
        /// identifier id.
        #[display = "Core::AddMem"]
        ADD_MEM = 6;
        /// Destroy an object.
        #[display = "Core::Destroy"]
        DESTROY = 7;
    }

    #[example = UPDATE_PROPERTIES]
    #[module = protocol::consts]
    pub struct Client(u8) {
        UNKNOWN;
        /// Is used to update the properties of a client.
        #[display = "Client::UpdateProperties"]
        UPDATE_PROPERTIES = 2;
    }

    #[example = ERROR]
    #[module = protocol::consts]
    pub struct ClientEvent(u8) {
        UNKNOWN;
        /// Get client information updates. This is emitted when binding to a
        /// client or when the client info is updated late
        #[display = "Client::Info"]
        INFO = 0;
        /// Is used to send an error to a client.
        #[display = "Client::Error"]
        ERROR = 1;
    }

    #[example = GLOBAL]
    #[module = protocol::consts]
    pub struct RegistryEvent(u8) {
        UNKNOWN;
        /// Notify a client about a new global object.
        #[display = "Registry::Global"]
        GLOBAL = 0;
        /// A global with id was removed.
        #[display = "Registry::GlobalRemove"]
        GLOBAL_REMOVE = 1;
    }

    #[example = UPDATE]
    #[module = protocol::consts]
    pub struct ClientNode(u8) {
        UNKNOWN;
        /// Get the node object associated with the client-node. This binds to
        /// the server side Node object.
        #[display = "ClientNode::GetNode"]
        GET_NODE = 1;
        /// Update the params and info of the node.
        #[display = "ClientNode::Update"]
        UPDATE = 2;
        /// Create, Update or destroy a node port.
        #[display = "ClientNode::PortUpdate"]
        PORT_UPDATE = 3;
        /// Set the node active or inactive.
        #[display = "ClientNode::SetActive"]
        SET_ACTIVE = 4;
    }

    #[example = SET_PARAM_EVENT]
    #[module = protocol::consts]
    pub struct ClientNodeEvent(u8) {
        UNKNOWN;
        /// The server will allocate the activation record and eventfd for the node and
        /// transfer this to the client with the Transport event.
        #[display = "ClientNode::Transport"]
        TRANSPORT = 0;
        /// Set a parameter on the Node.
        #[display = "ClientNode::SetParam"]
        SET_PARAM = 1;
        /// Set an IO area on the node.
        #[display = "ClientNode::SetIo"]
        SET_IO = 2;
        /// Send a command on the node.
        #[display = "ClientNode::Command"]
        COMMAND = 4;
        /// Set a parameter on the Port of the node.
        #[display = "ClientNode::PortSetParam"]
        PORT_SET_PARAM = 7;
        /// Use a set of buffers on the mixer port
        #[display = "ClientNode::UseBuffers"]
        USE_BUFFERS = 8;
        /// Set an IO area on a mixer port.
        #[display = "ClientNode::PortSetIo"]
        PORT_SET_IO = 9;
        /// Notify the client of the activation record of a peer node. This activation
        /// record should be triggered when this node finishes processing.
        #[display = "ClientNode::SetActivation"]
        SET_ACTIVATION = 10;
        /// Notify the node of the peer of a mixer port. This can be used to track the
        /// peer ports of a node.
        #[display = "ClientNode::PortSetMixInfo"]
        PORT_SET_MIX_INFO = 11;
    }
}
