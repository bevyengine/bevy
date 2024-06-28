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

    async fn update(&mut self) -> Vec<UpdateResults> {
        let pool = IoTaskPool::get();

        pool.scope(|scope| {
            for fut in self.future_iter(|entrey| async {
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
            }) {
                scope.spawn(fut)
            }
        })
    }

    async fn update_and_handle(&mut self) {
        let results = self.update().await;

        let mut to_be_removed = Vec::with_capacity(self.sockets.len());
        
        for res in results {
            let index = res.index;
            
            match res.results {
                Ok(optional) => {
                    if let Some(results) = optional {
                        let mut error_occured = false;
                        let mut should_drop = false;
                        
                        let current_entrey = &mut self.sockets[index];
                        
                        match results.write_result {
                            Ok(n) => {
                                //todo add write and read rates as diagnostic data
                            }
                            Err(action) => {
                                error_occured = true;
                                if action.is_drop() {
                                    should_drop = true;
                                }
                            }
                        }

                        match results.read_result {
                            Ok(n) => {

                            }
                            Err(action) => {
                                error_occured = true;
                                if action.is_drop() {
                                    should_drop = true;
                                }
                            }
                        }
                        
                        if let Err(action) = results.properties_result {
                            error_occured = true;
                            if action.is_drop() {
                                should_drop = true;
                            }
                        }
                        
                        if should_drop {
                            current_entrey.socket = None;
                        }
                        
                        if error_occured {
                            current_entrey.data.error_count += 1;
                        } else {
                            current_entrey.data.error_count = 0;
                        }
                    }
                }
                Err(_) => {
                    to_be_removed.push(index);
                }
            }
        }
        
        to_be_removed.sort_unstable();
        
        for index in to_be_removed.into_iter().rev() {
            self.sockets.remove(index);
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