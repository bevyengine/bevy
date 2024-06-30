#[cfg(feature = "Tcp")]
pub mod tcp_stream {
    use std::collections::vec_deque::Iter;
    use std::collections::VecDeque;
    use std::fmt::{Display, Formatter};
    use std::io;
    use std::io::{ErrorKind, IoSlice};
    use std::sync::Mutex;
    use static_init::dynamic;
    use bevy_internal::reflect::List;
    use bevy_internal::tasks::futures_lite::{AsyncReadExt, AsyncWriteExt, StreamExt};
    use crate::easy_sockets::{Buffer, ErrorAction, UpdateResult};
    use crate::easy_sockets::plugin::PLUGIN_INIT;
    use crate::easy_sockets::socket_manager::{OwnedBuffer, SocketManger};
    use crate::easy_sockets::spin_lock::SpinLockGuard;

    pub struct PeakIter<'a> {
        guard: SpinLockGuard<'a, TcpStreamBuffer>,
        outer_iter: Iter<'a, VecDeque<u8>>,
        inner_iter: Option<Iter<'a, u8>>
    }
    
    #[test]
    fn test() {
        
    }
    
    impl<'a> PeakIter<'a> {
        fn new(stream: &'a TcpStream) -> Self {
            let guard = stream.0.lock().unwrap();
            let iter = guard.incoming.iter();
            
            Self {
                guard: guard,
                outer_iter: iter,
                inner_iter: None,
            }
        }
    }
    
    impl<'a> Iterator for PeakIter<'a> {
        type Item = u8;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(inner_iter) = &mut self.inner_iter {
                if let Some(byte) = inner_iter.next() {
                    return Some(*byte)
                }
            }
            
            
            if let Some(new_vec) = self.outer_iter.next() {
                self.inner_iter = Some(new_vec.iter());

                //should always be some
                if let Some(byte) = self.inner_iter.as_mut().unwrap().next() {
                    return Some(*byte)
                }
            }
            
            None
        }
    }
    
    pub struct TcpStream(OwnedBuffer<TcpStreamBuffer>);
    
    impl TcpStream {
        pub fn peak_iter<'a>(&'a self) -> PeakIter<'a> {
            PeakIter::new(self)
        } 
    }
    
    struct TcpStreamManager(Mutex<SocketManger<TcpStreamBuffer, async_net::TcpStream>>);
    
    impl TcpStreamManager {
        pub fn register(&self, stream: async_net::TcpStream) -> Option<OwnedBuffer<TcpStreamBuffer>> {
            if PLUGIN_INIT.is_init() {
                let mut inner = self.0.lock().unwrap();

                return Some(inner.register(stream).unwrap())
            }
            None
        }
        
        pub fn get() -> &'static Self {
            &MANAGER
        }
    }

    #[dynamic]
    static MANAGER: TcpStreamManager = TcpStreamManager(Mutex::new(SocketManger::new()));

    #[derive(Default)]
    struct TcpStreamDiagnostics {
        written: usize,
        read: usize,
    }

    struct TcpStreamBuffer {
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
        type InnerSocket = async_net::TcpStream;
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
    
    
