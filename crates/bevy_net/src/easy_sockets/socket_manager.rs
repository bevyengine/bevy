use std::collections::VecDeque;
use std::future::Future;
use std::iter::{Enumerate, Map};
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::slice::IterMut;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::task::{Context, Poll, Waker};
use futures::future::join_all;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use bevy_internal::reflect::List;
use bevy_internal::render::render_resource::encase::private::RuntimeSizedArray;
use bevy_internal::tasks::{IoTaskPool, Task, TaskPool};
use bevy_internal::tasks::futures_lite::FutureExt;
use crate::easy_sockets::{Buffer, ErrorAction};
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
    write_result: Result<usize, ErrorAction>,
    read_result: Result<usize, ErrorAction>,
    properties_result: Result<(), ErrorAction>
}

struct UpdateResults {
    results: Result<Option<BufferUpdateResult>, ()>,
    index: usize
}

struct SocketEntry<B, S> {
    buffer: Weak<SpinLock<B>>,
    socket: Option<S>,
    data: DiagnosticData,

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
                    write_result: guard.flush_write_bufs(socket).await,
                    read_result: guard.fill_read_bufs(socket).await,
                    properties_result: guard.update_properties(socket).await,
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

                match update_results.properties_result {
                    Ok(_) => {}
                    Err(action) => {
                        error_occured = true;
                        if action.is_drop() {
                            should_drop_socket = true;
                        }
                    }
                }

                match update_results.write_result {
                    Ok(n) => {
                        self.data.written = n;
                    }
                    Err(action) => {
                        error_occured = true;
                        if action.is_drop() {
                            should_drop_socket = true;
                        }
                    }
                }

                match update_results.read_result {
                    Ok(n) => {
                        self.data.read = n;
                    }
                    Err(action) => {
                        error_occured = true;
                        if action.is_drop() {
                            should_drop_socket = true;
                        }
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

#[derive(Default)]
struct DiagnosticData {
    /// Number of consecutive ticks this
    /// buffer has had some kind of 
    /// none fatal error occur
    error_count: u32,
    /// Number of bytes written this tick.
    written: usize,
    /// Number of bytes read this tick.
    read: usize
}

struct SocketManger<B, S> {
    sockets: Vec<SocketEntry<B, S>>,
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    async fn update(&mut self) {
        let mut tasks = Vec::with_capacity(self.sockets.len());

        while let Some(entry) = self.sockets.pop() {
            tasks.push(IoTaskPool::get().spawn(async move {
                let mut entrey = entry;
                entrey.update();
                if entrey.drop_flag {
                    None
                } else {
                    Some(entrey)
                }
            }))
        }

        for task in tasks {
            if let Some(entrey) = task.await {
                self.sockets.push(entrey)
            }
        }
    }
    
    fn register(&mut self, socket: S) -> Result<OwnedBuffer<B>, (S, B::ConstructionError)> {
        match B::build(&socket) {
            Ok(buffer) => {
                let (weak, arc) = OwnedBuffer::new_with_weak(buffer);
                let entry = SocketEntry {
                    buffer: weak,
                    socket: Some(socket),
                    data: Default::default(),
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


//todo rewrite this
#[macro_export]
macro_rules! manager {


    ($name:ident, $buffer:ty, $socket:ty) => {
        use crate::easy_sockets::socket_manager::{SocketManager, OwnedBuffer};
        use bevy_internal::tasks::IoTaskPool;

        static manager: $name = $name {inner: SocketManager::new()};
            
        pub struct $name {
            inner: SocketManager<$buffer, $socket>,
        }
            
        impl $name {
            pub fn register(&self, socket: $socket) -> Result<OwnedBuffer<$buffer>, $buffer::ConstructionError> {
                self.inner.register_socket(socket)
            }
            pub fn get() -> &'static Self {
                &manager
            }
        }

        pub struct
            
        pub fn start_update_system() {
            IoTaskPool::try_get().expect("The io task pool was not initalised. \
            Maybe you forgot to add the SocketManager plugin?");
            $name.get().inner.update_and_handle()
        }
    };
}