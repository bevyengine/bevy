//! Fork of `async_executor`.
//!
//! It has been vendored along with its tests to update several outdated dependencies.
//!
//! [`async_executor`]: https://github.com/smol-rs/async-executor

#![expect(
    unsafe_code,
    reason = "Executor code requires unsafe code for dealing with non-'static lifetimes"
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
use core::cell::UnsafeCell;
use core::mem;
use std::thread::{AccessError, ThreadId};

use alloc::collections::VecDeque;
use alloc::fmt;
use async_task::{Builder, Runnable, Task};
use bevy_platform::prelude::Vec;
use bevy_platform::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, TryLockError};
use concurrent_queue::ConcurrentQueue;
use futures_lite::{future,FutureExt};
use slab::Slab;
use thread_local::ThreadLocal;
use crossbeam_utils::CachePadded;

// ThreadLocalState *must* stay `Sync` due to a currently existing soundness hole.
// See: https://github.com/Amanieu/thread_local-rs/issues/75
static THREAD_LOCAL_STATE: ThreadLocal<ThreadLocalState> = ThreadLocal::new();

pub(crate) fn install_runtime_into_current_thread(executor: &Executor) {
    // Use LOCAL_QUEUE here to set the thread destructor
    LOCAL_QUEUE.with(|_| {
        let tls = THREAD_LOCAL_STATE.get_or_default();
        let state = executor.state_as_arc();
        let state_ptr = Arc::into_raw(state).cast_mut();
        let old_ptr = tls.executor.swap(state_ptr, Ordering::Relaxed);
        if !old_ptr.is_null() {
            // SAFETY: If this pointer was not null, it was initialized from Arc::into_raw.
            drop(unsafe { Arc::from_raw(old_ptr) });
        }
    });
}

std::thread_local! {
    static LOCAL_QUEUE: CachePadded<UnsafeCell<LocalQueue>> = const {
        CachePadded::new(UnsafeCell::new(LocalQueue {
            local_queue: VecDeque::new(),
            local_active: Slab::new(),
        }))
    };
}

/// # Safety
/// This must not be accessed at the same time as `LOCAL_QUEUE` in any way.
#[inline(always)]
unsafe fn try_with_local_queue<T>(f: impl FnOnce(&mut LocalQueue) -> T) -> Result<T, AccessError> {
    LOCAL_QUEUE.try_with(|tls| {
        // SAFETY: This value is in thread local storage and thus can only be accessed
        // from one thread. The caller guarantees that this function is not used with
        // LOCAL_QUEUE in any way.
        f(unsafe { &mut *tls.get() })
    })
}

struct LocalQueue {
    local_queue: VecDeque<Runnable>,
    local_active: Slab<Waker>,
}

impl Drop for LocalQueue {
    fn drop(&mut self) {
        for waker in self.local_active.drain() {
            waker.wake();
        }

        while self.local_queue.pop_front().is_some() {}
    }
}

struct ThreadLocalState {
    executor: AtomicPtr<State>,
    stealable_queue: ConcurrentQueue<Runnable>,
    thread_locked_queue: ConcurrentQueue<Runnable>,
}

impl Default for ThreadLocalState {
    fn default() -> Self {
        Self {
            executor: AtomicPtr::new(core::ptr::null_mut()),
            stealable_queue: ConcurrentQueue::bounded(512),
            thread_locked_queue: ConcurrentQueue::unbounded(),
        }
    }
}

/// A task spawner for a specific thread. Must be created by calling [`TaskPool::current_thread_spawner`]
/// from the target thread.
///
/// [`TaskPool::current_thread_spawner`]: crate::TaskPool::current_thread_spawner
#[derive(Clone, Debug)]
pub struct LocalTaskSpawner<'a> {
    thread_id: ThreadId,
    target_queue: &'static ConcurrentQueue<Runnable>,
    state: Arc<State>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> LocalTaskSpawner<'a> {
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
        let result = self.target_queue.push(runnable);
        debug_assert!(result.is_ok());
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
            if unsafe { try_with_local_queue(|tls| tls.local_queue.push_back(runnable)) }.is_ok() {
                state.notify_specific_thread(thread_id, false);
            }
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
        let state = self.state();
        let mut active = state.active();

        // Remove the task from the set of active tasks when the future finishes.
        let entry = active.vacant_entry();
        let index = entry.key();
        let state_arc = self.state_as_arc();
        let future = AsyncCallOnDrop::new(future, move || drop(state_arc.active().try_remove(index)));

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

        // `Runnable::schedule ` has a extra extraneous Waker clone/drop if the schedule captures
        // variables, so directly schedule here instead.
        Self::schedule_runnable(state, runnable);
        task
    }

    /// Spawns a non-Send task onto the executor.
    pub fn spawn_local<T: 'static>(&self, future: impl Future<Output = T> + 'static) -> Task<T> {
        // SAFETY: future is 'static
        unsafe { self.spawn_local_scoped(future) }
    }

    /// Spawns a non-'static and non-Send task onto the executor.
    /// 
    /// # Panics
    ///  - This function will panic if any of the internal allocations causes an OOM (out of memory) error.
    ///  - This function will panic if the current thread's destructor has already been called.
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
        unsafe {
            try_with_local_queue(|tls| {
                let entry = tls.local_active.vacant_entry();
                let index = entry.key();
                let builder = Builder::new().propagate_panic(true);

                // SAFETY: There are no instances where the value is accessed mutably
                // from multiple locations simultaneously. This AsyncCallOnDrop will be
                // invoked after the surrounding scope has exited in either a
                // `try_tick_local` or `run` call.
                let future = AsyncCallOnDrop::new(future, move || {
                    try_with_local_queue(|tls| drop(tls.local_active.try_remove(index))).ok();
                });

                // This is a critical section which will result in UB by aliasing active
                // if the AsyncCallOnDrop is called while still in this function.
                //
                // To avoid this, this guard will abort the process if it does
                // panic. Rust's drop order will ensure that this will run before
                // executor, and thus before the above AsyncCallOnDrop is dropped.
                let _panic_guard = AbortOnPanic;

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
                let (runnable, task) = builder
                    .spawn_unchecked(|()| future, self.schedule_local());
                entry.insert(runnable.waker());

                mem::forget(_panic_guard);

                // Runnable::schedule has a extra extraneous Waker clone/drop if the schedule captures
                // variables, so directly schedule here instead.
                Self::schedule_runnable_local(self.state(), tls, runnable);

                task
            }).unwrap()
        }
    }

    pub fn current_thread_spawner(&self) -> LocalTaskSpawner<'a> {
        LocalTaskSpawner {
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
        unsafe { try_with_local_queue(|tls| tls.local_queue.pop_front()) }
            .ok()
            .flatten()
            .map(Runnable::run)
            .is_some()
    }

    /// Runs the executor until the given future completes.
    pub fn run<'b, T>(&'b self, future: impl Future<Output = T> + 'b) -> impl Future<Output = T> + 'b {
        let mut runner = Runner::new(self.state());

        // A future that runs tasks forever.
        let run_forever = async move {
            let mut rng = fastrand::Rng::new();
            loop {
                for _ in 0..200 {
                    let runnable = runner.runnable(&mut rng).await;
                    runnable.run();
                }
                future::yield_now().await;
            }
        };

        // Run `future` and `run_forever` concurrently until `future` completes.
        future.or(run_forever)
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let state = self.state_as_arc();

        move |runnable| {
            Self::schedule_runnable(&state, runnable);
        }
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule_local(&self) -> impl Fn(Runnable) + 'static {
        let state = self.state_as_arc();
        move |runnable| {
            // SAFETY: This value is in thread local storage and thus can only be accessed
            // from one thread. There are no instances where the value is accessed mutably
            // from multiple locations simultaneously.
            unsafe  {
                // If this returns Err, the thread's destructor has been called and thus it's meaningless
                // to push onto the queue.
                let _ = try_with_local_queue(|tls| {
                    Self::schedule_runnable_local(&state, tls, runnable);
                });
            }
        }
    }

    #[inline]
    fn schedule_runnable(state: &State, runnable: Runnable) {
        // Attempt to push onto the local queue first in dedicated executor threads,
        // because we know that this thread is awake and always processing new tasks.
        let runnable = if let Some(local_state) = THREAD_LOCAL_STATE.get() {
            if core::ptr::eq(local_state.executor.load(Ordering::Relaxed), state) {
                match local_state.stealable_queue.push(runnable) {
                    Ok(()) => {
                        state.notify_specific_thread(std::thread::current().id(), true);
                        return;
                    }
                    Err(r) => r.into_inner(),
                }
            } else {
                runnable
            }
        } else {
            runnable
        };
        // Otherwise push onto the global queue instead.
        let result = state.queue.push(runnable);
        debug_assert!(result.is_ok());
        state.notify();
    }

    #[inline]
    fn schedule_runnable_local(state: &State, tls: &mut LocalQueue, runnable: Runnable) {
        tls.local_queue.push_back(runnable);
        state.notify_specific_thread(std::thread::current().id(), false);
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
        mem::forget(arc);
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

        while state.queue.pop().is_ok() {}
    }
}

/// The state of a executor.
struct State {
    /// The global queue.
    queue: ConcurrentQueue<Runnable>,

    /// Local queues created by runners.
    stealer_queues: RwLock<Vec<&'static ConcurrentQueue<Runnable>>>,

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
            queue: ConcurrentQueue::unbounded(),
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
            let waker = self.sleepers.lock().unwrap_or_else(PoisonError::into_inner).notify();
            if let Some(w) = waker {
                w.wake();
            }
        }
    }

    /// Notifies a sleeping ticker.
    #[inline]
    fn notify_specific_thread(&self, thread_id: ThreadId, allow_stealing: bool) {
        let mut sleepers = self.sleepers.lock().unwrap_or_else(PoisonError::into_inner);
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
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_state(self, "State", f)
    }
}

/// A sleeping ticker
struct Sleeper {
    id: usize,
    thread_id: ThreadId,
    waker: Waker,
}

/// A list of sleeping tickers.
struct Sleepers {
    /// Number of sleeping tickers (both notified and unnotified).
    count: usize,

    /// IDs and wakers of sleeping unnotified tickers.
    ///
    /// A sleeping ticker is notified when its waker is missing from this list.
    wakers: Vec<Sleeper>,

    /// Reclaimed IDs.
    free_ids: Vec<usize>,
}

impl Sleepers {
    /// Inserts a new sleeping ticker.
    fn insert(&mut self, waker: &Waker) -> usize {
        let id = self.free_ids.pop().unwrap_or_else(|| self.count + 1);
        self.count += 1;
        self.wakers.push(Sleeper {
            id,
            thread_id: std::thread::current().id(),
            waker: waker.clone()
        });
        id
    }

    /// Re-inserts a sleeping ticker's waker if it was notified.
    ///
    /// Returns `true` if the ticker was notified.
    fn update(&mut self, id: usize, waker: &Waker) -> bool {
        for item in &mut self.wakers {
            if item.id == id {
                item.waker.clone_from(waker);
                return false;
            }
        }

        self.wakers.push(Sleeper { 
            id, 
            thread_id: std::thread::current().id(), 
            waker: waker.clone() 
        });
        true
    }

    /// Removes a previously inserted sleeping ticker.
    ///
    /// Returns `true` if the ticker was notified.
    fn remove(&mut self, id: usize) -> bool {
        self.count -= 1;
        self.free_ids.push(id);

        for i in (0..self.wakers.len()).rev() {
            if self.wakers[i].id == id {
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
            self.wakers.pop().map(|item| item.waker)
        } else {
            None
        }
    }

    /// Returns notification waker for a sleeping ticker.
    ///
    /// If a ticker was notified already or there are no tickers, `None` will be returned.
    fn notify_specific_thread(&mut self, thread_id: ThreadId) -> Option<Waker> {
        for i in 0..self.wakers.len() {
            if self.wakers[i].thread_id == thread_id {
                let sleeper = self.wakers.remove(i);
                return Some(sleeper.waker);
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
        let mut sleepers = self.state.sleepers.lock().unwrap_or_else(PoisonError::into_inner);

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
            let mut sleepers = self.state.sleepers.lock().unwrap_or_else(PoisonError::into_inner);
            sleepers.remove(self.sleeping);

            self.state
                .notified
                .store(sleepers.is_notified(), Ordering::Release);
        }
        self.sleeping = 0;
    }

    /// Waits for the next runnable task to run, given a function that searches for a task.
    /// 
    /// # Safety
    /// Caller must not access `LOCAL_QUEUE` either directly or with `try_with_local_queue` in any way inside `search`.
    unsafe fn runnable_with(&mut self, mut search: impl FnMut(&mut LocalQueue) -> Option<Runnable>) -> impl Future<Output = Runnable> {
        future::poll_fn(move |cx| {
            // SAFETY: Caller must ensure that there's no instances where LOCAL_QUEUE is accessed mutably
            // from multiple locations simultaneously.
            unsafe {
                try_with_local_queue(|tls| {
                    loop {
                        match search(tls) {
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
                }).unwrap_or(Poll::Pending)
            }
        })
    }
}

impl Drop for Ticker<'_> {
    fn drop(&mut self) {
        // If this ticker is in sleeping state, it must be removed from the sleepers list.
        if self.sleeping != 0 {
            let mut sleepers = self.state.sleepers.lock().unwrap_or_else(PoisonError::into_inner);
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
    local_state: &'a ThreadLocalState,
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
            .unwrap_or_else(PoisonError::into_inner)
            .push(&local_state.stealable_queue);
        runner
    }

    /// Waits for the next runnable task to run.
    fn runnable(&mut self, _rng: &mut fastrand::Rng) -> impl Future<Output = Runnable> {
        // SAFETY: The provided search function does not access LOCAL_QUEUE in any way, and thus cannot 
        // alias.
        let runnable = unsafe {
            self
            .ticker
            .runnable_with(|tls| {
                if let Some(r) = tls.local_queue.pop_back() {
                    return Some(r);
                }

                crate::cfg::multi_threaded! {
                    // Try the local queue.
                    if let Ok(r) = self.local_state.stealable_queue.pop() {
                        return Some(r);
                    }

                    // Try stealing from the global queue.
                    if let Ok(r) = self.state.queue.pop() {
                        steal(&self.state.queue, &self.local_state.stealable_queue);
                        return Some(r);
                    }

                    // Try stealing from other runners.
                    if let Ok(stealer_queues) = self.state.stealer_queues.try_read() {
                        // Pick a random starting point in the iterator list and rotate the list.
                        let n = stealer_queues.len();
                        let start = _rng.usize(..n);
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
                            if let Ok(r) = self.local_state.stealable_queue.pop() {
                                return Some(r);
                            }
                        }
                    }

                    if let Ok(r) = self.local_state.thread_locked_queue.pop() {
                        // Do not steal from this queue. If other threads steal
                        // from this current thread, the task will be moved.
                        //
                        // Instead, flush all queued tasks into the local queue to
                        // minimize the effort required to scan for these tasks.
                        flush_to_local(&self.local_state.thread_locked_queue, tls);
                        return Some(r);
                    }
                }

                None
            })
        };

        // Bump the tick counter.
        self.ticks = self.ticks.wrapping_add(1);

        if self.ticks.is_multiple_of(64) {
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
        while let Ok(r) = self.local_state.stealable_queue.pop() {
            r.schedule();
        }
    }
}

/// Steals some items from one queue into another.
fn steal<T>(src: &ConcurrentQueue<T>, dest: &ConcurrentQueue<T>) {
    // Half of `src`'s length rounded up.
    let mut count = src.len();

    if count > 0 {
        if let Some(capacity) = dest.capacity() {
            // Don't steal more than fits into the queue.
            count = count.min(capacity- dest.len());
        }

        // Steal tasks.
        for _ in 0..count {
            let Ok(val) = src.pop() else { break };
            assert!(dest.push(val).is_ok());
        }
    }
}

fn flush_to_local(src: &ConcurrentQueue<Runnable>, dst: &mut LocalQueue) {
    let count = src.len();

    if count > 0 {
        // Steal tasks.
        for _ in 0..count {
            let Ok(val) = src.pop() else { break };
            dst.local_queue.push_front(val);
        }
    }
}

/// Debug implementation for `Executor`.
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

/// Debug implementation for `Executor`.
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
    struct LocalRunners<'a>(&'a RwLock<Vec<&'static ConcurrentQueue<Runnable>>>);

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

struct AbortOnPanic;

impl Drop for AbortOnPanic {
    fn drop(&mut self) {
        // Panicking while unwinding will force an abort.
        panic!("Aborting due to allocator error");
    }
}

/// Runs a closure when dropped.
struct CallOnDrop<F: FnMut()>(F);

impl<F: FnMut()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

pin_project_lite::pin_project! {
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
    use super::*;
    use super::THREAD_LOCAL_STATE;
    use alloc::{string::String, boxed::Box};
    use futures_lite::{future, pin};
    use std::panic::catch_unwind;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use bevy_platform::sync::Mutex;
    use core::task::{Poll, Waker};
    use async_task::Task;
    use core::time::Duration;

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
    }

    #[test]
    fn executor_cancels_everything() {
        static DROP: AtomicUsize = AtomicUsize::new(0);
        static WAKER: Mutex<Option<Waker>> = Mutex::new(None);

        let ex = Executor::new();

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

        future::block_on(ex.run(async {
            for _ in 0..10 {
                future::yield_now().await;
            }
        }));

        assert!(WAKER.lock().unwrap().is_some());
        assert_eq!(DROP.load(Ordering::SeqCst), 0);

        drop(ex);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);

        assert!(catch_unwind(|| future::block_on(task)).is_err());
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn await_task_after_dropping_executor() {
        let s: String = "hello".into();

        let ex = Executor::new();
        let task: Task<&str> = ex.spawn(async { &*s });
        future::block_on(ex.run(async {
            for _ in 0..10 {
                future::yield_now().await;
            }
        }));


        drop(ex);
        assert_eq!(future::block_on(task), "hello");
        drop(s);
    }

    #[test]
    fn drop_executor_and_then_drop_finished_task() {
        static DROP: AtomicUsize = AtomicUsize::new(0);

        let ex = Executor::new();
        let task = ex.spawn(async {
            CallOnDrop(|| {
                DROP.fetch_add(1, Ordering::SeqCst);
            })
        });
        future::block_on(ex.run(async {
            for _ in 0..10 {
                future::yield_now().await;
            }
        }));

        assert_eq!(DROP.load(Ordering::SeqCst), 0);
        drop(ex);
        assert_eq!(DROP.load(Ordering::SeqCst), 0);
        drop(task);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_finished_task_and_then_drop_executor() {
        static DROP: AtomicUsize = AtomicUsize::new(0);

        let ex = Executor::new();
        let task = ex.spawn(async {
            CallOnDrop(|| {
                DROP.fetch_add(1, Ordering::SeqCst);
            })
        });
        future::block_on(ex.run(async {
            for _ in 0..10 {
                future::yield_now().await;
            }
        }));

        assert_eq!(DROP.load(Ordering::SeqCst), 0);
        drop(task);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
        drop(ex);
        assert_eq!(DROP.load(Ordering::SeqCst), 1);
    }

    fn do_run<Fut: Future<Output = ()>>(mut f: impl FnMut(Arc<Executor<'static>>) -> Fut) {
        // This should not run for longer than two minutes.
        #[cfg(not(miri))]
        let _stop_timeout = {
            let (stop_timeout, stopper) = async_channel::bounded::<()>(1);
            std::thread::spawn(move || {
                future::block_on(async move {
                    #[expect(clippy::print_stderr, reason = "Explicitly used to warn about timed out tests")]
                    let timeout = async {
                        async_io::Timer::after(Duration::from_secs(2 * 60)).await;
                        std::eprintln!("test timed out after 2m");
                        std::process::exit(1)
                    };

                    let _ = stopper.recv().or(timeout).await;
                });
            });
            stop_timeout
        };

        let ex = Arc::new(Executor::new());

        // Test 1: Use the `run` command.
        future::block_on(ex.run(f(ex.clone())));

        // Test 2: Run on many threads.
        std::thread::scope(|scope| {
            let (_signal, shutdown) = async_channel::bounded::<()>(1);

            for _ in 0..16 {
                let shutdown = shutdown.clone();
                let ex = &ex;
                scope.spawn(move || future::block_on(ex.run(shutdown.recv())));
            }

            future::block_on(f(ex.clone()));
        });
    }

    #[test]
    fn smoke() {
        do_run(|ex| async move { ex.spawn(async {}).await });
    }

    #[test]
    fn yield_now() {
        do_run(|ex| async move { ex.spawn(future::yield_now()).await });
    }

    #[test]
    fn timer() {
        do_run(|ex| async move {
            ex.spawn(async_io::Timer::after(Duration::from_millis(5)))
                .await;
        });
    }

    #[test]
    fn test_panic_propagation() {
        let ex = Executor::new();
        let task = ex.spawn(async { panic!("should be caught by the task") });

        // Running the executor should not panic.
        future::block_on(ex.run(async {
            for _ in 0..10 {
                future::yield_now().await;
            }
        }));

        // Polling the task should.
        assert!(future::block_on(task.catch_unwind()).is_err());
    }

    #[test]
    fn two_queues() {
        future::block_on(async {
            // Create an executor with two runners.
            let ex = Executor::new();
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
