use std::ops::DerefMut;
use std::sync::{Arc, Weak};
use bevy_internal::tasks::TaskPool;
use crate::easy_sockets::{AsyncHandler, Buffer, ErrorAction};
use crate::easy_sockets::spin_lock::SpinLock;

struct SocketManger<B, S> {
    handler: AsyncHandler<(Weak<SpinLock<B>>, Option<S>)>
}

struct BufferUpdateResult {
    write_result: Result<usize, ErrorAction>,
    read_result: Result<usize, ErrorAction>,
    properties_result: Option<ErrorAction>,
    index: usize
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    fn update_buffers(&mut self, pool: &TaskPool) -> Vec<Result<BufferUpdateResult, usize>> {
        self.handler.for_each_async_mut(|index, (weak, optional_socket)| async move {
            if let Some(lock) = weak.upgrade() {
                let mut guard = lock.lock_async().await.unwrap();
                let mut buffer = guard.deref_mut();
                
                if let Some(socket) = optional_socket {
                    
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
        
        for res in results {
            
            match res {
                Ok(UpdateResults) => {
                    if let Some(prop_error) = UpdateResults.properties_result {
                        if prop_error.is_drop() {
                            self.handler.0[UpdateResults.index].1 = None;
                        }
                        //todo log occurrences of recoverable errors
                    }
                    
                    match UpdateResults.read_result {
                        Ok(n) => {
                            //todo log bytes read for diagnostics
                        }
                        Err(a) => {
                            if a.is_drop() {
                                self.handler.0[UpdateResults.index].1 = None;
                            }
                            //todo log occurrences of recoverable errors
                        }
                    }
                    
                    match UpdateResults.write_result {
                        Ok(n) => {
                            //todo log bytes written for diagnostics
                        }
                        Err(error) => {
                            if error.is_drop() {
                                self.handler.0[UpdateResults.index].1 = None;
                            }
                            //todo log occurrences of recoverable errors
                        }
                    }
                }
                Err(index) => {
                    to_be_removed.push(index);
                }
            }
        }
        
        to_be_removed.sort_unstable();
        
        for index in to_be_removed.iter().rev() {
            self.handler.0.remove(*index);
        }
    }
}