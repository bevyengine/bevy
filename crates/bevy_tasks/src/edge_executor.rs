//! Alternative to `async_executor` based on [`edge_executor`] by Ivan Markov.
//!
//! It has been vendored along with its tests to update several outdated dependencies.
//!
//! [`async_executor`]: https://github.com/smol-rs/async-executor
//! [`edge_executor`]: https://github.com/ivmarkov/edge-executor

#![expect(unsafe_code, reason = "original implementation relies on unsafe")]
#![expect(
    dead_code,
    reason = "keeping methods from original implementation for transparency"
)]

// TODO: Create a more tailored replacement, possibly integrating [Fotre](https://github.com/NthTensor/Forte)

use alloc::rc::Rc;
use core::{
    future::{poll_fn, Future},
    marker::PhantomData,
    task::{Context, Poll},
};

use async_task::{Runnable, Task};
use atomic_waker::AtomicWaker;
use bevy_platform::sync::{Arc, LazyLock};
use futures_lite::FutureExt;

/// An async executor.
///
/// # Examples
///
/// A multi-threaded executor:
///
/// ```ignore
/// use async_channel::unbounded;
/// use easy_parallel::Parallel;
///
/// use edge_executor::{Executor, block_on};
///
/// let ex: Executor = Default::default();
/// let (signal, shutdown) = unbounded::<()>();
///
/// Parallel::new()
///     // Run four executor threads.
///     .each(0..4, |_| block_on(ex.run(shutdown.recv())))
///     // Run the main future on the current thread.
///     .finish(|| block_on(async {
///         println!("Hello world!");
///         drop(signal);
///     }));
/// ```
pub struct Executor<'a, const C: usize = 64> {
    state: LazyLock<Arc<State<C>>>,
    _invariant: PhantomData<core::cell::UnsafeCell<&'a ()>>,
}

impl<'a, const C: usize> Executor<'a, C> {
    /// Creates a new executor.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::Executor;
    ///
    /// let ex: Executor = Default::default();
    /// ```
    pub const fn new() -> Self {
        Self {
            state: LazyLock::new(|| Arc::new(State::new())),
            _invariant: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::Executor;
    ///
    /// let ex: Executor = Default::default();
    ///
    /// let task = ex.spawn(async {
    ///     println!("Hello world");
    /// });
    /// ```
    ///
    /// Note that if the executor's queue size is equal to the number of currently
    /// spawned and running tasks, spawning this additional task might cause the executor to panic
    /// later, when the task is scheduled for polling.
    pub fn spawn<F>(&self, fut: F) -> Task<F::Output>
    where
        F: Future + Send + 'a,
        F::Output: Send + 'a,
    {
        // SAFETY: Original implementation missing safety documentation
        unsafe { self.spawn_unchecked(fut) }
    }

    /// Attempts to run a task if at least one is scheduled.
    ///
    /// Running a scheduled task means simply polling its future once.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::Executor;
    ///
    /// let ex: Executor = Default::default();
    /// assert!(!ex.try_tick()); // no tasks to run
    ///
    /// let task = ex.spawn(async {
    ///     println!("Hello world");
    /// });
    /// assert!(ex.try_tick()); // a task was found
    /// ```    
    pub fn try_tick(&self) -> bool {
        if let Some(runnable) = self.try_runnable() {
            runnable.run();

            true
        } else {
            false
        }
    }

    /// Runs a single task asynchronously.
    ///
    /// Running a task means simply polling its future once.
    ///
    /// If no tasks are scheduled when this method is called, it will wait until one is scheduled.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::{Executor, block_on};
    ///
    /// let ex: Executor = Default::default();
    ///
    /// let task = ex.spawn(async {
    ///     println!("Hello world");
    /// });
    /// block_on(ex.tick()); // runs the task
    /// ```
    pub async fn tick(&self) {
        self.runnable().await.run();
    }

    /// Runs the executor asynchronously until the given future completes.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::{Executor, block_on};
    ///
    /// let ex: Executor = Default::default();
    ///
    /// let task = ex.spawn(async { 1 + 2 });
    /// let res = block_on(ex.run(async { task.await * 2 }));
    ///
    /// assert_eq!(res, 6);
    /// ```
    pub async fn run<F>(&self, fut: F) -> F::Output
    where
        F: Future + Send + 'a,
    {
        // SAFETY: Original implementation missing safety documentation
        unsafe { self.run_unchecked(fut).await }
    }

    /// Waits for the next runnable task to run.
    async fn runnable(&self) -> Runnable {
        poll_fn(|ctx| self.poll_runnable(ctx)).await
    }

    /// Polls the first task scheduled for execution by the executor.
    fn poll_runnable(&self, ctx: &Context<'_>) -> Poll<Runnable> {
        self.state().waker.register(ctx.waker());

        if let Some(runnable) = self.try_runnable() {
            Poll::Ready(runnable)
        } else {
            Poll::Pending
        }
    }

    /// Pops the first task scheduled for execution by the executor.
    ///
    /// Returns
    /// - `None` - if no task was scheduled for execution
    /// - `Some(Runnnable)` - the first task scheduled for execution. Calling `Runnable::run` will
    ///   execute the task. In other words, it will poll its future.
    fn try_runnable(&self) -> Option<Runnable> {
        let runnable;

        #[cfg(all(
            target_has_atomic = "8",
            target_has_atomic = "16",
            target_has_atomic = "32",
            target_has_atomic = "64",
            target_has_atomic = "ptr"
        ))]
        {
            runnable = self.state().queue.pop();
        }

        #[cfg(not(all(
            target_has_atomic = "8",
            target_has_atomic = "16",
            target_has_atomic = "32",
            target_has_atomic = "64",
            target_has_atomic = "ptr"
        )))]
        {
            runnable = self.state().queue.dequeue();
        }

        runnable
    }

    /// # Safety
    ///
    /// Original implementation missing safety documentation
    unsafe fn spawn_unchecked<F>(&self, fut: F) -> Task<F::Output>
    where
        F: Future,
    {
        let schedule = {
            let state = self.state().clone();

            move |runnable| {
                #[cfg(all(
                    target_has_atomic = "8",
                    target_has_atomic = "16",
                    target_has_atomic = "32",
                    target_has_atomic = "64",
                    target_has_atomic = "ptr"
                ))]
                {
                    state.queue.push(runnable).unwrap();
                }

                #[cfg(not(all(
                    target_has_atomic = "8",
                    target_has_atomic = "16",
                    target_has_atomic = "32",
                    target_has_atomic = "64",
                    target_has_atomic = "ptr"
                )))]
                {
                    state.queue.enqueue(runnable).unwrap();
                }

                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }
            }
        };

        // SAFETY: Original implementation missing safety documentation
        let (runnable, task) = unsafe { async_task::spawn_unchecked(fut, schedule) };

        runnable.schedule();

        task
    }

    /// # Safety
    ///
    /// Original implementation missing safety documentation
    async unsafe fn run_unchecked<F>(&self, fut: F) -> F::Output
    where
        F: Future,
    {
        let run_forever = async {
            loop {
                self.tick().await;
            }
        };

        run_forever.or(fut).await
    }

    /// Returns a reference to the inner state.
    fn state(&self) -> &Arc<State<C>> {
        &self.state
    }
}

impl<'a, const C: usize> Default for Executor<'a, C> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: Original implementation missing safety documentation
unsafe impl<'a, const C: usize> Send for Executor<'a, C> {}
// SAFETY: Original implementation missing safety documentation
unsafe impl<'a, const C: usize> Sync for Executor<'a, C> {}

/// A thread-local executor.
///
/// The executor can only be run on the thread that created it.
///
/// # Examples
///
/// ```ignore
/// use edge_executor::{LocalExecutor, block_on};
///
/// let local_ex: LocalExecutor = Default::default();
///
/// block_on(local_ex.run(async {
///     println!("Hello world!");
/// }));
/// ```
pub struct LocalExecutor<'a, const C: usize = 64> {
    executor: Executor<'a, C>,
    _not_send: PhantomData<core::cell::UnsafeCell<&'a Rc<()>>>,
}

impl<'a, const C: usize> LocalExecutor<'a, C> {
    /// Creates a single-threaded executor.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::LocalExecutor;
    ///
    /// let local_ex: LocalExecutor = Default::default();
    /// ```
    pub const fn new() -> Self {
        Self {
            executor: Executor::<C>::new(),
            _not_send: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::LocalExecutor;
    ///
    /// let local_ex: LocalExecutor = Default::default();
    ///
    /// let task = local_ex.spawn(async {
    ///     println!("Hello world");
    /// });
    /// ```
    ///
    /// Note that if the executor's queue size is equal to the number of currently
    /// spawned and running tasks, spawning this additional task might cause the executor to panic
    /// later, when the task is scheduled for polling.
    pub fn spawn<F>(&self, fut: F) -> Task<F::Output>
    where
        F: Future + 'a,
        F::Output: 'a,
    {
        // SAFETY: Original implementation missing safety documentation
        unsafe { self.executor.spawn_unchecked(fut) }
    }

    /// Attempts to run a task if at least one is scheduled.
    ///
    /// Running a scheduled task means simply polling its future once.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::LocalExecutor;
    ///
    /// let local_ex: LocalExecutor = Default::default();
    /// assert!(!local_ex.try_tick()); // no tasks to run
    ///
    /// let task = local_ex.spawn(async {
    ///     println!("Hello world");
    /// });
    /// assert!(local_ex.try_tick()); // a task was found
    /// ```    
    pub fn try_tick(&self) -> bool {
        self.executor.try_tick()
    }

    /// Runs a single task asynchronously.
    ///
    /// Running a task means simply polling its future once.
    ///
    /// If no tasks are scheduled when this method is called, it will wait until one is scheduled.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::{LocalExecutor, block_on};
    ///
    /// let local_ex: LocalExecutor = Default::default();
    ///
    /// let task = local_ex.spawn(async {
    ///     println!("Hello world");
    /// });
    /// block_on(local_ex.tick()); // runs the task
    /// ```
    pub async fn tick(&self) {
        self.executor.tick().await;
    }

    /// Runs the executor asynchronously until the given future completes.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edge_executor::{LocalExecutor, block_on};
    ///
    /// let local_ex: LocalExecutor = Default::default();
    ///
    /// let task = local_ex.spawn(async { 1 + 2 });
    /// let res = block_on(local_ex.run(async { task.await * 2 }));
    ///
    /// assert_eq!(res, 6);
    /// ```
    pub async fn run<F>(&self, fut: F) -> F::Output
    where
        F: Future,
    {
        // SAFETY: Original implementation missing safety documentation
        unsafe { self.executor.run_unchecked(fut) }.await
    }
}

impl<'a, const C: usize> Default for LocalExecutor<'a, C> {
    fn default() -> Self {
        Self::new()
    }
}

struct State<const C: usize> {
    #[cfg(all(
        target_has_atomic = "8",
        target_has_atomic = "16",
        target_has_atomic = "32",
        target_has_atomic = "64",
        target_has_atomic = "ptr"
    ))]
    queue: crossbeam_queue::ArrayQueue<Runnable>,
    #[cfg(not(all(
        target_has_atomic = "8",
        target_has_atomic = "16",
        target_has_atomic = "32",
        target_has_atomic = "64",
        target_has_atomic = "ptr"
    )))]
    queue: heapless::mpmc::MpMcQueue<Runnable, C>,
    waker: AtomicWaker,
}

impl<const C: usize> State<C> {
    fn new() -> Self {
        Self {
            #[cfg(all(
                target_has_atomic = "8",
                target_has_atomic = "16",
                target_has_atomic = "32",
                target_has_atomic = "64",
                target_has_atomic = "ptr"
            ))]
            queue: crossbeam_queue::ArrayQueue::new(C),
            #[cfg(not(all(
                target_has_atomic = "8",
                target_has_atomic = "16",
                target_has_atomic = "32",
                target_has_atomic = "64",
                target_has_atomic = "ptr"
            )))]
            queue: heapless::mpmc::MpMcQueue::new(),
            waker: AtomicWaker::new(),
        }
    }
}

#[cfg(test)]
mod different_executor_tests {
    use core::cell::Cell;

    use bevy_tasks::{block_on, futures_lite::{pending, poll_once}};
    use futures_lite::pin;

    use super::LocalExecutor;

    #[test]
    fn shared_queue_slot() {
        block_on(async {
            let was_polled = Cell::new(false);
            let future = async {
                was_polled.set(true);
                pending::<()>().await;
            };

            let ex1: LocalExecutor = Default::default();
            let ex2: LocalExecutor = Default::default();

            // Start the futures for running forever.
            let (run1, run2) = (ex1.run(pending::<()>()), ex2.run(pending::<()>()));
            pin!(run1);
            pin!(run2);
            assert!(poll_once(run1.as_mut()).await.is_none());
            assert!(poll_once(run2.as_mut()).await.is_none());

            // Spawn the future on executor one and then poll executor two.
            ex1.spawn(future).detach();
            assert!(poll_once(run2).await.is_none());
            assert!(!was_polled.get());

            // Poll the first one.
            assert!(poll_once(run1).await.is_none());
            assert!(was_polled.get());
        });
    }
}

#[cfg(test)]
mod drop_tests {
    use alloc::string::String;
    use core::mem;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use core::task::{Poll, Waker};
    use std::sync::Mutex;

    use bevy_platform::sync::LazyLock;
    use futures_lite::future;

    use super::{Executor, Task};

    #[test]
    fn leaked_executor_leaks_everything() {
        static DROP: AtomicUsize = AtomicUsize::new(0);
        static WAKER: LazyLock<Mutex<Option<Waker>>> = LazyLock::new(Default::default);

        let ex: Executor = Default::default();

        let task = ex.spawn(async {
            let _guard = CallOnDrop(|| {
                DROP.fetch_add(1, Ordering::SeqCst);
            });

            future::poll_fn(|cx| {
                *WAKER.lock().unwrap() = Some(cx.waker().clone());
                Poll::Pending::<()>
            })
            .await;
        });

        future::block_on(ex.tick());
        assert!(WAKER.lock().unwrap().is_some());
        assert_eq!(DROP.load(Ordering::SeqCst), 0);

        mem::forget(ex);
        assert_eq!(DROP.load(Ordering::SeqCst), 0);

        assert!(future::block_on(future::poll_once(task)).is_none());
        assert_eq!(DROP.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn await_task_after_dropping_executor() {
        let s: String = "hello".into();

        let ex: Executor = Default::default();
        let task: Task<&str> = ex.spawn(async { &*s });
        assert!(ex.try_tick());

        drop(ex);
        assert_eq!(future::block_on(task), "hello");
        drop(s);
    }

    #[test]
    fn drop_executor_and_then_drop_finished_task() {
        static DROP: AtomicUsize = AtomicUsize::new(0);

        let ex: Executor = Default::default();
        let task = ex.spawn(async {
            CallOnDrop(|| {
                DROP.fetch_add(1, Ordering::SeqCst);
            })
        });
        assert!(ex.try_tick());

        assert_eq!(DROP.load(Ordering::SeqCst), 0);
        drop(ex);
        assert_eq!(DROP.load(Ordering::SeqCst), 0);
        drop(task);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_finished_task_and_then_drop_executor() {
        static DROP: AtomicUsize = AtomicUsize::new(0);

        let ex: Executor = Default::default();
        let task = ex.spawn(async {
            CallOnDrop(|| {
                DROP.fetch_add(1, Ordering::SeqCst);
            })
        });
        assert!(ex.try_tick());

        assert_eq!(DROP.load(Ordering::SeqCst), 0);
        drop(task);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
        drop(ex);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
    }

    struct CallOnDrop<F: Fn()>(F);

    impl<F: Fn()> Drop for CallOnDrop<F> {
        fn drop(&mut self) {
            (self.0)();
        }
    }
}

#[cfg(test)]
mod local_queue {
    use alloc::boxed::Box;

    use futures_lite::{future, pin};

    use super::Executor;

    #[test]
    fn two_queues() {
        future::block_on(async {
            // Create an executor with two runners.
            let ex: Executor = Default::default();
            let (run1, run2) = (
                ex.run(future::pending::<()>()),
                ex.run(future::pending::<()>()),
            );
            let mut run1 = Box::pin(run1);
            pin!(run2);

            // Poll them both.
            assert!(future::poll_once(run1.as_mut()).await.is_none());
            assert!(future::poll_once(run2.as_mut()).await.is_none());

            // Drop the first one, which should leave the local queue in the `None` state.
            drop(run1);
            assert!(future::poll_once(run2.as_mut()).await.is_none());
        });
    }
}
