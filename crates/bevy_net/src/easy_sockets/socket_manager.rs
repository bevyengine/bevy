use std::future::Future;
use std::iter::{Enumerate, Map};
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::slice::IterMut;
use std::sync::{Arc, Mutex, RwLock, Weak};
use bevy_internal::tasks::{IoTaskPool, TaskPool};
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

struct SocketEnrey<B, S> {
    buffer: Weak<SpinLock<B>>,
    socket: Option<S>,
    data: DiagnosticData
}

#[derive(Default)]
struct DiagnosticData {
    /// Number of consecutive ticks this
    /// buffer has had some kind of 
    /// none fatal error occur
    error_count: u32,
}

struct SocketManger<B, S> {
    sockets: Vec<SocketEnrey<B, S>>,
}

struct FutureSpawner<'a, B, S, F, Fut>
where F: Fn(&'a mut SocketEnrey<B, S>) -> Fut, Fut: Future {
    inner: IterMut<'a, SocketEnrey<B, S>>,
    function: F
}

impl<'a, B, S, F, Fut> Iterator for FutureSpawner<'a, B, S, F, Fut>
where F: Fn(&'a mut SocketEnrey<B, S>) -> Fut, Fut: Future {
    type Item = Fut;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entrey) = self.inner.next() {
            let func = &self.function;
            return Some(func(entrey))
        }
        None
    }
}


impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    fn future_iter<'a, F, Fut>(&'a mut self, func: F) -> FutureSpawner<'a, B, S, F, Fut>
    where F: Fn(&'a mut SocketEnrey<B, S>) -> Fut, Fut: Future {
        FutureSpawner { inner: self.sockets.iter_mut(), function: func }
    }

    fn update_futures<'a, F1, F2, F3, F4>(&mut self) {
        let iter = self.future_iter(|entrey| async {
            if let Some(lock) = entrey.buffer.upgrade() {
                if let Some(socket) = &mut entrey.socket {
                    let mut guard = lock.lock_async().await.unwrap();
                    
                    return Ok(Some(BufferUpdateResult {
                        write_result: guard.flush_write_bufs(socket).await,
                        read_result: guard.fill_read_bufs(socket).await,
                        properties_result: guard.update_properties(socket).await
                    }))
                }
                
                return Ok(None)
            }
            
            Err(())
        }).enumerate().map(|(index, future)| {
            async move {
                UpdateResults { results: future.await, index }
            }
        });
        
        iter
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