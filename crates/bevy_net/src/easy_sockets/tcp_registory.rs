use std::collections::VecDeque;
use std::future::Future;
use std::io::{Error, ErrorKind, IoSlice, Read, Write};
use std::ops::{Deref, DerefMut};
use std::sync::Weak;
use serde::de::StdError;
use bevy_ecs::prelude::Resource;
use bevy_internal::tasks::TaskPool;
use crate::easy_sockets::AsyncHandler;
use crate::easy_sockets::tcp_registory::spin_lock::SpinLock;

struct SocketManger<B, S> {
    handler: AsyncHandler<Weak<SpinLock<(B, Option<S>)>>>
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    fn fill_read_bufs(&mut self, pool: &TaskPool) -> Vec<Result<Option<Result<usize, ErrorAction<<B as Buffer>::Error>>>, usize>> {
        self.handler.for_each_async_mut(|index, weak| async move {
            if let Some(lock) = weak.upgrade() {
                let mut guard = lock.lock_async().await.unwrap();
                let mut inner = guard.deref_mut();
                
                if let Some(socket) = &mut inner.1 {
                    return Ok(Some(inner.0.fill_read_bufs(socket).await))
                }
                return Ok(None)
            }

            Err(index)
        }, pool)
    }
    
    fn flush_write_bufs(&mut self, pool: &TaskPool) -> Vec<Result<Option<Result<usize, ErrorAction<<B as Buffer>::Error>>>, usize>> {
        self.handler.for_each_async_mut(|index, weak| async move {
            if let Some(lock) = weak.upgrade() {
                let mut guard = lock.lock_async().await.unwrap();
                let mut inner = guard.deref_mut();

                if let Some(socket) = &mut inner.1 {
                    return Ok(Some(inner.0.flush_write_bufs(socket).await))
                }
                return Ok(None)
            }

            Err(index)
        }, pool)
    }
    
    
}

enum ErrorAction<E> {
    /// Drop the socket.
    Drop,
    /// Present the error to the end user.
    Present(E),
    /// Take no automated action.
    /// However, you may wish to take
    /// your own corrective action.
    None
}

pub trait Buffer {
    type InnerSocket;

    type ReadIter: Iterator<Item = u8> + ExactSizeIterator;

    type PeekIter: Iterator<Item = u8>;

    type Error: StdError + Send + 'static;

    fn build() -> Self;

    async fn fill_read_bufs(&mut self, socket: &mut Self::InnerSocket) -> Result<usize, ErrorAction<Self::Error>>;

    async fn flush_write_bufs(&mut self, socket: &mut Self::InnerSocket) -> Result<usize, ErrorAction<Self::Error>>;
}

pub mod spin_lock {
    use std::future::Future;
    use std::hint;
    use std::ops::{Deref, DerefMut};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Poll, Waker};
    use std::thread::panicking;

    pub type SpinLockResult<'a, T> = Result<SpinLockGuard<'a, T>, ()>;

    pub struct SpinLock<T> {
        is_locked: AtomicBool,
        is_poisoned: bool,
        inner: T
    }

    pub struct SpinLockFuture<'a, T> {
        lock: &'a SpinLock<T>,
        waker: Option<Waker>
    }

    impl<'a, T> Future for SpinLockFuture<'a, T> {
        type Output = SpinLockResult<'a, T>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(res) = self.lock.try_lock() {
                return Poll::Ready(res)
            }

            self.waker = Some(cx.waker().clone());
            
            Poll::Pending
        }
    }

    #[allow(unsafe_code)]
    impl<T> SpinLock<T> {

        /// Checks if the value is in use, if not, returns
        /// Some(Ok) and locks the lock, indicating you now have exclusive read and write access.
        /// Returns None if the value is in use.
        /// Returns Err() if the value is poisoned.
        /// For correctness [`SpinLock::release_lock`]
        /// must be called after the value is no longer in use.
        fn inner_try_lock(&self) -> Option<Result<(), ()>> {
            if self.is_locked.load(Ordering::Acquire) {
                return None
            }

            self.is_locked.store(true, Ordering::Release);

            if self.is_poisoned {
                self.is_locked.store(false, Ordering::Release);
                return Some(Err(()))
            }

            Some(Ok(()))
        }

        fn release_lock(&self) {
            self.is_locked.store(false, Ordering::Release)
        }

        fn poison(&mut self) {
            self.is_poisoned = true;
        }

        unsafe fn as_mut(&self) -> &mut Self {
            ((self as *const Self) as *mut Self).as_mut().unwrap_unchecked()
        }

        pub fn try_lock<'a>(&'a self) -> Option<SpinLockResult<T>> {
            if let Some(res) = self.inner_try_lock() {
                if res.is_ok() {
                    unsafe {return Some(Ok(SpinLockGuard(self.as_mut())))}
                }
                return Some(Err(()))
            }

            None
        }

        pub fn lock<'a>(&'a self) -> SpinLockResult<T> {
            loop {
                let option = self.try_lock();

                if let Some(res) = option {
                    return res
                }

                hint::spin_loop()
            }
        }

        pub fn new(inner: T) -> Self {
            Self {
                is_locked: AtomicBool::new(false),
                is_poisoned: false,
                inner
            }
        }

        pub fn lock_async<'a>(&'a self) -> SpinLockFuture<'a, T> {
            SpinLockFuture { lock: self, waker: None }
        }
    }

    pub struct SpinLockGuard<'a, T>(&'a mut SpinLock<T>);

    impl<'a, T> Deref for SpinLockGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0.inner
        }
    }

    impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0.inner
        }
    }

    impl<'a, T> Drop for SpinLockGuard<'a, T> {
        fn drop(&mut self) {
            if panicking() {
                self.0.poison()
            }
            self.0.release_lock()
        }
    }
}