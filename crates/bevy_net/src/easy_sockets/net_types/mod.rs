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
    use bevy_asset::{AsyncReadExt, AsyncWriteExt};
    use bevy_internal::reflect::List;
    use crate::easy_sockets::{Buffer, ErrorAction};
    use crate::easy_sockets::net_types::DeferredSetting;
    use crate::manager;

    pub struct TcpStreamBuffer {
        terminal_error: Option<TcpStreamTerminalError>,
        /// Data received this tick
        received: u64,
        /// Data sent this tick
        sent: u64,
        
        incoming: VecDeque<VecDeque<u8>>,
        outgoing: VecDeque<VecDeque<u8>>,
        
        ttl_settings: DeferredSetting<u32>,
        nodelay_settings: DeferredSetting<bool>,
        
        
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

    //todo
}
    
    
