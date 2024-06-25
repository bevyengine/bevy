use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::{Arc, Weak};
use bevy_internal::log::warn;
use bevy_internal::tasks::TaskPool;
use crate::easy_sockets::{AsyncHandler, Buffer, ErrorAction};
use crate::easy_sockets::spin_lock::SpinLock;

struct SocketManger<B, S> {
    handler: AsyncHandler<SocketData<B, S>>
}

struct BufferUpdateResult {
    write_result: Result<usize, ErrorAction>,
    read_result: Result<usize, ErrorAction>,
    properties_result: Option<ErrorAction>,
    index: usize
}

#[cfg(feature = "MetaData")]
struct ByteMetaData {
    /// Consecutive recoverable errors
    /// returned by the buffer during any operations.
    recoverable_errors: u32,

    #[cfg(feature = "Extra-MetaData")]
    /// Consecutive recoverable errors
    /// returned the buffer during read updates.
    recoverable_errors_read: u32,
    #[cfg(feature = "Extra-MetaData")]
    /// Consecutive recoverable errors
    /// returned the buffer during write updates.
    recoverable_errors_write: u32,
    #[cfg(feature = "Extra-MetaData")]
    /// Consecutive recoverable errors
    /// returned the buffer during updates to
    /// miscellaneous properties of the socket.
    recoverable_errors_properties: u32,

    /// Number of bytes written this tick.
    written_this_tick: u32,
    /// Number of bytes read this tick.
    read_this_tick: u32,

    #[cfg(feature = "Extra-MetaData")]
    /// Total number of bytes written to
    /// the port in its lifetime.
    total_written: u64,
    #[cfg(feature = "Extra-MetaData")]
    /// Total number of bytes read from
    /// the port in its lifetime.
    total_read: u64,
}

#[cfg(feature = "MetaData")]
impl ByteMetaData {
    fn update(&mut self, result: BufferUpdateResult) {

        let mut recoverible_error_occured = false;

        if let Ok(n) = result.write_result {
            self.written_this_tick = n as u32;
        } else {
            self.written_this_tick = 0;

            let err = result.write_result.unwrap_err();

            if err.is_none() {
                recoverible_error_occured = true;
                #[cfg(feature = "Extra-MetaData")]
                { self.recoverable_errors_write += 1; }
            } else {
                #[cfg(feature = "Extra-MetaData")]
                { self.recoverable_errors_write = 0; }
            }
        }

        if let Ok(n) = result.read_result {
            self.read_this_tick = n as u32;
        } else {
            self.read_this_tick = 0;

            let err = result.read_result.unwrap_err();

            if err.is_none() {
                recoverible_error_occured = true;
                #[cfg(feature = "Extra-MetaData")]
                { self.recoverable_errors_read += 1; }
            } else {
                #[cfg(feature = "Extra-MetaData")]
                { self.recoverable_errors_read = 0; }
            }
        }

        #[cfg(feature = "Extra-MetaData")]
        if let Some(err) = result.properties_result {
            if err.is_none() {
                recoverible_error_occured = true;
                #[cfg(feature = "Extra-MetaData")]
                { self.recoverable_errors_properties += 1; }
            } else {
                #[cfg(feature = "Extra-MetaData")]
                { self.recoverable_errors_properties = 0; }
            }
        }

        #[cfg(feature = "Extra-MetaData")]
        {
            self.total_written += self.written_this_tick as u64;
            self.total_read += self.read_this_tick as u64;
        }


        if recoverible_error_occured {
            self.recoverable_errors += 1;
        } else {
            self.recoverable_errors += 0;
        }
    }

    fn warn_logging(&self, addr: SocketAddr) {
        todo!()
    }
    
    fn debug_logging(&self, addr: SocketAddr) {
        todo!()
    }
}

struct SocketData<B, S> {
    buffer: Weak<SpinLock<B>>,
    socket: Option<S>,
    #[cfg(feature = "MetaData")]
    meta_data: ByteMetaData
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    fn update_buffers(&mut self, pool: &TaskPool) -> Vec<Result<BufferUpdateResult, usize>> {
        self.handler.for_each_async_mut(|index, data | async move {
            if let Some(lock) = data.buffer.upgrade() {
                let mut guard = lock.lock_async().await.unwrap();
                let mut buffer = guard.deref_mut();
                
                if let Some(socket) = &mut data.socket {
                    
                    return Ok(
                        BufferUpdateResult {
                            write_result: buffer.flush_write_bufs(socket).await,
                            read_result: buffer.fill_read_bufs(socket).await,
                            properties_result: buffer.update_properties(socket).await,
                            index
                        }
                    )
                }
                return Err(index)
            }

            Err(index)
        }, pool)
    }
    
    fn update_and_handle(&mut self, pool: &TaskPool) {
        let results = self.update_buffers(pool);

        let mut to_be_removed = Vec::with_capacity(self.handler.0.len());
        
        for res in &results {
            
            match res {
                Ok(UpdateResults) => {
                    if let Some(prop_error) = UpdateResults.properties_result {
                        if prop_error.is_drop() {
                            self.handler.0[UpdateResults.index].socket = None;
                        }
                    }

                    if let Err(a) = UpdateResults.read_result {
                        if a.is_drop() {
                            self.handler.0[UpdateResults.index].socket = None;
                        }
                    }

                    if let Err(a) = UpdateResults.write_result {
                        if a.is_drop() {
                            self.handler.0[UpdateResults.index].socket = None;
                        }
                    }
                }
                Err(index) => {
                    to_be_removed.push(*index);
                }
            }
        }

        #[cfg(feature = "MetaData")]
        for res in results {
            if let Ok(res) = res {
                let metadata = &mut self.handler.0[res.index].meta_data;
                metadata.update(res);

            }
        }
        
        to_be_removed.sort_unstable();
        
        for index in to_be_removed.iter().rev() {
            self.handler.0.remove(*index);
        }
    }
}