use bevy_ecs::Bundle;

use std::net::{TcpStream, ToSocketAddrs, Shutdown, UdpSocket, SocketAddr, IpAddr};
use std::io::{Error, Write};
use ringbuf::RingBuffer;

use crate::common::{SocketId, NetProtocol};

const DEFAULT_BUFFER_SIZE: usize = 1024;

enum SocketInstance
{
    Udp(UdpSocket),
    Tcp(TcpStream),
}

/// Socket type - supports TCP and UDP connections
#[derive(Bundle)]
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

    /// Creates a new UDP socket
    /// Optionally, specify the size of the internal buffer
    pub fn connect_udp<A: ToSocketAddrs>(address: A, buf_size: Option<usize>) -> Option<Self> {
        // Bind address to localhost and the same port as the destination
        match UdpSocket::bind(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), address.to_socket_addrs().unwrap().next().unwrap().port())) {
            Ok(s) => match s.connect(address) {
                Ok(f) => Some(Socket {
                    id: SocketId::new(),
                    sock: SocketInstance::Udp(s),
                    buf_size: buf_size.unwrap_or(DEFAULT_BUFFER_SIZE)
                }),
                Err(_) => None
            },
            Err(_) => None
        }
    }

    /// Creates a new TCP socket
    pub fn connect_tcp<A: ToSocketAddrs>(address: A, buf_size: Option<usize>) -> Option<Self> {
        match TcpStream::connect(address) {
            Ok(s) => Some(Socket {
                id: SocketId::new(),
                sock: SocketInstance::Tcp(s),
                buf_size: buf_size.unwrap_or(DEFAULT_BUFFER_SIZE)
            }),
            Err(_) => None
        }
    }

    /// Send data over the socket
    /// Returns number of bytes written
    pub fn send(&mut self, data: &[u8]) -> Result<usize, ()>
    {
        (match & mut self.sock {
            SocketInstance::Udp(s) => s.send(data),
            SocketInstance::Tcp(s) => s.write(data),
        }).map_err(|_| ())
    }

    /// Retrieve any data on the socket
    /// Returns number of bytes read
    pub fn recv(&mut self, data: &mut [u8]) -> Result<usize, ()>
    {
        unimplemented!();
        Err(())
    }

    /// Returns whether the socket was shutdown successfully
    pub fn close(&mut self) -> bool
    {
        match &self.sock {
            SocketInstance::Udp(_) => true,
            SocketInstance::Tcp(stream) => stream.shutdown(Shutdown::Both)
                .map_or(false, |_| true),
        }
    }
}