use uuid::Uuid;
use std::net::{IpAddr, SocketAddr};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NetId(Uuid);

impl NetId {
    pub fn new() -> Self {
        NetId(Uuid::new_v4())
    }
}

/// Network error type
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NetError
{
    UnknownError,

    OpenError,
    CloseError,

    SendError,
    ReceiveError,

    AcceptError,
    QuitError
}

pub type SocketAddress = SocketAddr;
pub type IpAddress = IpAddr;
pub type Port = u16;

pub type SocketId = NetId;
pub type ListenerId = NetId;

/// Connection protocol
/// TCP has sockets/listeners, but UDP can use sockets/listeners interchangeably
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NetProtocol
{
    /// UDP (User Datagram Protocol)
    Udp,
    /// TCP (Transmission Control Protocol)
    Tcp,
}
