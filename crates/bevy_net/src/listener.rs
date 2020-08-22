use std::net::{TcpStream, ToSocketAddrs, Shutdown, UdpSocket, SocketAddr, IpAddr, TcpListener};
use std::io::Error;

use super::common::{ListenerId, NetProtocol};
use super::socket::Socket;

enum ListenerInstance
{
    Udp(UdpSocket),
    Tcp(TcpListener),
}

/// Listener type - supports TCP and UDP connections
pub struct Listener
{
    pub id: ListenerId,
    listener: ListenerInstance,
    connections: Vec<Socket>
}

impl Listener
{
    /// Returns listener type
    pub fn get_type(&self) -> NetProtocol
    {
        match self.listener {
            ListenerInstance::Udp(_) => NetProtocol::Udp,
            ListenerInstance::Tcp(_) => NetProtocol::Tcp,
        }
    }

    /// Creates a new UDP listener
    pub fn listen_udp(port: u16) -> Option<Self> {
        // Bind socket to localhost and the same port as the destination
        /*
        match UdpSocket::bind(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), port)) {
            Ok(s) => match s.connect(A) {
                Ok(f) => Some(Listener {
                    id: ListenerId::new(),
                    listener: ListenerInstance::Udp(s),
                    connections: vec![]
                }),
                Err(_) => None
            },
            Err(_) => None
        }
         */
        unimplemented!();
    }

    /// Creates a new TCP listener
    pub fn listen_tcp(port: u16) -> Option<Self> {
        // Bind listener to localhost and given port
        match TcpListener::bind(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), port)) {
            Ok(s) => Some(Listener {
                id: ListenerId::new(),
                listener: ListenerInstance::Tcp(s),
                connections: vec![]
            }),
            Err(_) => None
        }
    }

    /// Returns whether the listener was shutdown successfully
    pub fn close(&mut self) -> bool
    {
        match self.listener {
            ListenerInstance::Udp(_) => false,
            ListenerInstance::Tcp(_) => false,
        }
    }
}