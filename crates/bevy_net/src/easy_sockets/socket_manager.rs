use std::future::{Future, IntoFuture};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use bevy_internal::reflect::List;
use bevy_internal::tasks::{IoTaskPool, Task, TaskPool};
use crate::easy_sockets::{Buffer, ErrorAction, UpdateResult};
use crate::easy_sockets::spin_lock::{SpinLock, SpinLockGuard};

struct Time<F> 
where F: Future {
    inner: F,
    woken_time: Duration
}

impl<F> Time<F>
where F: Future {
    fn new(inner: F) -> Self {
        Self { inner: inner, woken_time: Duration::ZERO }
    }
}

impl<F> Future for Time<F> 
where F: Future {
    type Output = (Duration, F::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let start = Instant::now();
        match self.inner.poll(cx) {
            Poll::Ready(item) => {
                let end = Instant::now() - start;
                Poll::Ready((end + self.woken_time, item))
            }
            Poll::Pending => {
                self.woken_time += Instant::now() - start;
                Poll::Pending
            }
        }
    }
}

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

struct Update<'a, B, S>
where B: Buffer<InnerSocket = S>{
    manager: &'a mut SocketManger<B, S>,
    tasks: Vec<Task<Option<SocketEntry<B, S>>>>
}

impl<'a, B, S> Future for Update<'a, B, S>
where B: Buffer<InnerSocket = S> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(fut) = self.tasks.last_mut() {
            match fut.poll(cx) {
                Poll::Ready(optional) => {
                    if let Some(entry) = optional {
                        self.manager.sockets.push(entry)
                    }
                    self.tasks.pop();
                    
                    return Poll::Pending
                }
                Poll::Pending => {
                    return Poll::Pending
                }
            }
        }
        return Poll::Ready(())
    }
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    pub fn new() -> Self {
        Self {
            sockets: vec![],
        }
    }
    
    async fn update(&mut self) {
        let mut tasks = Vec::with_capacity(self.sockets.len());

        
        let pool = IoTaskPool::get().deref();
        
        while let Some(entry) = self.sockets.pop() {
            tasks.push(pool.spawn(async {
                let mut entrey = entry;
                entrey.update();
                if entrey.drop_flag {
                    None
                } else {
                    Some(entrey)
                }
            }))
        }

        todo!()
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