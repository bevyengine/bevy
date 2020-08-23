use crate::common::{SocketId, SocketAddress, Port, NetError, NetProtocol, ListenerId};

// Socket events

/// An event that is sent whenever a socket should be opened
/// The `port` field must be specified when using UDP
#[derive(Debug, Clone)]
pub struct OpenSocket {
    pub new_id: SocketId,
    pub remote_address: SocketAddress,
    pub port: Option<Port>,
    pub protocol: NetProtocol
}

/// An event that is sent whenever a socket is opened (connected)
#[derive(Debug, Clone)]
pub struct SocketOpened {
    pub id: SocketId,
}

/// An event that is sent whenever a socket should send data
#[derive(Debug, Clone)]
pub struct SendSocket {
    pub id: SocketId,
    pub tx_data: Vec<u8>
}

/// An event that is sent whenever a socket sent data
#[derive(Debug, Clone)]
pub struct SocketSent {
    pub id: SocketId,
    pub len: usize
}

/// An event that is sent whenever a socket receives data
#[derive(Debug, Clone)]
pub struct SocketReceive {
    pub id: SocketId,
    pub rx_data: Vec<u8>
}

/// An event that is sent whenever a socket has an error
#[derive(Debug, Clone)]
pub struct SocketError {
    pub id: SocketId,
    pub err: NetError
}

/// An event that is sent whenever a socket should be closed
#[derive(Debug, Clone)]
pub struct CloseSocket {
    pub id: SocketId,
}

/// An event that is sent whenever a socket is closed
#[derive(Debug, Clone)]
pub struct SocketClosed {
    pub id: SocketId,
}

// Listener events

/// An event that is sent whenever a listener should be created
#[derive(Debug, Clone)]
pub struct CreateListener {
    pub port: Port,
    pub protocol: NetProtocol
}

/// An event that is sent whenever a listener is opened
#[derive(Debug, Clone)]
pub struct ListenerCreated {
    pub id: ListenerId,
}

/*
/// An event that is sent whenever a listener receives a connection request
#[derive(Debug, Clone)]
pub struct ListenerRequest {
    pub id: ListenerId,
    pub remote_addr: SocketAddress
}

/// An event that is sent whenever a listener should respond to a connection request
#[derive(Debug, Clone)]
pub struct HandleListener {
    pub id: ListenerId,
    pub remote_addr: SocketAddress,
    pub accept: bool
}
*/

/// An event that is sent whenever a listener creates a connection
#[derive(Debug, Clone)]
pub struct ListenerConnected {
    pub id: ListenerId,
    pub socket: SocketId
}

/// An event that is sent whenever a listener has an error
#[derive(Debug, Clone)]
pub struct ListenerError {
    pub id: ListenerId,
    pub err: NetError,
}

/// An event that is sent whenever a listener should be closed
#[derive(Debug, Clone)]
pub struct CloseListener {
    pub id: ListenerId,
}

/// An event that is sent whenever a listener is closed
#[derive(Debug, Clone)]
pub struct ListenerClosed {
    pub id: ListenerId,
}
