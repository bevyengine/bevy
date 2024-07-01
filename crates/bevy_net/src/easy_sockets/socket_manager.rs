use std::future::{Future, IntoFuture};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{Duration, Instant};
use futures::future::join_all;
use bevy_tasks::{IoTaskPool, TaskPool};
use crate::easy_sockets::{Buffer, ErrorAction, UpdateResult};
use crate::easy_sockets::spin_lock::{SpinLock, SpinLockGuard};

/// A wrapper type around Arc<SpinLock<T>>.
/// It's used to ensure the arc 
/// isn't cloned which could cause 
/// incorrectness.
pub struct OwnedBuffer<T>(Arc<SpinLock<T>>);

impl<T> OwnedBuffer<T> {
    fn new_with_weak(inner: T) -> (Weak<SpinLock<T>>, Self) {
        let new = Self(Arc::new(SpinLock::new(inner)));
        let weak = Arc::downgrade(&new.0);

        (weak, new)
    }
}

impl<T> Deref for OwnedBuffer<T> {
    type Target = SpinLock<T>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

struct BufferUpdateResult {
    write_result: UpdateResult,
    read_result: UpdateResult,
    additional_result: UpdateResult
}

struct UpdateResults {
    results: Result<Option<BufferUpdateResult>, ()>,
    index: usize
}

struct SocketEntry<B, S>
where B: Buffer<InnerSocket = S> {
    buffer: Weak<SpinLock<B>>,
    socket: Option<S>,
    data: B::DiagnosticData,
    active_duration: Duration,
    drop_flag: bool
}

impl<B, S> SocketEntry<B, S>
where B: Buffer<InnerSocket = S> {

    /// Returns Ok if the Buffer is still in scope and
    /// if the socket is also still present and
    /// was updated.
    /// Returns Err() if either the buffer or socket are not present.
    async fn try_update_buffer(&mut self) -> Result<BufferUpdateResult, ()> {
        if let Some(buffer) = self.buffer.upgrade() {
            if let Some(socket) = &mut self.socket {
                let mut guard = buffer.lock_async().await.unwrap();
                
                return Ok(BufferUpdateResult {
                    write_result: guard.flush_write_bufs(socket, &mut self.data).await,
                    read_result: guard.fill_read_bufs(socket, &mut self.data).await,
                    additional_result: guard.additional_updates(socket, &mut self.data).await,
                })

            }
            return Err(())
        }

        return Err(())
    }

    async fn update(&mut self) {
        match self.try_update_buffer().await {
            Ok(update_results) => {
                let mut should_drop_socket = false;
                let mut error_occured = false;

                if let Err(action) = update_results.write_result {
                    error_occured = true;
                    if action.is_drop() {
                        should_drop_socket = true;
                    }
                }

                if let Err(action) = update_results.read_result {
                    error_occured = true;
                    if action.is_drop() {
                        should_drop_socket = true;
                    }
                }

                if should_drop_socket {
                    self.socket = None;
                }
            }
            Err(_) => {
                self.drop_flag = true;
            }
        }
    }
}

pub struct SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    sockets: Vec<SocketEntry<B, S>>,
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    pub fn new() -> Self {
        Self {
            sockets: vec![],
        }
    }
    
    async fn update(&mut self) {

        #[cfg(not(feature = "multithreaded"))]
        {
            join_all(self.sockets.iter_mut().map(|entrey| async move {
                entrey.update().await;
                entrey
            })).await;

            let mut i = self.sockets.len() + 1;

            while i != 0 {
                if self.sockets[i - 1].drop_flag {
                    self.sockets.remove(i - 1);
                }
                i -= 1;
            }
        }

        #[cfg(feature = "multithreaded")]
        {
            let mut tasks = Vec::with_capacity(self.sockets.len());
            
            while let Some(mut entrey) = self.sockets.pop() {
                tasks.push(async move {
                    entrey.update();
                    entrey
                })
            }
            
            for task in tasks {
                let upated = task.await;
                
                if !upated.drop_flag {
                    self.sockets.push(upated)
                }
            }
        }
    }
    
    pub fn register(&mut self, socket: S) -> Result<OwnedBuffer<B>, (S, B::ConstructionError)> {
        match B::build(&socket) {
            Ok(buffer) => {
                let (weak, arc) = OwnedBuffer::new_with_weak(buffer);
                let entry = SocketEntry {
                    buffer: weak,
                    socket: Some(socket),
                    data: Default::default(),
                    active_duration: Duration::ZERO,
                    drop_flag: false,
                };

                self.sockets.push(entry);

                Ok(arc)
            }
            Err(error) => {
                return Err((socket, error))
            }
        }
    }
}