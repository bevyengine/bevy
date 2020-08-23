use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs, UdpSocket};

use crate::{IpAddress, Port, SocketAddress};
use crate::common::{NetProtocol, SocketId};

/// 1MB default buffer size
pub const DEFAULT_BUFFER_SIZE: usize = 1_000_000;

pub trait Socket: Send + Sync
{
    /// Establish a new connection.
    /// Optionally, specify the socket ID, socket port (for UDP, leave empty for a random port), and size of the internal buffer
    fn connect<A: ToSocketAddrs>(address: A,
                                 socket_id: Option<SocketId>,
                                 socket_port: Option<Port>,
                                 buffer_size: Option<usize>) -> Result<Self, ()>
        where Self: Sized;

    /// Send data over the socket
    fn send(&mut self, data: &[u8]) -> Result<usize, ()>;

    /// Retrieve any data received.
    /// Limits the size of the returned buffer to the value of buf_size
    fn receive(&mut self) -> Result<Vec<u8>, ()>;

    /// Check whether a connection error has occurred
    fn check_err(&mut self) -> Result<(), ()>;

    /// Returns whether the socket was shutdown successfully
    fn close(&mut self) -> Result<(), ()>;

    /// Retrieve socket ID
    fn get_id(&self) -> SocketId;
    /// Set socket ID
    fn set_id(&mut self, id: SocketId);

    /// Retrieve maximum internal buffer size (used to store received data)
    fn get_buf_size(&self) -> usize;
    /// Set maximum internal buffer size
    fn set_buf_size(&mut self, buf_size: usize);

    /// Get socket protocol
    fn get_type(&self) -> NetProtocol;
}

/// TCP socket type
pub struct SocketTcp
{
    id: SocketId,
    buf: Vec<u8>,
    socket: TcpStream,
}

impl Socket for SocketTcp
{
    fn connect<A: ToSocketAddrs>(address: A,
                                 socket_id: Option<SocketId>,
                                 _: Option<Port>,
                                 buffer_size: Option<usize>) -> Result<Self, ()> {
        let socket = TcpStream::connect(address).map_err(|_| ())?;
        socket.set_nonblocking(true).map_err(|_| ())?;

        Ok(SocketTcp {
            id: socket_id.unwrap_or(SocketId::new()),
            buf: vec![0; buffer_size.unwrap_or(DEFAULT_BUFFER_SIZE)],
            socket,
        })
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, ()>
    {
        self.socket.write(data).map_err(|_| ())
    }

    fn receive(&mut self) -> Result<Vec<u8>, ()>
    {
        match self.socket.read(&mut self.buf[..]) {
            Ok(c) => Ok(Vec::from(&self.buf[0..c])),
            Err(e) => if e.kind() == ErrorKind::WouldBlock {
                Ok(vec![])
            } else {
                Err(())
            }
        }
    }

    fn check_err(&mut self) -> Result<(), ()>
    {
        if let Ok(e) = self.socket.take_error() {
            if let Some(_) = e {
                Err(())
            } else {
                Ok(())
            }
        } else {
            Err(())
        }
    }

    fn close(&mut self) -> Result<(), ()>
    {
        self.socket.shutdown(Shutdown::Both)
            .map_err(|_| ())
    }

    fn get_id(&self) -> SocketId {
        self.id
    }

    fn set_id(&mut self, id: SocketId) {
        self.id = id;
    }

    fn get_buf_size(&self) -> usize {
        self.buf.len()
    }

    fn set_buf_size(&mut self, buf_size: usize) {
        self.buf = vec![0; buf_size];
    }

    fn get_type(&self) -> NetProtocol {
        NetProtocol::Tcp
    }
}

impl SocketTcp
{
    /// Creates a socket from an existing TCP connection
    pub fn from_existing(socket: TcpStream) -> Self
    {
        Self {
            id: SocketId::new(),
            buf: vec![0; DEFAULT_BUFFER_SIZE],
            socket,
        }
    }
}

/// UDP socket type
pub struct SocketUdp
{
    id: SocketId,
    buf: Vec<u8>,
    socket: UdpSocket,
}

impl Socket for SocketUdp
{
    fn connect<A: ToSocketAddrs>(address: A,
                                 socket_id: Option<SocketId>,
                                 socket_port: Option<Port>,
                                 buffer_size: Option<usize>) -> Result<Self, ()> {
        // Todo - Prevent duplicate (same address) UDP socket connections
        // Todo - Check to make sure no listeners are bound on this port
        // The default port 0 will result in a random port being chosen
        let socket = UdpSocket::bind(
            SocketAddress::new(IpAddress::from([127, 0, 0, 1]),
                               socket_port.unwrap_or(0)))
            .map_err(|_| ())?;

        socket.connect(address).map_err(|_| ())?;
        socket.set_nonblocking(true).map_err(|_| ())?;

        Ok(SocketUdp {
            id: socket_id.unwrap_or(SocketId::new()),
            buf: vec![0; buffer_size.unwrap_or(DEFAULT_BUFFER_SIZE)],
            socket,
        })
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, ()>
    {
        self.socket.send(data).map_err(|_| ())
    }

    fn receive(&mut self) -> Result<Vec<u8>, ()>
    {
        match self.socket.recv(&mut self.buf[..]) {
            Ok(c) => Ok(Vec::from(&self.buf[0..c])),
            Err(e) => if e.kind() == ErrorKind::WouldBlock {
                Ok(vec![])
            } else {
                Err(())
            }
        }
    }

    fn check_err(&mut self) -> Result<(), ()>
    {
        if let Ok(e) = self.socket.take_error() {
            if let Some(_) = e {
                Err(())
            } else {
                Ok(())
            }
        } else {
            Err(())
        }
    }

    fn close(&mut self) -> Result<(), ()>
    {
        Ok(())
    }

    fn get_id(&self) -> SocketId {
        self.id
    }

    fn set_id(&mut self, id: SocketId) {
        self.id = id;
    }

    fn get_buf_size(&self) -> usize {
        self.buf.len()
    }

    fn set_buf_size(&mut self, buf_size: usize) {
        self.buf = vec![0; buf_size];
    }

    fn get_type(&self) -> NetProtocol {
        NetProtocol::Udp
    }
}