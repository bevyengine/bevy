use std::collections::vec_deque::{IntoIter, Iter};
use std::collections::VecDeque;
use std::future::Future;
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
use bevy_tasks::{ComputeTaskPool, IoTaskPool, Task};
use bevy_tasks::futures_lite::future::yield_now;
use crate::easy_sockets::{Buffer, UpdateResult};

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
    UnsupportedByPeer,
    Disabled,
    ConnectionLost(ConnectionError)
}



pub struct Connection {
    inner: quinn::Connection,
    outgoing: VecDeque<Bytes>,
    incoming: VecDeque<Bytes>,
    
    outgoing_result: Result<(), DataSendError>,
    incoming_error: Option<ConnectError>,
}

pub struct RecvStream {
    inner: quinn::RecvStream,
    queue: VecDeque<VecDeque<u8>>,
    error: Option<ReadError>
}

impl RecvStream {
    pub fn receive_error(&mut self) {
        todo!()
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {


        todo!()
    }
}

pub struct SendStream {
    inner: quinn::SendStream,
    queue: VecDeque<Bytes>,
    error: Option<WriteError>
}

pub enum SendError {
    Stopped(VarInt),
    ConnectionLost(ConnectionError),
    ZeroRttRejected
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
        if let Some(error) = self.error.take() {
            match error {
                WriteError::Stopped(i) => {
                    return Some(SendError::Stopped(i))
                }
                WriteError::ConnectionLost(e) => {
                    return Some(SendError::ConnectionLost(e))
                }
                WriteError::ClosedStream => {
                    //this isn't possible
                    unreachable!()
                }
                WriteError::ZeroRttRejected => {
                    return Some(SendError::ZeroRttRejected)
                }
            }
        }

        None
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

    pub fn open_uni(&self) {
        //this should be completed quickly
        let res =
            IoTaskPool::get().scope(
                |s| s.spawn(self.inner.open_uni())
            ).pop().unwrap();


        todo!()
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