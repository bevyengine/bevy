use std::sync::{Arc, Weak};
use crate::easy_sockets::spin_lock::SpinLock;

struct DeferredSetting<T> {
    current: T,
    change: Option<T>
}

impl<T: Clone> DeferredSetting<T> {
    fn new(current_value: T) -> Self {
        Self {
            current: current_value,
            change: None
        }
    }
    
    /// Tries to apply the deferred setting.
    /// If no issue or error was encountered while trying 
    /// to apply the setting None is returned.
    /// In this case the internal current value is updated to reflect the change made.
    /// Otherwise no change was made and Some containing the error value is returned,
    /// the intnal state of the structure does not change.
    fn try_apply<F, E>(&mut self, fun: F) -> Result<(), E>
    where F: FnOnce(T) -> Result<(), (T, E)> {
        if let Some(new) = self.change.take() {
            let changed = new.clone();
            match fun(new) {
                Ok(_) => {
                    self.current = changed;
                    return Ok(())
                }
                Err((value, error)) => {
                    self.change = Some(value);
                    return Err(error)
                }
            }
        }
        Ok(())
    }
    
    fn current(&self) -> &T {
        &self.current
    }
    
    fn set_deferred_change(&mut self, new_value: T) {
        self.change = Some(new_value);
    }
}

#[cfg(feature = "Tcp")]
pub mod TcpStream {
    use std::collections::VecDeque;
    use std::fmt::{Display, Formatter};
    use std::io;
    use std::io::{ErrorKind, IoSlice};
    use async_net::TcpStream;
    use bevy_internal::reflect::List;
    use bevy_internal::tasks::futures_lite::{AsyncReadExt, AsyncWriteExt};
    use crate::easy_sockets::{Buffer, ErrorAction, UpdateResult};
    use crate::easy_sockets::net_types::DeferredSetting;

    #[derive(Default)]
    struct TcpStreamDiagnostics {
        written: usize,
        read: usize,
    }

    pub struct TcpStreamBuffer {
        terminal_error: Option<TcpStreamTerminalError>,
        bytes_read_last: usize,

        incoming: VecDeque<VecDeque<u8>>,
        outgoing: VecDeque<VecDeque<u8>>,
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
        type ConstructionError = ();
        type DiagnosticData = TcpStreamDiagnostics;

        fn build(socket: &Self::InnerSocket) -> Result<Self, Self::ConstructionError> {
            Ok(Self {
                terminal_error: None,
                bytes_read_last: 0,
                incoming: Default::default(),
                outgoing: Default::default(),
            })
        }

        async fn fill_read_bufs(&mut self, socket: &mut Self::InnerSocket, data: &mut Self::DiagnosticData) -> UpdateResult {
            let mut bytes = Vec::with_capacity(self.bytes_read_last * 2);
            match socket.read_to_end(&mut bytes).await {
                Ok(n) => {
                    self.bytes_read_last = n;
                    data.read = n;

                    bytes.shrink_to_fit();

                    self.incoming.push_back(bytes.into());

                    Ok(())
                }
                Err(error) => {
                    data.read = 0;
                    self.terminal_error = Some(TcpStreamTerminalError::Unexpected(error));
                    Err(ErrorAction::Drop)
                }
            }
        }

        async fn flush_write_bufs(&mut self, socket: &mut Self::InnerSocket, data: &mut Self::DiagnosticData) -> UpdateResult {
            
            data.written = 0;

            loop {
                let (s1, s2) = self.outgoing[0].as_slices();
                let slices = [IoSlice::new(s1), IoSlice::new(s2)];

                match socket.write_vectored(&slices).await {
                    Ok(n) => {
                        if n == 0 {
                            return Ok(())
                        }
                        
                        data.written += n;

                        let mut remaining = n;

                        if remaining == self.outgoing[0].len() {
                            self.outgoing.pop_front();
                        } else {
                            self.outgoing[0].drain(0..remaining);
                        }
                    }
                    Err(error) => {
                        match error.kind() {
                            ErrorKind::WriteZero => {
                                return Ok(())
                            }

                            ErrorKind::ConnectionRefused |
                            ErrorKind::ConnectionReset |
                            ErrorKind::ConnectionAborted |
                            ErrorKind::NotConnected => {
                                self.terminal_error = Some(TcpStreamTerminalError::NotConnected);
                                return Err(ErrorAction::Drop)
                            }
                            unexpected => {
                                self.terminal_error = Some(TcpStreamTerminalError::Unexpected(error));
                                return Err(ErrorAction::Drop)
                            }
                        }
                    }
                }
            }
        }

        async fn additional_updates(&mut self, socket: &mut Self::InnerSocket, data: &mut Self::DiagnosticData) -> UpdateResult {
            //todo: implement
            Ok(())
        }
    }
}
    
    
