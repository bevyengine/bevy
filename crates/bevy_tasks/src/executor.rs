// Forked from async_executor

use crate::TaskPool;
use parking_lot::Mutex;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Poll, Waker};

use async_task::Runnable;
use concurrent_queue::ConcurrentQueue;
use futures_lite::{future, prelude::*};
use st3::{Stealer, Worker, B512};

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
    pub fn new(thread_counts: &[usize]) -> Executor<'a> {
        Executor {
            state: Arc::new(State::new(thread_counts)),
            _marker: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T: Send + 'a>(
        &self,
        priority: usize,
        future: impl Future<Output = T> + Send + 'a,
    ) -> Task<T> {
        // Create the task and schedule it
        let (runnable, task) =
            unsafe { async_task::spawn_unchecked(future, self.schedule(priority)) };
        runnable.schedule();
        task
    }

    /// Attempts to run a task if at least one is scheduled.
    pub fn try_tick(&self, priority: usize) -> bool {
        let group = &self.state.groups[priority];
        match group.queue.pop() {
            Err(_) => false,
            Ok(runnable) => {
                // Notify another ticker now to pick up where this ticker left off, just in case
                // running the task takes a long time.
                group.notify();

                // Run the task.
                runnable.run();
                true
            }
        }
    }

    /// Runs the executor until the given future completes.
    pub async fn run<T>(
        &self,
        priority: usize,
        thread_id: usize,
        future: impl Future<Output = T>,
    ) -> T {
        let mut runner = Runner::new(priority, thread_id, &self.state);

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

        move |runnable| {
            let group = &state.groups[priority];
            group.queue.push(runnable).unwrap();
            group.notify();
        }
    }
}

impl Drop for Executor<'_> {
    fn drop(&mut self) {
        for group in self.state.groups.iter() {
            let mut sleepers = group.sleepers.lock();
            for (_, waker) in sleepers.wakers.drain(..) {
                waker.wake();
            }
            drop(sleepers);
            while group.queue.pop().is_ok() {}
        }
    }
}

/// The state of a executor.
#[derive(Debug)]
struct State {
    groups: Box<[GroupState]>,
}

impl State {
    /// Creates state for a new executor.
    fn new(thread_counts: &[usize]) -> State {
        let groups = thread_counts
            .iter()
            .map(|count| {
                let workers = (0..*count)
                    .map(|_| Worker::<Runnable, B512>::new())
                    .collect::<Vec<_>>();
                let stealers = workers
                    .iter()
                    .map(|w| w.stealer())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                // TODO: This is a hack only for initialziation
                // probably should refactor it out.
                let available = ConcurrentQueue::bounded(*count.max(&1));
                for worker in workers {
                    available.push(worker).unwrap();
                }
                GroupState {
                    queue: ConcurrentQueue::unbounded(),
                    stealers,
                    available,
                    searchers: AtomicUsize::new(0),
                    notified: AtomicBool::new(true),
                    sleepers: Mutex::new(Sleepers {
                        count: 0,
                        wakers: Vec::new(),
                        free_ids: Vec::new(),
                    }),
                }
            })
            .collect::<Vec<_>>();
        State {
            groups: groups.into(),
        }
    }
}

#[derive(Debug)]
struct GroupState {
    /// The global queue.
    queue: ConcurrentQueue<Runnable>,

    /// Local queues created by runners.
    stealers: Box<[Stealer<Runnable, B512>]>,

    available: ConcurrentQueue<Worker<Runnable, B512>>,

    searchers: AtomicUsize,

    /// Set to `true` when a sleeping ticker is notified or no tickers are sleeping.
    notified: AtomicBool,

    sleepers: Mutex<Sleepers>,
}

impl GroupState {
    /// Notifies a sleeping ticker. Returns true if the notification search
    /// should continue.
    #[inline]
    fn notify(&self) -> bool {
        if self
            .notified
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let (waker, should_continue) = {
                let mut sleepers = self.sleepers.lock();
                let waker = sleepers.notify();
                (waker, sleepers.is_empty())
            };
            if let Some(w) = waker {
                w.wake();
            }
            return should_continue;
        }
        false
    }

    /// Attempt to start a new search over the queues. Returns
    /// false if there are too many searchers and that the runner
    /// should abort the search.
    #[inline]
    fn start_search(&self) -> bool {
        let searchers = self.searchers.load(Ordering::Acquire);
        if 2 * searchers > self.stealers.len() {
            return false;
        }
        self.searchers.fetch_add(1, Ordering::Release);
        true
    }

    /// Ends a search. Returns true if this is the last searcher
    /// and that the runner wake and notify other runners.
    #[inline]
    fn end_search(&self) -> bool {
        self.searchers.fetch_sub(1, Ordering::Release) == 1
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
        self.count += 1;
        let id = self.free_ids.pop().unwrap_or(self.count);
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

    #[inline]
    fn is_empty(&self) -> bool {
        self.wakers.is_empty()
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

/// A worker in a work-stealing executor.
///
/// This is just a ticker that also has an associated local queue for improved cache locality.
#[derive(Debug)]
struct Runner<'a> {
    priority: usize,
    thread_id: usize,

    /// The executor state.
    group: &'a GroupState,

    /// Set to a non-zero sleeper ID when in sleeping state.
    ///
    /// States a ticker can be in:
    /// 1) Woken.
    /// 2a) Sleeping and unnotified.
    /// 2b) Sleeping and notified.
    sleeping: usize,

    state: &'a State,
    /// The local queue.
    worker: Worker<Runnable, B512>,
    rng: fastrand::Rng,
    /// Bumped every time a runnable task is found.
    ticks: usize,
}

impl Runner<'_> {
    /// Creates a runner and registers it in the executor state.
    fn new(priority: usize, thread_id: usize, state: &State) -> Runner<'_> {
        let group = &state.groups[priority];
        let worker = group.available.pop().unwrap();
        Runner {
            priority,
            thread_id,
            state,
            sleeping: 0,
            worker,
            group,
            rng: fastrand::Rng::new(),
            ticks: 0,
        }
    }

    fn priority_iter(&self) -> impl Iterator<Item = usize> {
        // Prioritize the immediate responsibility of the runner, then search in reverse order
        std::iter::once(self.priority).chain((self.priority + 1..self.state.groups.len()).rev())
    }

    /// Moves the ticker into sleeping and unnotified state.
    ///
    /// Returns `false` if the ticker was already sleeping and unnotified.
    fn sleep(&mut self, waker: &Waker) -> bool {
        let mut sleepers = self.group.sleepers.lock();

        match self.sleeping {
            // Move to sleeping state.
            0 => {
                self.sleeping = sleepers.insert(waker);
            }

            // Already sleeping, check if notified.
            id => {
                if !sleepers.update(id, waker) {
                    return false;
                }
            }
        }
        self.group
            .notified
            .swap(sleepers.is_notified(), Ordering::SeqCst);

        true
    }

    /// Moves the ticker into woken state.
    fn wake(&mut self) {
        let id = self.sleeping;
        self.sleeping = 0;
        if id != 0 {
            let mut sleepers = self.group.sleepers.lock();
            sleepers.remove(id);

            self.group
                .notified
                .swap(sleepers.is_notified(), Ordering::SeqCst);
        }
    }

    /// Waits for the next runnable task to run.
    async fn runnable(&mut self) -> Runnable {
        let runnable = future::poll_fn(|cx| {
            loop {
                // Try the local queue.
                if let Some(r) = self.worker.pop() {
                    self.wake_and_notify();
                    return Poll::Ready(r);
                }

                // Try the local task queue.
                let local_r = TaskPool::LOCAL_EXECUTOR.with(|local| local.try_fetch());
                if let Some(r) = local_r {
                    self.wake_and_notify();
                    return Poll::Ready(r);
                }

                // Try stealing from global queues.
                for priority in self.priority_iter() {
                    let group = &self.state.groups[priority];
                    let stealers = &self.state.groups[priority].stealers;

                    if !group.start_search() {
                        continue;
                    }

                    if let Ok(r) = group.queue.pop() {
                        self.steal(&group.queue);
                        if group.end_search() {
                            self.wake_and_notify();
                        }
                        return Poll::Ready(r);
                    }

                    // // Pick a random starting point in the iterator list and rotate the list.
                    if !stealers.is_empty() {
                        let start = self.rng.usize(..stealers.len());
                        // Try stealing from each local queue in the list.
                        for idx in start..start + stealers.len() {
                            let idx = idx % stealers.len();
                            let stealer = &stealers[idx];
                            if priority == self.priority && idx == self.thread_id {
                                continue;
                            }
                            // Limit the number of higher priority tasks stolen to avoid taking
                            // too many. Higher priority threads can't steal these tasks back.
                            //
                            // Only steal enough such that every other thread in the local priority
                            // can steal one task.
                            let limit = if priority > self.priority {
                                self.group.stealers.len()
                            } else {
                                usize::MAX
                            };
                            let count_fn = |n: usize| ((n + 1) / 2).max(limit);
                            if let Ok((r, _)) = stealer.steal_and_pop(&self.worker, count_fn) {
                                if group.end_search() {
                                    self.wake_and_notify();
                                }
                                return Poll::Ready(r);
                            }
                        }
                    }

                    group.end_search();
                }

                // Move to sleeping and unnotified state.
                if !self.sleep(cx.waker()) {
                    // If already sleeping and unnotified, return.
                    return Poll::Pending;
                }
            }
        })
        .await;

        // Bump the tick counter.
        self.ticks += 1;
        if self.ticks % 64 == 0 {
            // Steal tasks from the global queue to ensure fair task scheduling.
            self.steal(&self.state.groups[self.priority].queue);
        }

        runnable
    }

    fn wake_and_notify(&mut self) {
        // Wake up.
        self.wake();
        // Notify another ticker now to pick up where this ticker left off, just in
        // case running the new task takes a long time.
        for group in self.state.groups[..=self.priority].iter().rev() {
            if !group.notify() {
                return;
            }
        }
    }

    /// Steals some items from one queue into another the local queue.
    fn steal(&self, src: &ConcurrentQueue<Runnable>) {
        if src.is_empty() {
            // Don't steal more than fits into the queue.
            for _ in 0..self.worker.spare_capacity() {
                let Ok(t) = src.pop() else { break };
                let res = self.worker.push(t);
                debug_assert!(res.is_ok());
            }
        }
    }
}

impl Drop for Runner<'_> {
    fn drop(&mut self) {
        // If this ticker is in sleeping state, it must be removed from the sleepers list.
        let id = self.sleeping;
        self.sleeping = 0;
        if id != 0 {
            let mut sleepers = self.group.sleepers.lock();
            let notified = sleepers.remove(id);

            self.group
                .notified
                .swap(sleepers.is_notified(), Ordering::SeqCst);

            // If this ticker was notified, then notify another ticker.
            if notified {
                drop(sleepers);
                self.group.notify();
            }
        }

        // Re-schedule remaining tasks in the local queue.
        while let Some(r) = self.worker.pop() {
            r.schedule();
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
