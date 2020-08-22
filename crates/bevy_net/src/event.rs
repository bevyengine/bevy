use std::net::SocketAddr;

use crate::common::{SocketId, NetProtocol, ListenerId};

// Socket events

/// An event that is sent whenever a socket should be opened
#[derive(Debug, Clone)]
pub struct OpenSocket {
    pub new_id: SocketId,
    pub remote_address: SocketAddr,
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
    pub data: Vec<u8>
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
    pub recv: Vec<u8>
}

/// An event that is sent whenever a socket has an error
#[derive(Debug, Clone)]
pub struct SocketError {
    pub id: Option<SocketId>,
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

/// An event that is sent whenever a listener should be opened
#[derive(Debug, Clone)]
pub struct OpenListener {
    pub remote_address: SocketAddr,
    pub protocol: NetProtocol
}

/// An event that is sent whenever a listener is opened (connected)
#[derive(Debug, Clone)]
pub struct ListenerOpened {
    pub id: ListenerId,
}

/// An event that is sent whenever a listener should send data
#[derive(Debug, Clone)]
pub struct SendListener {
    pub id: ListenerId,
    pub send: Vec<u8>
}

/// An event that is sent whenever a listener receives data
#[derive(Debug, Clone)]
pub struct ListenerReceive {
    pub id: ListenerId,
    pub recv: Vec<u8>
}

/// An event that is sent whenever a listener has an error
#[derive(Debug, Clone)]
pub struct ListenerError {
    pub id: ListenerId,
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
