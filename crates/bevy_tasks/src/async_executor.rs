#![expect(
    unsafe_code,
    reason = "Executor code requires unsafe code for dealing with non-'static lifetimes"
)]
#![expect(
    clippy::unused_unit,
    reason = "False positive detection on {Async}CallOnDrop"
)]
#![allow(
    dead_code,
    reason = "Not all functions are used with every feature combination"
)]

use core::marker::PhantomData;
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use core::task::{Context, Poll, Waker};
use std::thread::ThreadId;

use alloc::collections::VecDeque;
use alloc::fmt;
use async_task::{Builder, Runnable, Task};
use bevy_platform::prelude::Vec;
use bevy_platform::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, TryLockError};
use crossbeam_queue::{ArrayQueue, SegQueue};
use futures_lite::{future, prelude::*};
use pin_project_lite::pin_project;
use slab::Slab;
use thread_local::ThreadLocal;

// ThreadLocalState *must* stay `Sync` due to a currently existing soundness hole.
// See: https://github.com/Amanieu/thread_local-rs/issues/75
static THREAD_LOCAL_STATE: ThreadLocal<ThreadLocalState> = ThreadLocal::new();

pub(crate) fn install_runtime_into_current_thread() {
    let tls = THREAD_LOCAL_STATE.get_or_default();
    tls.executor_thread.store(true, Ordering::Relaxed);
}

// Do not access this directly, use `with_local_queue` instead.
cfg_if::cfg_if! {
    if #[cfg(all(debug_assertions, not(miri)))] {
        use core::cell::RefCell;

        std::thread_local! {
            static LOCAL_QUEUE: RefCell<LocalQueue> = const {
                RefCell::new(LocalQueue  {
                    local_queue: VecDeque::new(),
                    local_active:Slab::new(),
                })
            };
        }
    } else {
        use core::cell::UnsafeCell;

        std::thread_local! {
            static LOCAL_QUEUE: UnsafeCell<LocalQueue> = const {
                UnsafeCell::new(LocalQueue  {
                    local_queue: VecDeque::new(),
                    local_active:Slab::new(),
                })
            };
        }
    }
}

/// # Safety
/// This must not be accessed at the same time as `LOCAL_QUEUE` in any way.
#[inline(always)]
unsafe fn with_local_queue<T>(f: impl FnOnce(&mut LocalQueue) -> T) -> T {
    LOCAL_QUEUE.with(|tls| {
        cfg_if::cfg_if! {
            if #[cfg(all(debug_assertions, not(miri)))] {
                f(&mut tls.borrow_mut())
            } else {
                // SAFETY: This value is in thread local storage and thus can only be accessed
                // from one thread. The caller guarantees that this function is not used with
                // LOCAL_QUEUE in any way.
                f(unsafe { &mut *tls.get() })
            }
        }
    })
}

struct LocalQueue {
    local_queue: VecDeque<Runnable>,
    local_active: Slab<Waker>,
}

struct ThreadLocalState {
    executor_thread: AtomicBool,
    thread_id: ThreadId,
    stealable_queue: ArrayQueue<Runnable>,
    thread_locked_queue: SegQueue<Runnable>,
}

impl Default for ThreadLocalState {
    fn default() -> Self {
        Self {
            executor_thread: AtomicBool::new(false),
            thread_id: std::thread::current().id(),
            stealable_queue: ArrayQueue::new(512),
            thread_locked_queue: SegQueue::new(),
        }
    }
}

/// A task spawner for a specific thread. Must be created by calling [`TaskPool::current_thread_spawner`]
/// from the target thread.
///
/// [`TaskPool::current_thread_spawner`]: crate::TaskPool::current_thread_spawner
#[derive(Clone, Debug)]
pub struct ThreadSpawner<'a> {
    thread_id: ThreadId,
    target_queue: &'static SegQueue<Runnable>,
    state: Arc<State>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> ThreadSpawner<'a> {
    /// Spawns a task onto the specific target thread.
    pub fn spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T> {
        // SAFETY: T and `future` are both 'static, so the Task is guaranteed to not outlive it.
        unsafe { self.spawn_scoped(future) }
    }

    /// Spawns a task onto the executor.
    ///
    /// # Safety
    /// The caller must ensure that the returned Task does not outlive 'a.
    pub unsafe fn spawn_scoped<T: Send + 'a>(
        &self,
        future: impl Future<Output = T> + Send + 'a,
    ) -> Task<T> {
        let mut active = self.state.active();

        // Remove the task from the set of active tasks when the future finishes.
        let entry = active.vacant_entry();
        let index = entry.key();
        let state = self.state.clone();
        let future = AsyncCallOnDrop::new(future, move || drop(state.active().try_remove(index)));

        // Create the task and register it in the set of active tasks.
        //
        // SAFETY:
        //
        // - `future` is `Send`. Therefore we do not need to worry about what thread
        //   the produced `Runnable` is used and dropped from.
        // - `future` is not `'static`, but the caller must make sure that the Task
        //   and thus the `Runnable` will not outlive `'a`.
        // - `self.schedule()` is `Send`, `Sync` and `'static`, as checked below.
        //   Therefore we do not need to worry about what is done with the
        //   `Waker`.
        let (runnable, task) = unsafe {
            Builder::new()
                .propagate_panic(true)
                .spawn_unchecked(|()| future, self.schedule())
        };
        entry.insert(runnable.waker());

        // Instead of directly scheduling this task, it's put into the onto the
        // thread locked queue to be moved to the target thread, where it will
        // either be run immediately or flushed into the thread's local queue.
        self.target_queue.push(runnable);
        task
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let thread_id = self.thread_id;
        let state = self.state.clone();

        move |runnable| {
            // SAFETY: This value is in thread local storage and thus can only be accessed
            // from one thread. There are no instances where the value is accessed mutably
            // from multiple locations simultaneously.
            unsafe {
                with_local_queue(|tls| tls.local_queue.push_back(runnable));
            }
            state.notify_specific_thread(thread_id, false);
        }
    }
}

/// An async executor.
pub struct Executor<'a> {
    /// The executor state.
    state: AtomicPtr<State>,

    /// Makes the `'a` lifetime invariant.
    _marker: PhantomData<&'a ()>,
}

impl UnwindSafe for Executor<'_> {}
impl RefUnwindSafe for Executor<'_> {}

impl fmt::Debug for Executor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_executor(self, "Executor", f)
    }
}

impl<'a> Executor<'a> {
    /// Creates a new executor.
    pub const fn new() -> Executor<'a> {
        Executor {
            state: AtomicPtr::new(core::ptr::null_mut()),
            _marker: PhantomData,
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T: Send + 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        let mut active = self.state().active();

        // Remove the task from the set of active tasks when the future finishes.
        let entry = active.vacant_entry();
        let index = entry.key();
        let state = self.state_as_arc();
        let future = AsyncCallOnDrop::new(future, move || drop(state.active().try_remove(index)));

        // Create the task and register it in the set of active tasks.
        //
        // SAFETY:
        //
        // - `future` is `Send`. Therefore we do not need to worry about what thread
        //   the produced `Runnable` is used and dropped from.
        // - `future` is not `'static`, but we make sure that the `Runnable` does
        //   not outlive `'a`. When the executor is dropped, the `active` field is
        //   drained and all of the `Waker`s are woken. Then, the queue inside of
        //   the `Executor` is drained of all of its runnables. This ensures that
        //   runnables are dropped and this precondition is satisfied.
        // - `self.schedule()` is `Send`, `Sync` and `'static`, as checked below.
        //   Therefore we do not need to worry about what is done with the
        //   `Waker`.
        let (runnable, task) = unsafe {
            Builder::new()
                .propagate_panic(true)
                .spawn_unchecked(|()| future, self.schedule())
        };
        entry.insert(runnable.waker());

        runnable.schedule();
        task
    }

    /// Spawns a non-Send task onto the executor.
    pub fn spawn_local<T: 'static>(&self, future: impl Future<Output = T> + 'static) -> Task<T> {
        // SAFETY: future is 'static
        unsafe { self.spawn_local_scoped(future) }
    }

    /// Spawns a non-'static and non-Send task onto the executor.
    ///
    /// # Safety
    /// The caller must ensure that the returned Task does not outlive 'a.
    pub unsafe fn spawn_local_scoped<T: 'a>(
        &self,
        future: impl Future<Output = T> + 'a,
    ) -> Task<T> {
        // Remove the task from the set of active tasks when the future finishes.
        //
        // SAFETY: There are no instances where the value is accessed mutably
        // from multiple locations simultaneously.
        let (runnable, task) = unsafe {
            with_local_queue(|tls| {
                let entry = tls.local_active.vacant_entry();
                let index = entry.key();
                // SAFETY: There are no instances where the value is accessed mutably
                // from multiple locations simultaneously. This AsyncCallOnDrop will be
                // invoked after the surrounding scope has exited in either a
                // `try_tick_local` or `run` call.
                let future = AsyncCallOnDrop::new(future, move || {
                    with_local_queue(|tls| drop(tls.local_active.try_remove(index)));
                });

                // Create the task and register it in the set of active tasks.
                //
                // SAFETY:
                //
                // - `future` is not `Send`, but the produced `Runnable` does is bound
                //   to thread-local storage and thus cannot leave this thread of execution.
                // - `future` may not be `'static`, but the caller is required to ensure that
                //   the future does not outlive the borrowed non-metadata variables of the
                //   task.
                // - `self.schedule_local()` is not `Send` or `Sync` so all instances
                //   must not leave the current thread of execution, and it does not
                //   all of them are bound vy use of thread-local storage.
                // - `self.schedule_local()` is `'static`, as checked below.
                let (runnable, task) = Builder::new()
                    .propagate_panic(true)
                    .spawn_unchecked(|()| future, self.schedule_local());
                entry.insert(runnable.waker());

                (runnable, task)
            })
        };

        runnable.schedule();
        task
    }

    pub fn current_thread_spawner(&self) -> ThreadSpawner<'a> {
        ThreadSpawner {
            thread_id: std::thread::current().id(),
            target_queue: &THREAD_LOCAL_STATE.get_or_default().thread_locked_queue,
            state: self.state_as_arc(),
            _marker: PhantomData,
        }
    }

    pub fn try_tick_local() -> bool {
        // SAFETY: There are no instances where the value is accessed mutably
        // from multiple locations simultaneously. As the Runnable is run after
        // this scope closes, the AsyncCallOnDrop around the future will be invoked
        // without overlapping mutable accssses.
        unsafe { with_local_queue(|tls| tls.local_queue.pop_front()) }
            .map(Runnable::run)
            .is_some()
    }

    /// Runs the executor until the given future completes.
    pub async fn run<T>(&self, future: impl Future<Output = T>) -> T {
        self.state().run(future).await
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let state = self.state_as_arc();

        move |runnable| {
            // Attempt to push onto the local queue first in dedicated executor threads,
            // because we know that this thread is awake and always processing new tasks.
            let runnable = if let Some(local_state) = THREAD_LOCAL_STATE.get() {
                if local_state.executor_thread.load(Ordering::Relaxed) {
                    match local_state.stealable_queue.push(runnable) {
                        Ok(()) => {
                            state.notify_specific_thread(local_state.thread_id, true);
                            return;
                        }
                        Err(r) => r,
                    }
                } else {
                    runnable
                }
            } else {
                runnable
            };
            // Otherwise push onto the global queue instead.
            state.queue.push(runnable);
            state.notify();
        }
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule_local(&self) -> impl Fn(Runnable) + 'static {
        let state = self.state_as_arc();
        let local_state: &'static ThreadLocalState = THREAD_LOCAL_STATE.get_or_default();
        move |runnable| {
            // SAFETY: This value is in thread local storage and thus can only be accessed
            // from one thread. There are no instances where the value is accessed mutably
            // from multiple locations simultaneously.
            unsafe {
                with_local_queue(|tls| tls.local_queue.push_back(runnable));
            }
            state.notify_specific_thread(local_state.thread_id, false);
        }
    }

    /// Returns a pointer to the inner state.
    #[inline]
    fn state_ptr(&self) -> *const State {
        #[cold]
        fn alloc_state(atomic_ptr: &AtomicPtr<State>) -> *mut State {
            let state = Arc::new(State::new());
            let ptr = Arc::into_raw(state).cast_mut();
            if let Err(actual) = atomic_ptr.compare_exchange(
                core::ptr::null_mut(),
                ptr,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                // SAFETY: This was just created from Arc::into_raw.
                drop(unsafe { Arc::from_raw(ptr) });
                actual
            } else {
                ptr
            }
        }

        let mut ptr = self.state.load(Ordering::Acquire);
        if ptr.is_null() {
            ptr = alloc_state(&self.state);
        }
        ptr
    }

    /// Returns a reference to the inner state.
    #[inline]
    fn state(&self) -> &State {
        // SAFETY: So long as an Executor lives, it's state pointer will always be valid
        // when accessed through state_ptr.
        unsafe { &*self.state_ptr() }
    }

    // Clones the inner state Arc
    #[inline]
    fn state_as_arc(&self) -> Arc<State> {
        // SAFETY: So long as an Executor lives, it's state pointer will always be a valid
        // Arc when accessed through state_ptr.
        let arc = unsafe { Arc::from_raw(self.state_ptr()) };
        let clone = arc.clone();
        core::mem::forget(arc);
        clone
    }
}

impl Drop for Executor<'_> {
    fn drop(&mut self) {
        let ptr = *self.state.get_mut();
        if ptr.is_null() {
            return;
        }

        // SAFETY: As ptr is not null, it was allocated via Arc::new and converted
        // via Arc::into_raw in state_ptr.
        let state = unsafe { Arc::from_raw(ptr) };

        let mut active = state.active();
        for w in active.drain() {
            w.wake();
        }
        drop(active);

        while state.queue.pop().is_some() {}
    }
}

/// The state of a executor.
struct State {
    /// The global queue.
    queue: SegQueue<Runnable>,

    /// Local queues created by runners.
    stealer_queues: RwLock<Vec<&'static ArrayQueue<Runnable>>>,

    /// Set to `true` when a sleeping ticker is notified or no tickers are sleeping.
    notified: AtomicBool,

    /// A list of sleeping tickers.
    sleepers: Mutex<Sleepers>,

    /// Currently active tasks.
    active: Mutex<Slab<Waker>>,
}

impl State {
    /// Creates state for a new executor.
    const fn new() -> State {
        State {
            queue: SegQueue::new(),
            stealer_queues: RwLock::new(Vec::new()),
            notified: AtomicBool::new(true),
            sleepers: Mutex::new(Sleepers {
                count: 0,
                wakers: Vec::new(),
                free_ids: Vec::new(),
            }),
            active: Mutex::new(Slab::new()),
        }
    }

    /// Returns a reference to currently active tasks.
    fn active(&self) -> MutexGuard<'_, Slab<Waker>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Notifies a sleeping ticker.
    #[inline]
    fn notify(&self) {
        if self
            .notified
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let waker = self.sleepers.lock().unwrap().notify();
            if let Some(w) = waker {
                w.wake();
            }
        }
    }

    /// Notifies a sleeping ticker.
    #[inline]
    fn notify_specific_thread(&self, thread_id: ThreadId, allow_stealing: bool) {
        let mut sleepers = self.sleepers.lock().unwrap();
        let mut waker = sleepers.notify_specific_thread(thread_id);
        if waker.is_none()
            && allow_stealing
            && self
                .notified
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            waker = sleepers.notify();
        }
        if let Some(w) = waker {
            w.wake();
        }
    }

    pub async fn run<T>(&self, future: impl Future<Output = T>) -> T {
        let mut runner = Runner::new(self);
        let mut rng = fastrand::Rng::new();

        // A future that runs tasks forever.
        let run_forever = async {
            loop {
                for _ in 0..200 {
                    let runnable = runner.runnable(&mut rng).await;
                    runnable.run();
                }
                future::yield_now().await;
            }
        };

        // Run `future` and `run_forever` concurrently until `future` completes.
        future.or(run_forever).await
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_state(self, "State", f)
    }
}

/// A list of sleeping tickers.
struct Sleepers {
    /// Number of sleeping tickers (both notified and unnotified).
    count: usize,

    /// IDs and wakers of sleeping unnotified tickers.
    ///
    /// A sleeping ticker is notified when its waker is missing from this list.
    wakers: Vec<(usize, ThreadId, Waker)>,

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
        self.wakers
            .push((id, std::thread::current().id(), waker.clone()));
        id
    }

    /// Re-inserts a sleeping ticker's waker if it was notified.
    ///
    /// Returns `true` if the ticker was notified.
    fn update(&mut self, id: usize, waker: &Waker) -> bool {
        for item in &mut self.wakers {
            if item.0 == id {
                item.2.clone_from(waker);
                return false;
            }
        }

        self.wakers
            .push((id, std::thread::current().id(), waker.clone()));
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
            self.wakers.pop().map(|item| item.2)
        } else {
            None
        }
    }

    /// Returns notification waker for a sleeping ticker.
    ///
    /// If a ticker was notified already or there are no tickers, `None` will be returned.
    fn notify_specific_thread(&mut self, thread_id: ThreadId) -> Option<Waker> {
        for i in (0..self.wakers.len()).rev() {
            if self.wakers[i].1 == thread_id {
                let (_, _, waker) = self.wakers.remove(i);
                return Some(waker);
            }
        }
        None
    }
}

/// Runs task one by one.
struct Ticker<'a> {
    /// The executor state.
    state: &'a State,

    /// Set to a non-zero sleeper ID when in sleeping state.
    ///
    /// States a ticker can be in:
    /// 1) Woken.
    ///    2a) Sleeping and unnotified.
    ///    2b) Sleeping and notified.
    sleeping: usize,
}

impl Ticker<'_> {
    /// Creates a ticker.
    fn new(state: &State) -> Ticker<'_> {
        Ticker { state, sleeping: 0 }
    }

    /// Moves the ticker into sleeping and unnotified state.
    ///
    /// Returns `false` if the ticker was already sleeping and unnotified.
    fn sleep(&mut self, waker: &Waker) -> bool {
        let mut sleepers = self.state.sleepers.lock().unwrap();

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

        self.state
            .notified
            .store(sleepers.is_notified(), Ordering::Release);

        true
    }

    /// Moves the ticker into woken state.
    fn wake(&mut self) {
        if self.sleeping != 0 {
            let mut sleepers = self.state.sleepers.lock().unwrap();
            sleepers.remove(self.sleeping);

            self.state
                .notified
                .store(sleepers.is_notified(), Ordering::Release);
        }
        self.sleeping = 0;
    }

    /// Waits for the next runnable task to run, given a function that searches for a task.
    async fn runnable_with(&mut self, mut search: impl FnMut() -> Option<Runnable>) -> Runnable {
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
        if self.sleeping != 0 {
            let mut sleepers = self.state.sleepers.lock().unwrap();
            let notified = sleepers.remove(self.sleeping);

            self.state
                .notified
                .store(sleepers.is_notified(), Ordering::Release);

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
struct Runner<'a> {
    /// The executor state.
    state: &'a State,

    /// Inner ticker.
    ticker: Ticker<'a>,

    /// Bumped every time a runnable task is found.
    ticks: usize,

    // The thread local state of the executor for the current thread.
    local_state: &'static ThreadLocalState,
}

impl Runner<'_> {
    /// Creates a runner and registers it in the executor state.
    fn new(state: &State) -> Runner<'_> {
        let local_state = THREAD_LOCAL_STATE.get_or_default();
        let runner = Runner {
            state,
            ticker: Ticker::new(state),
            ticks: 0,
            local_state,
        };
        state
            .stealer_queues
            .write()
            .unwrap()
            .push(&local_state.stealable_queue);
        runner
    }

    /// Waits for the next runnable task to run.
    async fn runnable(&mut self, rng: &mut fastrand::Rng) -> Runnable {
        let runnable = self
            .ticker
            .runnable_with(|| {
                // SAFETY: There are no instances where the value is accessed mutably
                // from multiple locations simultaneously.
                let local_pop = unsafe { with_local_queue(|tls| tls.local_queue.pop_front()) };
                if let Some(r) = local_pop {
                    return Some(r);
                }

                // Try the local queue.
                if let Some(r) = self.local_state.stealable_queue.pop() {
                    return Some(r);
                }

                // Try stealing from the global queue.
                if let Some(r) = self.state.queue.pop() {
                    steal(&self.state.queue, &self.local_state.stealable_queue);
                    return Some(r);
                }

                // Try stealing from other runners.
                let stealer_queues = self.state.stealer_queues.read().unwrap();

                // Pick a random starting point in the iterator list and rotate the list.
                let n = stealer_queues.len();
                let start = rng.usize(..n);
                let iter = stealer_queues
                    .iter()
                    .chain(stealer_queues.iter())
                    .skip(start)
                    .take(n);

                // Remove this runner's local queue.
                let iter =
                    iter.filter(|local| !core::ptr::eq(**local, &self.local_state.stealable_queue));

                // Try stealing from each local queue in the list.
                for local in iter {
                    steal(*local, &self.local_state.stealable_queue);
                    if let Some(r) = self.local_state.stealable_queue.pop() {
                        return Some(r);
                    }
                }

                if let Some(r) = self.local_state.thread_locked_queue.pop() {
                    // Do not steal from this queue. If other threads steal
                    // from this current thread, the task will be moved.
                    //
                    // Instead, flush all queued tasks into the local queue to
                    // minimize the effort required to scan for these tasks.
                    //
                    // SAFETY: This is not being used at the same time as any
                    // access to LOCAL_QUEUE.
                    unsafe { flush_to_local(&self.local_state.thread_locked_queue) };
                    return Some(r);
                }

                None
            })
            .await;

        // Bump the tick counter.
        self.ticks = self.ticks.wrapping_add(1);

        if self.ticks % 64 == 0 {
            // Steal tasks from the global queue to ensure fair task scheduling.
            steal(&self.state.queue, &self.local_state.stealable_queue);
        }

        runnable
    }
}

impl Drop for Runner<'_> {
    fn drop(&mut self) {
        // Remove the local queue.
        {
            let mut stealer_queues = self.state.stealer_queues.write().unwrap();
            if let Some((idx, _)) = stealer_queues
                .iter()
                .enumerate()
                .rev()
                .find(|(_, local)| core::ptr::eq(**local, &self.local_state.stealable_queue))
            {
                stealer_queues.remove(idx);
            }
        }

        // Re-schedule remaining tasks in the local queue.
        while let Some(r) = self.local_state.stealable_queue.pop() {
            r.schedule();
        }
    }
}

trait WorkQueue<T> {
    fn stealable_count(&self) -> usize;
    fn queue_pop(&self) -> Option<T>;
}

impl<T> WorkQueue<T> for ArrayQueue<T> {
    #[inline]
    fn stealable_count(&self) -> usize {
        self.len().div_ceil(2)
    }

    #[inline]
    fn queue_pop(&self) -> Option<T> {
        self.pop()
    }
}

impl<T> WorkQueue<T> for SegQueue<T> {
    #[inline]
    fn stealable_count(&self) -> usize {
        self.len()
    }

    #[inline]
    fn queue_pop(&self) -> Option<T> {
        self.pop()
    }
}

/// Steals some items from one queue into another.
fn steal<T, Q: WorkQueue<T>>(src: &Q, dest: &ArrayQueue<T>) {
    // Half of `src`'s length rounded up.
    let mut count = src.stealable_count();

    if count > 0 {
        // Don't steal more than fits into the queue.
        count = count.min(dest.capacity() - dest.len());

        // Steal tasks.
        for _ in 0..count {
            let Some(val) = src.queue_pop() else { break };
            assert!(dest.push(val).is_ok());
        }
    }
}

/// Flushes all of the items from a queue into the thread local queue.
///
/// # Safety
/// This must not be accessed at the same time as `LOCAL_QUEUE` in any way.
unsafe fn flush_to_local(src: &SegQueue<Runnable>) {
    let count = src.len();

    if count > 0 {
        // SAFETY: Caller assures that `LOCAL_QUEUE` does not have any
        // overlapping accesses.
        unsafe {
            with_local_queue(|tls| {
                // Steal tasks.
                for _ in 0..count {
                    let Some(val) = src.queue_pop() else { break };
                    tls.local_queue.push_front(val);
                }
            });
        }
    }
}

/// Debug implementation for `Executor` and `LocalExecutor`.
fn debug_executor(executor: &Executor<'_>, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Get a reference to the state.
    let ptr = executor.state.load(Ordering::Acquire);
    if ptr.is_null() {
        // The executor has not been initialized.
        struct Uninitialized;

        impl fmt::Debug for Uninitialized {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<uninitialized>")
            }
        }

        return f.debug_tuple(name).field(&Uninitialized).finish();
    }

    // SAFETY: If the state pointer is not null, it must have been
    // allocated properly by Arc::new and converted via Arc::into_raw
    // in state_ptr.
    let state = unsafe { &*ptr };

    debug_state(state, name, f)
}

/// Debug implementation for `Executor` and `LocalExecutor`.
fn debug_state(state: &State, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    /// Debug wrapper for the number of active tasks.
    struct ActiveTasks<'a>(&'a Mutex<Slab<Waker>>);

    impl fmt::Debug for ActiveTasks<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0.try_lock() {
                Ok(lock) => fmt::Debug::fmt(&lock.len(), f),
                Err(TryLockError::WouldBlock) => f.write_str("<locked>"),
                Err(TryLockError::Poisoned(err)) => fmt::Debug::fmt(&err.into_inner().len(), f),
            }
        }
    }

    /// Debug wrapper for the local runners.
    struct LocalRunners<'a>(&'a RwLock<Vec<&'static ArrayQueue<Runnable>>>);

    impl fmt::Debug for LocalRunners<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0.try_read() {
                Ok(lock) => f
                    .debug_list()
                    .entries(lock.iter().map(|queue| queue.len()))
                    .finish(),
                Err(TryLockError::WouldBlock) => f.write_str("<locked>"),
                Err(TryLockError::Poisoned(_)) => f.write_str("<poisoned>"),
            }
        }
    }

    /// Debug wrapper for the sleepers.
    struct SleepCount<'a>(&'a Mutex<Sleepers>);

    impl fmt::Debug for SleepCount<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0.try_lock() {
                Ok(lock) => fmt::Debug::fmt(&lock.count, f),
                Err(TryLockError::WouldBlock) => f.write_str("<locked>"),
                Err(TryLockError::Poisoned(_)) => f.write_str("<poisoned>"),
            }
        }
    }

    f.debug_struct(name)
        .field("active", &ActiveTasks(&state.active))
        .field("global_tasks", &state.queue.len())
        .field("stealer_queues", &LocalRunners(&state.stealer_queues))
        .field("sleepers", &SleepCount(&state.sleepers))
        .finish()
}

/// Runs a closure when dropped.
struct CallOnDrop<F: FnMut()>(F);

impl<F: FnMut()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

pin_project! {
    /// A wrapper around a future, running a closure when dropped.
    struct AsyncCallOnDrop<Fut, Cleanup: FnMut()> {
        #[pin]
        future: Fut,
        cleanup: CallOnDrop<Cleanup>,
    }
}

impl<Fut, Cleanup: FnMut()> AsyncCallOnDrop<Fut, Cleanup> {
    fn new(future: Fut, cleanup: Cleanup) -> Self {
        Self {
            future,
            cleanup: CallOnDrop(cleanup),
        }
    }
}

impl<Fut: Future, Cleanup: FnMut()> Future for AsyncCallOnDrop<Fut, Cleanup> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }
}

#[cfg(test)]
mod test {
    use super::Executor;
    use super::THREAD_LOCAL_STATE;

    fn _ensure_send_and_sync() {
        fn is_send<T: Send>(_: T) {}
        fn is_sync<T: Sync>(_: T) {}
        fn is_static<T: 'static>(_: T) {}

        is_send::<Executor<'_>>(Executor::new());
        is_sync::<Executor<'_>>(Executor::new());

        let ex = Executor::new();
        is_send(ex.schedule());
        is_sync(ex.schedule());
        is_static(ex.schedule());
        is_send(ex.current_thread_spawner());
        is_sync(ex.current_thread_spawner());
        is_send(THREAD_LOCAL_STATE.get_or_default());
        is_sync(THREAD_LOCAL_STATE.get_or_default());

        /// ```compile_fail
        /// use crate::async_executor::LocalExecutor;
        /// use futures_lite::future::pending;
        ///
        /// fn is_send<T: Send>(_: T) {}
        /// fn is_sync<T: Sync>(_: T) {}
        ///
        /// is_send::<LocalExecutor<'_>>(LocalExecutor::new());
        /// is_sync::<LocalExecutor<'_>>(LocalExecutor::new());
        ///
        /// let ex = LocalExecutor::new();
        /// is_send(ex.run(pending::<()>()));
        /// is_sync(ex.run(pending::<()>()));
        /// is_send(ex.tick());
        /// is_sync(ex.tick());
        /// ```
        fn _negative_test() {}
    }
}
