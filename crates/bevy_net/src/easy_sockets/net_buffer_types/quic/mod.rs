use std::collections::vec_deque::{IntoIter, Iter};
use std::collections::VecDeque;
use std::future::Future;
use std::hint::black_box;
use std::iter::{Enumerate, Iterator};
use std::mem;
use std::net::SocketAddr;
use std::task::Context;
use std::time::Duration;
use bytes::Bytes;
use futures::task::SpawnExt;
use quinn::{ConnectError, ConnectionError, OpenUni, ReadError, SendDatagramError, VarInt, WriteError};
use quinn_proto::ConnectionStats;
use static_init::dynamic;
use bevy_reflect::{impl_type_path, TypePath};
use bevy_tasks::{ComputeTaskPool, IoTaskPool, Task};
use bevy_tasks::futures_lite::future::yield_now;
use crate::easy_sockets::{Buffer};
use crate::easy_sockets::socket_manager::{Key, Sockets};

pub mod bevy_quinn;

pub trait ToBytes {
    fn to_bytes(self) -> Bytes;
}

impl<T> ToBytes for T
where T: Into<Bytes> {
    fn to_bytes(self) -> Bytes {
        self.into()
    }
}

pub enum DataSendError {
    /// Datagrams are not supported by the peer
    UnsupportedByPeer,
    /// Datagrams are locally disabled
    Disabled,
    /// The connection was lost
    ConnectionLost(ConnectionError)
}



pub struct Connection {
    inner: quinn::Connection,
    outgoing: VecDeque<Bytes>,
    incoming: VecDeque<Bytes>,
    
    outgoing_result: Result<(), DataSendError>,
    incoming_error: Option<ConnectError>,
}

#[derive(TypePath)]
pub struct RecvStream {
    inner: quinn::RecvStream,
    queue: VecDeque<VecDeque<u8>>,
    total_bytes: usize,
    error: Option<ReceiveError>
}

#[derive(Clone, Debug)]
pub enum ReceiveError {
    /// The connection was reset
    Reset(VarInt),
    /// The connection was lost
    ConnectionLost(ConnectionError),
    /// The stream has been closed or was never connected
    ClosedStream,
    /// Attempted to connect with a 0 rtt connection,
    /// the peer rejected it.
    ZeroRttRejected
}

impl ReceiveError {
    fn from(error: ReadError) -> Self {
        match error {
            ReadError::Reset(i) => {Self::Reset(i)}
            ReadError::ConnectionLost(c) => {Self::ConnectionLost(c)}
            ReadError::ClosedStream => {Self::ClosedStream}
            ReadError::IllegalOrderedRead => {unreachable!()}
            ReadError::ZeroRttRejected => {Self::ZeroRttRejected}
        }
    }
}

impl Drop for RecvStream {
    fn drop(&mut self) {
        let _ = self.inner.stop(VarInt::from(0_u32));
    }
}

impl RecvStream {
    /// Get the last error that occurred on this stream, if any.
    pub fn get_error(&mut self) -> Option<ReceiveError> {
        self.error.take()
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut i = 0;
        while let Some(mut queue) = self.queue.pop_front() {
            while let Some(byte) = queue.pop_front() {
                if i >= self.queue.len() {
                    self.queue.push_front(queue);
                    self.total_bytes -= i;
                    return i
                }
                buf[i] = byte;
                i += 1;
            }
        }
        self.total_bytes -= i;
        return i
    }
    
    /// Shuts down the stream gracefully, awaiting the reset of the stream by the peer
    /// or until the connection is lost. Returns all remaining received data as well as it's result.
    pub fn finish(mut self) -> Task<(VecDeque<VecDeque<u8>>, Result<Option<VarInt>, ResetError>)> {
        IoTaskPool::get().spawn(async move {
            let res = self.inner.received_reset().await;

            (self.queue, res)
        })
    }
}

impl Buffer for RecvStream {
    fn read_from_io(&mut self, target: usize) -> impl Future<Output=Result<usize, ()>> + Send {
        async move {
            let mut buf = Vec::with_capacity(target);
            match self.inner.read(&mut buf).await {
                Ok(op) => {
                    if let Some(n) = op {
                        if n == 0 {
                            return Err(())
                        }
                        if n < target {
                            return Ok(n)
                        } else {
                            return Err(())
                        }
                    }
                    self.error = Some(ReceiveError::ClosedStream);
                    Err(())
                }
                Err(e) => {
                    self.error = Some(ReceiveError::from(e));
                    Err(())
                }
            }
        }
    }

    fn write_to_io(&mut self, target: usize) -> impl Future<Output=Result<usize, ()>> + Send {
        async {Err(())}
    }

    fn additional_updates(&mut self) -> impl Future<Output=()> + Send {
        async {}
    }
}

#[derive(TypePath)]
pub struct SendStream {
    inner: quinn::SendStream,
    queue: VecDeque<Bytes>,
    error: Option<SendError>
}

#[derive(TypePath)]
pub enum SendError {
    Stopped(VarInt),
    ConnectionLost(ConnectionError),
    ZeroRttRejected
}

impl SendError {
    fn from(writeerror: WriteError) -> Self {
        match writeerror {
            WriteError::Stopped(i) => {
                Self::Stopped(i)
            }
            WriteError::ConnectionLost(c) => {
                Self::ConnectionLost(c)
            }
            WriteError::ClosedStream => {
                unreachable!()
            }
            WriteError::ZeroRttRejected => {
                Self::ZeroRttRejected
            }
        }
    }
}


impl Drop for SendStream {
    fn drop(&mut self) {
        let _ = self.inner.finish();
        self.inner.reset(VarInt::from(0_u32));
    }
}

impl SendStream {

    /// Gets the most recent error, if any. All subsequent calls
    /// will result in None unless backend writes have been attempted since then.
    fn handle_error(&mut self) -> Option<SendError> {
        self.error.take()
    }
    fn write(&mut self, bytes: impl ToBytes) {
        self.queue.push_back(bytes.to_bytes())
    }

    /// Shutdown the stream gracefully, sending any remaining queued
    /// data to the recipient.
    fn finish(mut self) {
        IoTaskPool::get().spawn(async move {

            let slice = self.queue.make_contiguous();

            let _ = self.inner.write_all_chunks(slice).await;
        }).detach();
    }

    fn set_priority(&mut self, priority: i32) {
        //unwrapping is safe since
        //errors only occur if the stream has already
        //been closed which is only possible by dropping
        //in our implementation
        self.inner.set_priority(priority).unwrap()
    }

    fn priority(&self) -> i32 {
        //unwrapping is safe since
        //errors only occur if the stream has already
        //been closed which is only possible by dropping
        //in our implementation
        self.inner.priority().unwrap()
    }
}

impl Buffer for SendStream {
    fn read_from_io(&mut self, target: usize) -> impl Future<Output=Result<usize, ()>> + Send {
        async {Err(())}
    }

    fn write_to_io(&mut self, target: usize) -> impl Future<Output=Result<usize, ()>> + Send {
        async move {
            match self.inner.write(&self.queue[0][..]).await {
                Ok(n) => {
                    if n == 0 {
                        return Err(())
                    }
                    if n < target {
                        return Ok(n)
                    } else {
                        return Err(())
                    }
                }
                Err(error) => {
                    self.error = Some(SendError::from(error));
                    Err(())
                }
            }
        }
    }

    fn additional_updates(&mut self) -> impl Future<Output=()> + Send {
        async {}
    }
}

impl Connection {
    pub fn send_data(&mut self, bytes: impl ToBytes) {
        self.outgoing.push_back(bytes.to_bytes());
    }

    pub fn stats(&self) -> ConnectionStats {
        self.inner.stats()
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.inner.remote_address()
    }

    pub fn receive_data(&mut self, buf: &mut [u8]) -> usize {
        todo!()
    }

    pub fn close(&mut self, error_code: VarInt, reason: &[u8]) {
        self.inner.close(error_code, reason)
    }

    pub fn close_reason(&self) -> Option<ConnectionError> {
        self.inner.close_reason()
    }

    pub fn rtt(&self) -> Duration {
        self.inner.rtt()
    }

    pub fn accept_uni(&self) -> Result<RecvStream, ConnectionError> {
        todo!()
    }

    pub fn accept_bi(&self) -> Result<(SendStream, RecvStream), ConnectionError> {
        todo!()
    }

    pub fn open_uni(&self, sends: &mut Sockets<SendStream>) -> Result<Key<SendStream>, ConnectionError>{
        //this should complete quickly
        let res = 
            IoTaskPool::get().scope(
                |s| s.spawn(self.inner.open_uni())).pop().unwrap();
        
        match res {
            Ok(stream) => {
                Ok(sends.register(SendStream {
                    inner: stream,
                    queue: Default::default(),
                    error: None,
                }))
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn open_bi(&self) {
        //this should be completed quickly
        let res =
            IoTaskPool::get().scope(|s|
            s.spawn(self.inner.open_bi())
            ).pop().unwrap();


        todo!()
    }
}