// Forked from async_executor

use parking_lot::{Mutex, RwLock};
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Poll, Waker};

use async_task::Runnable;
use concurrent_queue::ConcurrentQueue;
use futures_lite::{future, prelude::*};
use slab::Slab;

#[doc(no_inline)]
pub use async_task::Task;

/// An async executor.
#[derive(Debug)]
pub struct Executor<'a> {
    /// The executor state.
    state: Arc<State>,

    /// Makes the `'a` lifetime invariant.
    _marker: PhantomData<std::cell::UnsafeCell<&'a ()>>,
}

unsafe impl Send for Executor<'_> {}
unsafe impl Sync for Executor<'_> {}

impl UnwindSafe for Executor<'_> {}
impl RefUnwindSafe for Executor<'_> {}

impl<'a> Executor<'a> {
    /// Creates a new executor.
    #[inline]
    pub fn new(priorities: usize) -> Executor<'a> {
        Executor {
            state: Arc::new(State::new(priorities)),
            _marker: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T: Send + 'a>(
        &self,
        priority: usize,
        future: impl Future<Output = T> + Send + 'a,
    ) -> Task<T> {
        let mut active = self.state.active.lock();

        // Remove the task from the set of active tasks when the future finishes.
        let index = active.vacant_entry().key();
        let state = self.state.clone();
        let future = async move {
            let _guard = CallOnDrop(move || drop(state.active.lock().try_remove(index)));
            future.await
        };

        // Create the task and register it in the set of active tasks.
        let (runnable, task) =
            unsafe { async_task::spawn_unchecked(future, self.schedule(priority)) };
        active.insert(runnable.waker());

        runnable.schedule();
        task
    }

    /// Attempts to run a task if at least one is scheduled.
    pub fn try_tick(&self, priority: usize) -> bool {
        match self.state.queues[priority].pop() {
            Err(_) => false,
            Ok(runnable) => {
                // Notify another ticker now to pick up where this ticker left off, just in case
                // running the task takes a long time.
                self.state.notify();

                // Run the task.
                runnable.run();
                true
            }
        }
    }

    /// Runs the executor until the given future completes.
    pub async fn run<T>(&self, priority: usize, future: impl Future<Output = T>) -> T {
        let runner = Runner::new(priority, &self.state);

        // A future that runs tasks forever.
        let run_forever = async {
            loop {
                for _ in 0..200 {
                    let runnable = runner.runnable().await;
                    runnable.run();
                }
                future::yield_now().await;
            }
        };

        // Run `future` and `run_forever` concurrently until `future` completes.
        future.or(run_forever).await
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self, priority: usize) -> impl Fn(Runnable) + Send + Sync + 'static {
        let state = self.state.clone();

        // TODO(stjepang): If possible, push into the current local queue and notify the ticker.
        move |runnable| {
            state.queues[priority].push(runnable).unwrap();
            state.notify();
        }
    }
}

impl Drop for Executor<'_> {
    fn drop(&mut self) {
        let mut active = self.state.active.lock();
        for w in active.drain() {
            w.wake();
        }
        drop(active);

        for queue in self.state.queues.iter() {
            while queue.pop().is_ok() {}
        }
    }
}

/// A thread-local executor.
///
/// The executor can only be run on the thread that created it.
#[derive(Debug)]
pub struct LocalExecutor<'a> {
    /// The inner executor.
    inner: Executor<'a>,

    /// Makes the type `!Send` and `!Sync`.
    _marker: PhantomData<Rc<()>>,
}

impl UnwindSafe for LocalExecutor<'_> {}
impl RefUnwindSafe for LocalExecutor<'_> {}

impl<'a> LocalExecutor<'a> {
    /// Creates a single-threaded executor.
    pub fn new() -> LocalExecutor<'a> {
        LocalExecutor {
            inner: Executor::new(1),
            _marker: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T: 'a>(&self, future: impl Future<Output = T> + 'a) -> Task<T> {
        let mut active = self.inner.state.active.lock();

        // Remove the task from the set of active tasks when the future finishes.
        let index = active.vacant_entry().key();
        let state = self.inner.state.clone();
        let future = async move {
            let _guard = CallOnDrop(move || drop(state.active.lock().try_remove(index)));
            future.await
        };

        // Create the task and register it in the set of active tasks.
        let (runnable, task) = unsafe { async_task::spawn_unchecked(future, self.schedule()) };
        active.insert(runnable.waker());

        runnable.schedule();
        task
    }

    /// Attempts to run a task if at least one is scheduled.
    ///
    /// Running a scheduled task means simply polling its future once.
    pub fn try_tick(&self) -> bool {
        self.inner.try_tick(0)
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let state = self.inner.state.clone();

        move |runnable| {
            state.queues[0].push(runnable).unwrap();
            state.notify();
        }
    }
}

impl<'a> Default for LocalExecutor<'a> {
    fn default() -> LocalExecutor<'a> {
        LocalExecutor::new()
    }
}

/// The state of a executor.
#[derive(Debug)]
struct State {
    /// The global queue.
    queues: Box<[ConcurrentQueue<Runnable>]>,

    /// Local queues created by runners.
    local_queues: Box<[RwLock<Vec<Arc<ConcurrentQueue<Runnable>>>>]>,

    /// Set to `true` when a sleeping ticker is notified or no tickers are sleeping.
    notified: AtomicBool,

    /// A list of sleeping tickers.
    sleepers: Mutex<Sleepers>,

    /// Currently active tasks.
    active: Mutex<Slab<Waker>>,
}

impl State {
    /// Creates state for a new executor.
    fn new(priorities: usize) -> State {
        let queues = (0..priorities)
            .map(|_| ConcurrentQueue::unbounded())
            .collect::<Vec<_>>();
        let local_queues = (0..priorities)
            .map(|_| RwLock::new(Vec::new()))
            .collect::<Vec<_>>();
        State {
            queues: queues.into_boxed_slice(),
            local_queues: local_queues.into_boxed_slice(),
            notified: AtomicBool::new(true),
            sleepers: Mutex::new(Sleepers {
                count: 0,
                wakers: Vec::new(),
                free_ids: Vec::new(),
            }),
            active: Mutex::new(Slab::new()),
        }
    }

    /// Notifies a sleeping ticker.
    #[inline]
    fn notify(&self) {
        if self
            .notified
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let waker = self.sleepers.lock().notify();
            if let Some(w) = waker {
                w.wake();
            }
        }
    }
}

/// A list of sleeping tickers.
#[derive(Debug)]
struct Sleepers {
    /// Number of sleeping tickers (both notified and unnotified).
    count: usize,

    /// IDs and wakers of sleeping unnotified tickers.
    ///
    /// A sleeping ticker is notified when its waker is missing from this list.
    wakers: Vec<(usize, Waker)>,

    /// Reclaimed IDs.
    free_ids: Vec<usize>,
}

impl Sleepers {
    /// Inserts a new sleeping ticker.
    fn insert(&mut self, waker: &Waker) -> usize {
        let id = match self.free_ids.pop() {
            Some(id) => id,
            None => self.count + 1,
        };
        self.count += 1;
        self.wakers.push((id, waker.clone()));
        id
    }

    /// Re-inserts a sleeping ticker's waker if it was notified.
    ///
    /// Returns `true` if the ticker was notified.
    fn update(&mut self, id: usize, waker: &Waker) -> bool {
        for item in &mut self.wakers {
            if item.0 == id {
                if !item.1.will_wake(waker) {
                    item.1 = waker.clone();
                }
                return false;
            }
        }

        self.wakers.push((id, waker.clone()));
        true
    }

    /// Removes a previously inserted sleeping ticker.
    ///
    /// Returns `true` if the ticker was notified.
    fn remove(&mut self, id: usize) -> bool {
        self.count -= 1;
        self.free_ids.push(id);

        for i in (0..self.wakers.len()).rev() {
            if self.wakers[i].0 == id {
                self.wakers.remove(i);
                return false;
            }
        }
        true
    }

    /// Returns `true` if a sleeping ticker is notified or no tickers are sleeping.
    fn is_notified(&self) -> bool {
        self.count == 0 || self.count > self.wakers.len()
    }

    /// Returns notification waker for a sleeping ticker.
    ///
    /// If a ticker was notified already or there are no tickers, `None` will be returned.
    fn notify(&mut self) -> Option<Waker> {
        if self.wakers.len() == self.count {
            self.wakers.pop().map(|item| item.1)
        } else {
            None
        }
    }
}

/// Runs task one by one.
#[derive(Debug)]
struct Ticker<'a> {
    /// The executor state.
    state: &'a State,

    /// Set to a non-zero sleeper ID when in sleeping state.
    ///
    /// States a ticker can be in:
    /// 1) Woken.
    /// 2a) Sleeping and unnotified.
    /// 2b) Sleeping and notified.
    sleeping: AtomicUsize,
}

impl Ticker<'_> {
    /// Creates a ticker.
    fn new(state: &State) -> Ticker<'_> {
        Ticker {
            state,
            sleeping: AtomicUsize::new(0),
        }
    }

    /// Moves the ticker into sleeping and unnotified state.
    ///
    /// Returns `false` if the ticker was already sleeping and unnotified.
    fn sleep(&self, waker: &Waker) -> bool {
        let mut sleepers = self.state.sleepers.lock();

        match self.sleeping.load(Ordering::SeqCst) {
            // Move to sleeping state.
            0 => self
                .sleeping
                .store(sleepers.insert(waker), Ordering::SeqCst),

            // Already sleeping, check if notified.
            id => {
                if !sleepers.update(id, waker) {
                    return false;
                }
            }
        }

        self.state
            .notified
            .swap(sleepers.is_notified(), Ordering::SeqCst);

        true
    }

    /// Moves the ticker into woken state.
    fn wake(&self) {
        let id = self.sleeping.swap(0, Ordering::SeqCst);
        if id != 0 {
            let mut sleepers = self.state.sleepers.lock();
            sleepers.remove(id);

            self.state
                .notified
                .swap(sleepers.is_notified(), Ordering::SeqCst);
        }
    }

    /// Waits for the next runnable task to run, given a function that searches for a task.
    async fn runnable_with(&self, mut search: impl FnMut() -> Option<Runnable>) -> Runnable {
        future::poll_fn(|cx| {
            loop {
                match search() {
                    None => {
                        // Move to sleeping and unnotified state.
                        if !self.sleep(cx.waker()) {
                            // If already sleeping and unnotified, return.
                            return Poll::Pending;
                        }
                    }
                    Some(r) => {
                        // Wake up.
                        self.wake();

                        // Notify another ticker now to pick up where this ticker left off, just in
                        // case running the task takes a long time.
                        self.state.notify();

                        return Poll::Ready(r);
                    }
                }
            }
        })
        .await
    }
}

impl Drop for Ticker<'_> {
    fn drop(&mut self) {
        // If this ticker is in sleeping state, it must be removed from the sleepers list.
        let id = self.sleeping.swap(0, Ordering::SeqCst);
        if id != 0 {
            let mut sleepers = self.state.sleepers.lock();
            let notified = sleepers.remove(id);

            self.state
                .notified
                .swap(sleepers.is_notified(), Ordering::SeqCst);

            // If this ticker was notified, then notify another ticker.
            if notified {
                drop(sleepers);
                self.state.notify();
            }
        }
    }
}

/// A worker in a work-stealing executor.
///
/// This is just a ticker that also has an associated local queue for improved cache locality.
#[derive(Debug)]
struct Runner<'a> {
    priority: usize,

    /// The executor state.
    state: &'a State,

    /// Inner ticker.
    ticker: Ticker<'a>,

    /// The local queue.
    local: Arc<ConcurrentQueue<Runnable>>,

    /// Bumped every time a runnable task is found.
    ticks: AtomicUsize,
}

impl Runner<'_> {
    /// Creates a runner and registers it in the executor state.
    fn new(priority: usize, state: &State) -> Runner<'_> {
        let runner = Runner {
            priority,
            state,
            ticker: Ticker::new(state),
            local: Arc::new(ConcurrentQueue::bounded(512)),
            ticks: AtomicUsize::new(0),
        };
        state.local_queues[priority]
            .write()
            .push(runner.local.clone());
        runner
    }

    fn priority_iter(&self) -> impl Iterator<Item=usize> {
	// Prioritize the immediate responsibility of the runner, then search in reverse order
	std::iter::once(self.priority).chain((self.priority + 1..self.state.queues.len()).rev())
    }

    /// Waits for the next runnable task to run.
    async fn runnable(&self) -> Runnable {
        let runnable = self
            .ticker
            .runnable_with(|| {
                // Try the local queue.
                if let Ok(r) = self.local.pop() {
                    return Some(r);
                }

                // Try stealing from the global queue then try stealing from higher priority global queues.
                for priority in self.priority_iter() {
                    if let Ok(r) = self.state.queues[priority].pop() {
                        steal(&self.state.queues[priority], &self.local);
                        return Some(r);
                    }
                }

                // // Try stealing from other runners local queues.
                for priority in self.priority_iter() {
                    let local_queues = self.state.local_queues[priority].read();

                    // // Pick a random starting point in the iterator list and rotate the list.
                    let n = local_queues.len();
                    let start = fastrand::usize(..n);
                    let iter = local_queues
                        .iter()
                        .chain(local_queues.iter())
                        .skip(start)
                        .take(n);

                    // // Remove this runner's local queue.
                    let iter = iter.filter(|local| !Arc::ptr_eq(local, &self.local));

                    // Try stealing from each local queue in the list.
                    for local in iter {
                        steal(local, &self.local);
                        if let Ok(r) = self.local.pop() {
                            return Some(r);
                        }
                    }
                }

                None
            })
            .await;

        // Bump the tick counter.
        let ticks = self.ticks.fetch_add(1, Ordering::SeqCst);

        if ticks % 64 == 0 {
            // Steal tasks from the global queue to ensure fair task scheduling.
            steal(&self.state.queues[self.priority], &self.local);
        }

        runnable
    }
}

impl Drop for Runner<'_> {
    fn drop(&mut self) {
        // Remove the local queue.
        self.state.local_queues[self.priority]
            .write()
            .retain(|local| !Arc::ptr_eq(local, &self.local));

        // Re-schedule remaining tasks in the local queue.
        while let Ok(r) = self.local.pop() {
            r.schedule();
        }
    }
}

/// Steals some items from one queue into another.
fn steal<T>(src: &ConcurrentQueue<T>, dest: &ConcurrentQueue<T>) {
    // Half of `src`'s length rounded up.
    let mut count = (src.len() + 1) / 2;

    if count > 0 {
        // Don't steal more than fits into the queue.
        if let Some(cap) = dest.capacity() {
            count = count.min(cap - dest.len());
        }

        // Steal tasks.
        for _ in 0..count {
            if let Ok(t) = src.pop() {
                assert!(dest.push(t).is_ok());
            } else {
                break;
            }
        }
    }
}

/// Runs a closure when dropped.
struct CallOnDrop<F: Fn()>(F);

impl<F: Fn()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}
