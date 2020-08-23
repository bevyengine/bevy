use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};

use crate::{SocketAddress, SocketId};
use crate::common::{ListenerId, NetProtocol};
use crate::socket::Socket;

enum ListenerInstance
{
    Udp(UdpSocket),
    Tcp(TcpListener),
}

/// Listener type
/// Supports TCP and UDP connections
pub struct Listener
{
    pub id: ListenerId,
    listener: ListenerInstance,
    connections: Vec<SocketId>,
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

    /// Creates a new listener
    /// Listens on localhost:<port> for incoming connections
    pub fn listen(protocol: NetProtocol, port: u16, listener_id: Option<ListenerId>) -> Result<Self, ()> {
        let id = listener_id.unwrap_or(ListenerId::new());
        let addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), port);
        let connections = vec![];

        // Bind listener to localhost and given port
        match protocol {
            NetProtocol::Udp =>
                {
                    let listener = UdpSocket::bind(addr).map_err(|_| ())?;
                    listener.set_nonblocking(true).map_err(|_| ())?;

                    Ok(Listener {
                        id,
                        connections,
                        listener: ListenerInstance::Udp(listener),
                    })
                }
            NetProtocol::Tcp =>
                {
                    let listener = TcpListener::bind(addr).map_err(|_| ())?;
                    listener.set_nonblocking(true).map_err(|_| ())?;
                    Ok(Listener {
                        id,
                        connections,
                        listener: ListenerInstance::Tcp(listener),
                    })
                }
        }
    }

    /// Accepts incoming requests
    /// Returns a tuple containing the remote address, any data sent during the request (UDP), and the socket (TCP)
    pub fn check_incoming(&mut self) -> Result<(SocketAddress, Option<Socket>, Option<Vec<u8>>), ()> {
        match &self.listener {
            // As UDP is connectionless, we have to receive some data in order to establish a "connection"
            ListenerInstance::Udp(listener) => {
                let mut data = Vec::<u8>::new();
                let connection = listener.recv_from(&mut data).map_err(|_| ())?;

                // Socket::connect(connection.1, NetProtocol::Udp, )

                Ok((connection.1, None, Some(data)))
            }
            ListenerInstance::Tcp(listener) => {
                let connection = listener.accept().map_err(|_| ())?;
                Ok((connection.1, Some(Socket::from_tcp(connection.0)), None))
            }
        }
    }

    /// Check whether a connection error has occurred
    pub fn check_err(&mut self) -> Result<(), ()> {
        match &self.listener {
            ListenerInstance::Udp(s) =>
                if let Ok(e) = s.take_error() {
                    if let Some(_) = e {
                        Err(())
                    } else {
                        Ok(())
                    }
                } else {
                    Err(())
                },
            ListenerInstance::Tcp(s) =>
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

    /// Closes all listener-associated sockets
    pub fn close(&mut self) -> Result<(), ()>
    {
        unimplemented!();
        match &self.listener {
            ListenerInstance::Udp(_) => Ok(()),
            ListenerInstance::Tcp(l) => Ok(()),
        }
    }
}