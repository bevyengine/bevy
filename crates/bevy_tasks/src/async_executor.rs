use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, TryLockError};
use std::task::{Context, Poll, Waker};
use std::thread::{Thread, ThreadId};

use async_task::{Builder, Runnable, Task};
use bevy_platform::prelude::Vec;
use concurrent_queue::ConcurrentQueue;
use futures_lite::{future, prelude::*};
use pin_project_lite::pin_project;
use slab::Slab;
use thread_local::ThreadLocal;

static THREAD_LOCAL_STATE: ThreadLocal<ThreadLocalState> = ThreadLocal::new();

struct ThreadLocalState {
    thread_id: ThreadId,
    thread_locked_queue: ConcurrentQueue<Runnable>,
    local_queue: RefCell<VecDeque<Runnable>>,
    local_active: RefCell<Slab<Waker>>,
}

impl Default for ThreadLocalState {
    fn default() -> Self {
        Self {
            thread_id: std::thread::current().id(),
            thread_locked_queue: ConcurrentQueue::unbounded(),
            local_queue: RefCell::new(VecDeque::new()),
            local_active: RefCell::new(Slab::new()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ThreadSpawner<'a> {
    thread_id: ThreadId,
    target_queue: &'static ConcurrentQueue<Runnable>,
    state: Arc<State>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> ThreadSpawner<'a> {
    /// Spawns a task onto the executor.
    pub fn spawn<T: Send + 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        let mut active = self.state.active();

        // Remove the task from the set of active tasks when the future finishes.
        let entry = active.vacant_entry();
        let index = entry.key();
        let state = self.state.clone();
        let future = AsyncCallOnDrop::new(future, move || drop(state.active().try_remove(index)));

        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // Create the task and register it in the set of active tasks.
        //
        // SAFETY:
        //
        // If `future` is not `Send`, this must be a `LocalExecutor` as per this
        // function's unsafe precondition. Since `LocalExecutor` is `!Sync`,
        // `try_tick`, `tick` and `run` can only be called from the origin
        // thread of the `LocalExecutor`. Similarly, `spawn` can only  be called
        // from the origin thread, ensuring that `future` and the executor share
        // the same origin thread. The `Runnable` can be scheduled from other
        // threads, but because of the above `Runnable` can only be called or
        // dropped on the origin thread.
        //
        // `future` is not `'static`, but we make sure that the `Runnable` does
        // not outlive `'a`. When the executor is dropped, the `active` field is
        // drained and all of the `Waker`s are woken. Then, the queue inside of
        // the `Executor` is drained of all of its runnables. This ensures that
        // runnables are dropped and this precondition is satisfied.
        //
        // `self.schedule()` is `Send`, `Sync` and `'static`, as checked below.
        // Therefore we do not need to worry about what is done with the
        // `Waker`.
        let (runnable, task) = unsafe {
            Builder::new()
                .propagate_panic(true)
                .spawn_unchecked(|()| future, self.schedule())
        };
        entry.insert(runnable.waker());

        runnable.schedule();
        task
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let thread_id = self.thread_id;
        let queue: &'static ConcurrentQueue<Runnable> = self.target_queue;
        let state = self.state.clone();

        move |runnable| {
            queue.push(runnable).unwrap();
            state.notify_specific_thread(thread_id);
        }
    }
}

/// An async executor.
pub struct Executor<'a> {
    /// The executor state.
    state: AtomicPtr<State>,

    /// Makes the `'a` lifetime invariant.
    _marker: PhantomData<std::cell::UnsafeCell<&'a ()>>,
}

#[expect(
    unsafe_code,
    reason = "unsized coercion is an unstable feature for non-std types"
)]
// SAFETY: Executor stores no thread local state that can be accessed via other thread.
unsafe impl Send for Executor<'_> {}
#[expect(
    unsafe_code,
    reason = "unsized coercion is an unstable feature for non-std types"
)]
// SAFETY: Executor internally synchronizes all of it's operations internally.
unsafe impl Sync for Executor<'_> {}

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
            state: AtomicPtr::new(std::ptr::null_mut()),
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

        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // Create the task and register it in the set of active tasks.
        //
        // SAFETY:
        //
        // If `future` is not `Send`, this must be a `LocalExecutor` as per this
        // function's unsafe precondition. Since `LocalExecutor` is `!Sync`,
        // `try_tick`, `tick` and `run` can only be called from the origin
        // thread of the `LocalExecutor`. Similarly, `spawn` can only  be called
        // from the origin thread, ensuring that `future` and the executor share
        // the same origin thread. The `Runnable` can be scheduled from other
        // threads, but because of the above `Runnable` can only be called or
        // dropped on the origin thread.
        //
        // `future` is not `'static`, but we make sure that the `Runnable` does
        // not outlive `'a`. When the executor is dropped, the `active` field is
        // drained and all of the `Waker`s are woken. Then, the queue inside of
        // the `Executor` is drained of all of its runnables. This ensures that
        // runnables are dropped and this precondition is satisfied.
        //
        // `self.schedule()` is `Send`, `Sync` and `'static`, as checked below.
        // Therefore we do not need to worry about what is done with the
        // `Waker`.
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
    pub fn spawn_local<T: 'a>(&self, future: impl Future<Output = T> + 'a) -> Task<T> {
        // Remove the task from the set of active tasks when the future finishes.
        let local_state: &'static ThreadLocalState = THREAD_LOCAL_STATE.get_or_default();
        let mut local_active = local_state.local_active.borrow_mut();
        let entry = local_active.vacant_entry();
        let index = entry.key();
        let future = AsyncCallOnDrop::new(future, move || {
            drop(local_state.local_active.borrow_mut().try_remove(index))
        });

        #[expect(
            unsafe_code,
            reason = "Builder::spawn_local requires a 'static lifetime"
        )]
        // Create the task and register it in the set of active tasks.
        //
        // SAFETY:
        //
        // If `future` is not `Send`, this must be a `LocalExecutor` as per this
        // function's unsafe precondition. Since `LocalExecutor` is `!Sync`,
        // `try_tick`, `tick` and `run` can only be called from the origin
        // thread of the `LocalExecutor`. Similarly, `spawn` can only  be called
        // from the origin thread, ensuring that `future` and the executor share
        // the same origin thread. The `Runnable` can be scheduled from other
        // threads, but because of the above `Runnable` can only be called or
        // dropped on the origin thread.
        //
        // `future` is not `'static`, but we make sure that the `Runnable` does
        // not outlive `'a`. When the executor is dropped, the `active` field is
        // drained and all of the `Waker`s are woken. Then, the queue inside of
        // the `Executor` is drained of all of its runnables. This ensures that
        // runnables are dropped and this precondition is satisfied.
        //
        // `self.schedule()` is `Send`, `Sync` and `'static`, as checked below.
        // Therefore we do not need to worry about what is done with the
        // `Waker`.
        let (runnable, task) = unsafe {
            Builder::new()
                .propagate_panic(true)
                .spawn_unchecked(|()| future, self.schedule_local())
        };
        entry.insert(runnable.waker());

        drop(local_active);

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
        if let Some(runnable) = THREAD_LOCAL_STATE
            .get_or_default()
            .local_queue
            .borrow_mut()
            .pop_back()
        {
            // Run the task.
            runnable.run();
            true
        } else {
            false
        }
    }

    /// Runs the executor until the given future completes.
    pub async fn run<T>(&self, future: impl Future<Output = T>) -> T {
        self.state().run(future).await
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let state = self.state_as_arc();

        // TODO: If possible, push into the current local queue and notify the ticker.
        move |runnable| {
            state.queue.push(runnable).unwrap();
            state.notify();
        }
    }

    /// Returns a function that schedules a runnable task when it gets woken up.
    fn schedule_local(&self) -> impl Fn(Runnable) + 'static {
        let state = self.state_as_arc();
        let local_state: &'static ThreadLocalState = THREAD_LOCAL_STATE.get_or_default();
        // TODO: If possible, push into the current local queue and notify the ticker.
        move |runnable| {
            local_state.local_queue.borrow_mut().push_back(runnable);
            state.notify_specific_thread(local_state.thread_id);
        }
    }

    /// Returns a pointer to the inner state.
    #[inline]
    fn state_ptr(&self) -> *const State {
        #[cold]
        fn alloc_state(atomic_ptr: &AtomicPtr<State>) -> *mut State {
            let state = Arc::new(State::new());
            // TODO: Switch this to use cast_mut once the MSRV can be bumped past 1.65
            let ptr = Arc::into_raw(state) as *mut State;
            if let Err(actual) = atomic_ptr.compare_exchange(
                std::ptr::null_mut(),
                ptr,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                #[expect(
                    unsafe_code,
                    reason = "unsized coercion is an unstable feature for non-std types"
                )]
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
        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // SAFETY: So long as an Executor lives, it's state pointer will always be valid
        // when accessed through state_ptr.
        unsafe {
            &*self.state_ptr()
        }
    }

    // Clones the inner state Arc
    #[inline]
    fn state_as_arc(&self) -> Arc<State> {
        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // SAFETY: So long as an Executor lives, it's state pointer will always be a valid
        // Arc when accessed through state_ptr.
        let arc = unsafe { Arc::from_raw(self.state_ptr()) };
        let clone = arc.clone();
        std::mem::forget(arc);
        clone
    }
}

impl Drop for Executor<'_> {
    fn drop(&mut self) {
        let ptr = *self.state.get_mut();
        if ptr.is_null() {
            return;
        }

        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
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

impl<'a> Default for Executor<'a> {
    fn default() -> Executor<'a> {
        Executor::new()
    }
}

/// The state of a executor.
struct State {
    /// The global queue.
    queue: ConcurrentQueue<Runnable>,

    /// Local queues created by runners.
    local_queues: RwLock<Vec<Arc<ConcurrentQueue<Runnable>>>>,

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
            local_queues: RwLock::new(Vec::new()),
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
        self.active.lock().unwrap_or_else(|e| e.into_inner())
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
    fn notify_specific_thread(&self, thread_id: ThreadId) {
        let waker = self
            .sleepers
            .lock()
            .unwrap()
            .notify_specific_thread(thread_id);
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

    /// The local queue.
    local: Arc<ConcurrentQueue<Runnable>>,

    /// Bumped every time a runnable task is found.
    ticks: usize,

    // The thread local state of the executor for the current thread.
    local_state: &'static ThreadLocalState,
}

impl Runner<'_> {
    /// Creates a runner and registers it in the executor state.
    fn new(state: &State) -> Runner<'_> {
        let runner = Runner {
            state,
            ticker: Ticker::new(state),
            local: Arc::new(ConcurrentQueue::bounded(512)),
            ticks: 0,
            local_state: THREAD_LOCAL_STATE.get_or_default(),
        };
        state
            .local_queues
            .write()
            .unwrap()
            .push(runner.local.clone());
        runner
    }

    /// Waits for the next runnable task to run.
    async fn runnable(&mut self, rng: &mut fastrand::Rng) -> Runnable {
        let runnable = self
            .ticker
            .runnable_with(|| {
                if let Some(r) = self.local_state.local_queue.borrow_mut().pop_back() {
                    return Some(r);
                }

                // Try the local queue.
                if let Ok(r) = self.local.pop() {
                    return Some(r);
                }

                // Try stealing from the global queue.
                if let Ok(r) = self.state.queue.pop() {
                    steal(&self.state.queue, &self.local);
                    return Some(r);
                }

                // Try stealing from other runners.
                let local_queues = self.state.local_queues.read().unwrap();

                // Pick a random starting point in the iterator list and rotate the list.
                let n = local_queues.len();
                let start = rng.usize(..n);
                let iter = local_queues
                    .iter()
                    .chain(local_queues.iter())
                    .skip(start)
                    .take(n);

                // Remove this runner's local queue.
                let iter = iter.filter(|local| !Arc::ptr_eq(local, &self.local));

                // Try stealing from each local queue in the list.
                for local in iter {
                    steal(local, &self.local);
                    if let Ok(r) = self.local.pop() {
                        return Some(r);
                    }
                }

                if let Ok(r) = self.local_state.thread_locked_queue.pop() {
                    // Do not steal from this queue. If other threads steal
                    // from this current thread, the task will be moved.
                    return Some(r);
                }

                None
            })
            .await;

        // Bump the tick counter.
        self.ticks = self.ticks.wrapping_add(1);

        if self.ticks % 64 == 0 {
            // Steal tasks from the global queue to ensure fair task scheduling.
            steal(&self.state.queue, &self.local);
        }

        runnable
    }
}

impl Drop for Runner<'_> {
    fn drop(&mut self) {
        // Remove the local queue.
        self.state
            .local_queues
            .write()
            .unwrap()
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

    #[expect(
        unsafe_code,
        reason = "unsized coercion is an unstable feature for non-std types"
    )]
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
    struct LocalRunners<'a>(&'a RwLock<Vec<Arc<ConcurrentQueue<Runnable>>>>);

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
        .field("local_runners", &LocalRunners(&state.local_queues))
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

    fn _ensure_send_and_sync() {
        fn is_send<T: Send>(_: T) {}
        fn is_sync<T: Sync>(_: T) {}
        fn is_static<T: 'static>(_: T) {}

        is_send::<Executor<'_>>(Executor::new());
        is_sync::<Executor<'_>>(Executor::new());

        let ex = Executor::new();
        is_send(ex.tick());
        is_sync(ex.tick());
        is_send(ex.schedule());
        is_sync(ex.schedule());
        is_static(ex.schedule());
        is_send(ex.current_thread_spawner());
        is_sync(ex.current_thread_spawner());

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
