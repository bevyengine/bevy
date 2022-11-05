use async_task::{Runnable, Task};
use concurrent_queue::ConcurrentQueue;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};

/// A simple MPSC executor.
///
/// The executor can be run on any thread and enqueue tasks from any thread.
/// All enqueued tasks must be [`Send`]. Internally only has one queue and
/// does not do any form of work stealing.
#[derive(Debug)]
pub struct SimpleExecutor<'a> {
    queue: ConcurrentQueue<Runnable>,
    _marker: PhantomData<&'a ()>,
}

impl UnwindSafe for SimpleExecutor<'_> {}
impl RefUnwindSafe for SimpleExecutor<'_> {}

impl<'a> SimpleExecutor<'a> {
    /// Creates a single-threaded executor.
    pub fn new() -> SimpleExecutor<'a> {
        SimpleExecutor {
            queue: ConcurrentQueue::unbounded(),
            _marker: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T: 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        // SAFETY: The provided future is Send and scoped to the lifetime of the executor.
        //
        // Even if the returned Task and waker are sent to another thread, the associated inner
        // task is only dropped when `try_tick` is triggered.
        let (runnable, task) = unsafe { async_task::spawn_unchecked(future, self.schedule()) };
        self.queue.push(runnable).unwrap();
        task
    }

    /// Attempts to run a task if at least one is scheduled.
    ///
    /// Running a scheduled task means simply polling its future once.
    #[inline]
    pub fn try_tick(&self) -> bool {
        match self.queue.pop() {
            Err(_) => false,
            Ok(runnable) => {
                runnable.run();
                true
            }
        }
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + '_ + Send + Sync {
        move |runnable| {
            self.queue.push(runnable).unwrap();
        }
    }
}
