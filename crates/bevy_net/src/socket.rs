use std::io::{Error, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs, UdpSocket};

use bevy_ecs::Bundle;

use crate::{IpAddress, Port, SocketAddress};
use crate::common::{NetProtocol, SocketId};

/// 1MB default buffer size
pub const DEFAULT_BUFFER_SIZE: usize = 1_000_000;

#[derive(Debug)]
enum SocketInstance
{
    Udp(UdpSocket),
    Tcp(TcpStream),
}

/// Socket type - supports TCP and UDP connections
pub struct Socket
{
    pub id: SocketId,
    pub buf_size: usize,
    sock: SocketInstance,
}

impl Socket
{
    /// Returns socket type
    pub fn get_type(&self) -> NetProtocol
    {
        match self.sock {
            SocketInstance::Udp(_) => NetProtocol::Udp,
            SocketInstance::Tcp(_) => NetProtocol::Tcp,
        }
    }

    /// Creates a socket from an existing TCP connection
    pub fn from_tcp(socket: TcpStream) -> Self
    {
        Self {
            id: SocketId::new(),
            buf_size: DEFAULT_BUFFER_SIZE,
            sock: SocketInstance::Tcp(socket)
        }
    }

    /// Creates a new socket
    /// Optionally, specify the socket ID, socket port (for UDP, leave empty for a random port), and size of the internal buffer
    pub fn connect<A: ToSocketAddrs>(address: A,
                                     protocol: NetProtocol,
                                     socket_id: Option<SocketId>,
                                     socket_port: Option<Port>,
                                     buffer_size: Option<usize>) -> Result<Self, ()> {
        let buf_size = buffer_size.unwrap_or(DEFAULT_BUFFER_SIZE);
        let id = socket_id.unwrap_or(SocketId::new());

        match protocol {
            NetProtocol::Udp => {
                // Todo - Prevent duplicate (same address) UDP socket connections
                // Todo - Check to make sure no listeners are bound on this port
                // The default port 0 will result in a random port being chosen
                let sock = UdpSocket::bind(
                    SocketAddress::new(IpAddress::from([127, 0, 0, 1]),
                                       socket_port.unwrap_or(0)))
                    .map_err(|_| ())?;

                sock.connect(address).map_err(|_| ())?;
                sock.set_nonblocking(true).map_err(|_| ())?;

                Ok(Socket {
                    id,
                    buf_size,
                    sock: SocketInstance::Udp(sock),
                })
            }
            NetProtocol::Tcp => {
                let sock = TcpStream::connect(address).map_err(|_| ())?;
                sock.set_nonblocking(true).map_err(|_| ())?;
                Ok(Socket {
                    id,
                    buf_size,
                    sock: SocketInstance::Tcp(sock),
                })
            }
        }
    }

    /// Send data over the socket
    /// Returns number of bytes written
    pub fn send(&mut self, data: &[u8]) -> Result<usize, ()>
    {
        match &mut self.sock {
            SocketInstance::Udp(s) => s.send(data),
            SocketInstance::Tcp(s) => s.write(data),
        }.map_err(|_| ())
    }

    /// Retrieve any data on the socket
    /// Limits the size of the returned buffer to the value of buf_size
    pub fn recv(&mut self) -> Result<Vec<u8>, ()>
    {
        let mut d = Vec::<u8>::new();
        d.reserve(self.buf_size);

        match &mut self.sock {
            SocketInstance::Udp(sock) => sock.recv(&mut d[0..self.buf_size]),
            SocketInstance::Tcp(stream) => stream.read(&mut d[0..self.buf_size])
        }.map_err(|_| ()).map(|_| d)
    }

    /// Check whether a connection error has occurred
    pub fn check_err(&mut self) -> Result<(), ()>
    {
        match &self.sock {
            SocketInstance::Udp(s) =>
                if let Ok(e) = s.take_error() {
                    if let Some(_) = e {
                        Err(())
                    } else {
                        Ok(())
                    }
                } else {
                    Err(())
                },
            SocketInstance::Tcp(s) =>
                if let Ok(e) = s.take_error() {
                    if let Some(_) = e {
                        Err(())
                    } else {
                        Ok(())
                    }
                } else {
                    Err(())
                },
        }
    }

    /// Returns whether the socket was shutdown successfully
    pub fn close(&mut self) -> Result<(), ()>
    {
        match &self.sock {
            SocketInstance::Udp(_) => Ok(()),
            SocketInstance::Tcp(stream) => stream.shutdown(Shutdown::Both)
                .map_err(|_| ()),
        }
    }
}