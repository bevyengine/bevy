use async_task::{Runnable, Task};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::rc::Rc;

/// A thread-local executor.
///
/// The executor can only be run on the thread that created it.
#[derive(Debug)]
pub struct LocalExecutor<'a> {
    queue: RefCell<VecDeque<Runnable>>,

    /// Makes the type `!Send` and `!Sync`.
    _marker: PhantomData<&'a Rc<()>>,
}

impl UnwindSafe for LocalExecutor<'_> {}
impl RefUnwindSafe for LocalExecutor<'_> {}

impl<'a> LocalExecutor<'a> {
    /// Creates a single-threaded executor.
    pub fn new() -> LocalExecutor<'a> {
        LocalExecutor {
            queue: RefCell::new(VecDeque::new()),
            _marker: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T: 'a>(&self, future: impl Future<Output = T> + 'a) -> Task<T> {
        // SAFETY: The spawned Task can only be progressed via `try_tick` which must be accessed
        // from the thread that owns the executor and the task.
        //
        // Even if the returned Task and waker are sent to another thread, the associated inner
        // task is only dropped when `try_tick` is triggered.
        let (runnable, task) = unsafe { async_task::spawn_unchecked(future, self.schedule()) };
        self.queue.borrow_mut().push_back(runnable);
        task
    }

    /// Attempts to fetch a task if at least one is scheduled.
    #[inline]
    pub fn try_fetch(&self) -> Option<Runnable> {
        self.queue.borrow_mut().pop_front()
    }

    /// Attempts to run a task if at least one is scheduled.
    ///
    /// Running a scheduled task means simply polling its future once.
    #[inline]
    pub fn try_tick(&self) -> bool {
        match self.try_fetch() {
            None => false,
            Some(runnable) => {
                runnable.run();
                true
            }
        }
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + '_ {
        move |runnable| {
            self.queue.borrow_mut().push_back(runnable);
        }
    }
}

impl<'a> Default for LocalExecutor<'a> {
    fn default() -> LocalExecutor<'a> {
        LocalExecutor::new()
    }
}
