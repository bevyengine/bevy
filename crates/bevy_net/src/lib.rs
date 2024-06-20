use std::collections::VecDeque;
use std::io;
use std::io::{Error, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use bevy_internal::tasks::IoTaskPool;

pub mod async_ports;


const UDP_MAX_LENGTH: usize = 65507;

struct UdpBuffer {
    socket: UdpSocket,
    buffer: [u8; UDP_MAX_LENGTH],
    buf_start: usize
}

impl Read for UdpBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut i = 0;

        //todo improve efficacy when reading into long buffer by avoid a redundent copy into self.buffer
        loop {
            for byte in self.buffer[self.buf_start..].iter() {
                if let Some(ref_) = buf.get_mut(i) {
                    *ref_ = *byte;
                } else {
                    return Ok(i + 1)
                }

                i += 1;
                self.buf_start += 1;
            }

            let read = self.socket.recv(&mut self.buffer)?;

            if read == 0 {
                return Ok(i)
            }

            self.buf_start = UDP_MAX_LENGTH - read;
        }
    }
}

trait ConnectRequest: Sized {

    type Connection: Connection;

    /// Accept this incoming connection.
    /// If the connection process can not be completed
    /// before max_wait expiers, then the function should return
    /// an ErrorType::Other to indicate as such.
    async fn accept(self, max_wait: Duration) -> Result<Self::Connection, ErrorType<()>>;

    /// Deny the request, sending a message that
    /// may be used for debugging and logging.
    async fn deny(self,
        message: <<Self as ConnectRequest>::Connection as Connection>::Signal
    ) -> Result<(), io::Error>;
}

trait Listener: Sized {
    type Request: ConnectRequest;

    async fn bind(port: impl ToSocketAddrs) -> Result<Self, io::Error>;

    /// Checks for incoming connections on the port, if one is available
    /// then return Some containing that connection, pulling it off of the queue.
    async fn get_incoming(&mut self) -> Option<<<Self as Listener>::Request as ConnectRequest>::Connection>;
}


enum ErrorType<E> {
    Io(io::Error),
    Other(E)
}

/// This trait represents a custom 
/// connection-based networking protocol.
trait Connection: Sized {
    type Signal: Default;

    type ByteHandler: ByteHandler;
    
    type Deserializer<'a>: Deserializer<'a>;
    
    type Error;
    
    async fn request_connect(
        local_addr: impl ToSocketAddrs,
        remote_addr: impl ToSocketAddrs,
        response_timeout: Option<Duration>,
        connection_timeout: Option<Duration>
    ) -> Result<Self, ErrorType<Self::Error>>;

    async fn send<T: Serialize>(&mut self, data: T) -> Result<(), io::Error>;

    /// If the type is deserialized successfully the value is returned,
    /// otherwise an error value is returned. The only reason this method should
    /// fail should be an issue with type deserialization or an io error.
    async fn receive<'a, T: Deserialize<'a>>(&mut self) -> Result<T, ErrorType<<<Self as Connection>::Deserializer<'a> as Deserializer<'a>>::Error>>;

    /// Returns the local port used for this connection.
    fn local_addr(&self) -> SocketAddr;

    /// Returns the remote port used for this connection.
    fn remote_addr(&self) -> SocketAddr;

    /// Indicates weather the connection is still alive or not.
    /// A connection is considered not alive when the timeout duration
    /// has passed since any packets have been received from the remote end.
    /// However, some implementations may have other conditions that kill the connection.
    /// Once a connection is dead no further data is accepted from the remote end,
    /// any data that was received when it was alive will remain accessible.
    fn is_alive(&self) -> bool;

    /// Effectively the same as dropping self expect a non-default final
    /// message can be sent to the remote end to indicate the exact
    /// reason for closing the connection.
    ///
    /// Implementers of this trait should also implement drop
    /// and call close with a default message
    /// indicating the connection was closed as such.
    async fn close(self, message: Self::Signal);
}

trait ByteHandler<const CHUNK_SIZE: usize> {
    type Error;
    
    const BYTE_ALIGNMENT: usize;
    
    fn parse_incoming(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error>;

    fn parse_outgoing(&mut self, bytes: &mut [u8]);
}

struct Foo;





