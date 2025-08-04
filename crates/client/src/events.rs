use protocol::{consts::Direction, id::Param};

use crate::{ClientNodeId, PortId};

/// A parameter for a client node has been set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SetNodeParamEvent {
    pub node_id: ClientNodeId,
    pub param: Param,
}

/// A parameter for the port of a client node has been set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RemoveNodeParamEvent {
    pub node_id: ClientNodeId,
    pub param: Param,
}

/// A parameter for the port of a client node has been set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SetPortParamEvent {
    pub node_id: ClientNodeId,
    pub direction: Direction,
    pub port_id: PortId,
    pub param: Param,
}

/// A parameter for the port of a client node has been removed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RemovePortParamEvent {
    pub node_id: ClientNodeId,
    pub direction: Direction,
    pub port_id: PortId,
    pub param: Param,
}

/// An event produced by a stream about things which might interest a client
/// implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StreamEvent {
    Process(ClientNodeId),
    NodeCreated(ClientNodeId),
    SetNodeParam(SetNodeParamEvent),
    RemoveNodeParam(RemoveNodeParamEvent),
    SetPortParam(SetPortParamEvent),
    RemovePortParam(RemovePortParamEvent),
}
