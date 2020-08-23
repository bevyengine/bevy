use std::io::ErrorKind;
use std::net::{TcpListener, UdpSocket};

use crate::{IpAddress, Port, SocketAddress, SocketId};
use crate::common::{ListenerId, NetProtocol};
use crate::socket::{Socket, SocketTcp, SocketUdp};

/// 1 MB buffer for UDP "connections"
const DEFAULT_UDP_BUFFER: usize = 1_000_000;

pub trait Listener: Send + Sync
{
    /// Retrieve listener ID
    fn get_id(&self) -> ListenerId;
    /// Set listener ID
    fn set_id(&mut self, id: ListenerId);

    /// Returns listener type
    fn get_type(&self) -> NetProtocol;

    /// Creates a new listener.
    /// Listens on localhost:<port> for incoming connections.
    /// Optionally specify the listener's ID
    fn listen(port: Port, listener_id: Option<ListenerId>) -> Result<Self, ()>
        where Self: Sized;

    /// Check and accept an incoming connection
    fn check_incoming(&mut self) -> Result<Option<Box<dyn Socket>>, ()>;

    /// Check for connection errors
    fn check_err(&mut self) -> Result<(), ()>;

    /// Close the listener (deny new connections)
    fn close(&mut self) -> Result<(), ()>;
}

/// TCP listener
pub struct ListenerTcp
{
    id: ListenerId,
    listener: TcpListener,
    connections: Vec<SocketId>,
    open: bool,
}

impl Listener for ListenerTcp
{
    fn get_id(&self) -> SocketId {
        self.id
    }

    fn set_id(&mut self, id: SocketId) {
        self.id = id;
    }

    fn get_type(&self) -> NetProtocol {
        NetProtocol::Tcp
    }

    fn listen(port: u16, listener_id: Option<ListenerId>) -> Result<Self, ()> where Self: Sized {
        let addr = SocketAddress::new(IpAddress::from([127, 0, 0, 1]), port);
        let listener = TcpListener::bind(addr).map_err(|_| ())?;

        listener.set_nonblocking(true).map_err(|_| ())?;

        Ok(ListenerTcp {
            id: listener_id.unwrap_or(ListenerId::new()),
            connections: vec![],
            open: true,
            listener,
        })
    }

    fn check_incoming(&mut self) -> Result<Option<Box<dyn Socket>>, ()> {
        match self.listener.accept() {
            Ok(connection) => {
                let socket = Box::new(SocketTcp::from_existing(connection.0));
                self.connections.push(socket.get_id());
                Ok(Some(socket))
            }
            Err(err) => if err.kind() == ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(())
            }
        }
    }

    fn check_err(&mut self) -> Result<(), ()> {
        if let Ok(e) = self.listener.take_error() {
            if let Some(_) = e {
                Err(())
            } else {
                Ok(())
            }
        } else {
            Err(())
        }
    }

    fn close(&mut self) -> Result<(), ()> {
        self.open = false;
        Ok(())
    }
}

/// UDP listener type
pub struct ListenerUdp
{
    id: ListenerId,
    listener: UdpSocket,
    connections: Vec<SocketId>,
    buf: Vec<u8>,
    open: bool,
}

impl Listener for ListenerUdp
{
    fn get_id(&self) -> SocketId {
        self.id
    }

    fn set_id(&mut self, id: SocketId) {
        self.id = id;
    }

    fn get_type(&self) -> NetProtocol {
        NetProtocol::Udp
    }

    fn listen(port: u16, listener_id: Option<ListenerId>) -> Result<Self, ()> where Self: Sized {
        let addr = SocketAddress::new(IpAddress::from([127, 0, 0, 1]), port);
        let listener = UdpSocket::bind(addr).map_err(|_| ())?;
        listener.set_nonblocking(true).map_err(|_| ())?;

        Ok(ListenerUdp {
            id: listener_id.unwrap_or(ListenerId::new()),
            connections: vec![],
            buf: vec![0; DEFAULT_UDP_BUFFER],
            open: true,
            listener,
        })
    }

    /// For UDP, this function will create a new socket on a new port.
    /// If you wish to use the same port (meaning no new socket) please call the `read_incoming` method instead
    fn check_incoming(&mut self) -> Result<Option<Box<dyn Socket>>, ()> {
        // As UDP is connectionless, we have to receive some data in order to establish a "connection"
        match self.listener.peek_from(&mut self.buf) {
            Ok(connection) => {
                let socket = Box::new(SocketUdp::connect(connection.1,
                                                         None,
                                                         None,
                                                         Some(DEFAULT_UDP_BUFFER))?);
                self.connections.push(socket.get_id());
                Ok(Some(socket))
            }
            Err(err) => if err.kind() == ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(())
            }
        }
    }

    fn check_err(&mut self) -> Result<(), ()> {
        if let Ok(e) = self.listener.take_error() {
            if let Some(_) = e {
                Err(())
            } else {
                Ok(())
            }
        } else {
            Err(())
        }
    }

    fn close(&mut self) -> Result<(), ()> {
        self.open = false;
        Ok(())
    }
}

impl ListenerUdp
{
    /// Read incoming data without opening a new connection.
    /// Returns the data read and the remote address
    pub fn read_incoming(&mut self) -> Result<Option<(&Vec<u8>, SocketAddress)>, ()>
    {
        match self.listener.peek_from(&mut self.buf) {
            Ok(connection) => Ok(Some((&self.buf, connection.1))),
            Err(err) => if err.kind() == ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(())
            }
        }
    }
}