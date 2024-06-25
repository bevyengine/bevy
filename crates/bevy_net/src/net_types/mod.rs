use std::sync::{Arc, Weak};
use crate::easy_sockets::spin_lock::SpinLock;

#[cfg(feature = "Tcp")]
pub mod TcpStream {
    use std::collections::VecDeque;
    use std::fmt::{Display, Formatter};
    use std::io;
    use std::io::{ErrorKind, IoSlice, IoSliceMut};
    use std::sync::Arc;
    use async_net::TcpStream;
    use bevy_asset::{AsyncReadExt, AsyncWriteExt};
    use bevy_internal::reflect::List;
    use crate::easy_sockets::{Buffer, ErrorAction, ToByteQueue};
    use crate::easy_sockets::spin_lock::SpinLock;

    pub struct TcpStreamBuffer {
        terminal_error: Option<TcpStreamTerminalError>,
        received_last_tick: usize,
        incoming: VecDeque<VecDeque<u8>>,
        outgoing: VecDeque<VecDeque<u8>>
    }
    
    #[derive(Debug)]
    pub enum TcpStreamTerminalError {
        /// The stream has been terminated
        /// or is otherwise no longer active.
        NotConnected,
        /// The remote server reset the connection.
        Reset,
        ///An unexpected error occurred.
        Unexpected(io::Error)
    }

    impl Display for TcpStreamTerminalError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                TcpStreamTerminalError::NotConnected => f.write_str("Not Connected"),
                TcpStreamTerminalError::Reset =>  f.write_str("Reset"),
                TcpStreamTerminalError::Unexpected(e) => e.fmt(f)
            }
        }
    }

    impl std::error::Error for TcpStreamTerminalError {}

    impl Buffer for TcpStreamBuffer {
        type InnerSocket = TcpStream;
        
        fn build() -> Self {
            todo!()
        }

        async fn fill_read_bufs(&mut self, socket: &mut Self::InnerSocket) -> Result<usize, ErrorAction> {
            let mut vec = Vec::with_capacity(self.received_last_tick * 3);
            match socket.read_to_end(&mut vec).await {
                Ok(read) => {
                    self.received_last_tick = read;
                    vec.shrink_to_fit();
                    Ok(read)
                },
                //the recoverable errors would be handled by the async-io::TcpStream
                //thus anything else is fatal and the error is saved
                Err(io_error) => {
                    self.terminal_error = Some(TcpStreamTerminalError::Unexpected(io_error));
                    Err(ErrorAction::Drop)
                }
            }
        }

        async fn flush_write_bufs(&mut self, socket: &mut Self::InnerSocket) -> Result<usize, ErrorAction> {
            let mut slices = Vec::with_capacity(self.outgoing.len());
            
            for outgoing in &self.outgoing {
                //the first slice will always be empty since
                let (s1, s2) = outgoing.as_slices();
                slices.push(IoSlice::new(s2));
            }
            
            match socket.write_vectored(&*slices).await {
                Ok(n) => {
                    let mut remaining = n;
                    for i in 0..self.outgoing.len() {
                        if remaining >= self.outgoing[i].len() {
                            let removed = self.outgoing.remove(i).unwrap();
                            remaining -= removed.len();
                        } else {
                            self.outgoing[i].drain(..remaining);
                            break
                        }
                    }
                    
                    Ok(n)
                }
                Err(error) => {
                    
                    match error.kind() {

                        //trivial
                        ErrorKind::WriteZero => Err(ErrorAction::None),

                        //non fatal
                        ErrorKind::ConnectionReset => {
                            self.terminal_error = Some(TcpStreamTerminalError::Reset);
                            Err(ErrorAction::Drop)
                        },

                        ErrorKind::NotConnected |
                        ErrorKind::ConnectionAborted |
                        ErrorKind::ConnectionRefused => {
                            self.terminal_error = Some(TcpStreamTerminalError::NotConnected);
                            Err(ErrorAction::Drop)
                        },

                        //fatal error
                        Unepected => {
                            self.terminal_error = Some(TcpStreamTerminalError::Unexpected(error));
                            Err(ErrorAction::Drop)
                        }
                    }
                }
            }
        }

        async fn update_properties(&mut self, socket: &mut Self::InnerSocket) -> Option<ErrorAction> {
            todo!()
        }
    }
}
    
    
