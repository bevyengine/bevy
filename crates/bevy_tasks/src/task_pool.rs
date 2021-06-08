use crate::{Task, TaskPoolThreadPanicPolicy};
use bevy_utils::tracing::{error, warn};
use futures_lite::{future, pin};
use parking_lot::RwLock;
use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

/// Used to create a TaskPool
#[derive(Debug, Default, Clone)]
pub struct TaskPoolBuilder {
    /// If set, we'll set up the thread pool to use at most n threads. Otherwise use
    /// the logical core count of the system
    num_threads: Option<usize>,
    /// If set, we'll use the given stack size rather than the system default
    stack_size: Option<usize>,
    /// Allows customizing the name of the threads - helpful for debugging. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    thread_name: Option<String>,
    /// Allows customizing the policy for when a [`TaskPool`]'s thread(s) panic.
    panic_policy: Option<TaskPoolThreadPanicPolicy>,
}

impl TaskPoolBuilder {
    /// Creates a new TaskPoolBuilder instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the number of threads created for the pool. If unset, we default to the number
    /// of logical cores of the system
    pub fn num_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }

    /// Override the stack size of the threads created for the pool
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Override the name of the threads created for the pool. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    pub fn thread_name(mut self, thread_name: String) -> Self {
        self.thread_name = Some(thread_name);
        self
    }

    pub fn panic_policy(mut self, policy: TaskPoolThreadPanicPolicy) -> Self {
        self.panic_policy = Some(policy);
        self
    }

    /// Creates a new ThreadPoolBuilder based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal(
            self.num_threads,
            self.stack_size,
            self.thread_name.as_deref(),
            self.panic_policy,
        )
    }
}

#[derive(Debug)]
struct ThreadState {
    handle: JoinHandle<()>,
    panicking: Arc<AtomicBool>,
}

impl ThreadState {
    pub fn panicking(&self) -> bool {
        self.panicking.load(Ordering::Acquire)
    }

    pub fn thread(&self) -> &std::thread::Thread {
        self.handle.thread()
    }
}

struct PanicState {
    panicking: Arc<AtomicBool>,
}

impl Drop for PanicState {
    fn drop(&mut self) {
        if thread::panicking() {
            self.panicking.store(true, Ordering::Release);
        }
    }
}

#[derive(Debug)]
struct TaskPoolInner {
    threads: Vec<ThreadState>,
    shutdown_tx: async_channel::Sender<()>,
}

impl Drop for TaskPoolInner {
    fn drop(&mut self) {
        self.shutdown_tx.close();

        let panicking = thread::panicking();
        for state in self.threads.drain(..) {
            let res = state.handle.join();
            if !panicking {
                res.expect("Task thread panicked while executing.");
            }
        }
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool.
#[derive(Debug, Clone)]
pub struct TaskPool {
    /// The executor for the pool
    ///
    /// This has to be separate from TaskPoolInner because we have to create an Arc<Executor> to
    /// pass into the worker threads, and we must create the worker threads before we can create
    /// the Vec<Task<T>> contained within TaskPoolInner
    executor: Arc<async_executor::Executor<'static>>,

    /// Inner state of the pool
    inner: Arc<RwLock<TaskPoolInner>>,

    /// Panic policy of the inner thread pool.
    panic_policy: TaskPoolThreadPanicPolicy,

    /// Receiving channel used when an inner thread panics and
    /// another one is spawned in its place.
    shutdown_rx: async_channel::Receiver<()>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = async_executor::LocalExecutor::new();
    }

    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
    }

    fn any_panicking_threads(&self) -> bool {
        self.inner
            .read()
            .threads
            .iter()
            .any(|state| state.panicking())
    }

    pub(crate) fn handle_panicking_threads(&self) {
        if self.any_panicking_threads() {
            match self.panic_policy {
                TaskPoolThreadPanicPolicy::Propagate => {
                    for state in self.inner.write().threads.drain(..) {
                        let thread = state.thread().clone();
                        if let Err(err) = state.handle.join() {
                            error!("TaskPool's inner thread '{:?}' panicked!", thread);
                            std::panic::resume_unwind(err);
                        }
                    }
                }
                TaskPoolThreadPanicPolicy::Restart => {
                    for (idx, state) in self
                        .inner
                        .write()
                        .threads
                        .iter_mut()
                        .filter(|state| state.panicking())
                        .enumerate()
                    {
                        let thread_name = match state.thread().name() {
                            Some(name) => name.to_owned(),
                            None => format!("TaskPool ({})", idx),
                        };

                        let old_state = std::mem::replace(
                            state,
                            Self::spawn_thread_internal(
                                None,
                                thread_name,
                                self.executor.clone(),
                                self.shutdown_rx.clone(),
                            ),
                        );

                        // join the panicked thread handle
                        let panic_error = old_state.handle.join().unwrap_err();

                        warn!(
                            "TaskPool's inner thread '{:?}' panicked with error: {:?}",
                            state.thread(),
                            panic_error
                        );
                    }
                }
            }
        }
    }

    fn spawn_thread_internal(
        stack_size: Option<usize>,
        thread_name: String,
        executor: Arc<async_executor::Executor<'static>>,
        shutdown_rx: async_channel::Receiver<()>,
    ) -> ThreadState {
        let mut thread_builder = thread::Builder::new().name(thread_name);

        if let Some(stack_size) = stack_size {
            thread_builder = thread_builder.stack_size(stack_size);
        }

        let panicking = Arc::new(AtomicBool::new(false));
        let panicking_clone = panicking.clone();

        let handle = thread_builder
            .spawn(move || {
                let _panic_state = PanicState {
                    panicking: panicking_clone,
                };
                let shutdown_future = executor.run(shutdown_rx.recv());
                // Use unwrap_err because we expect a Closed error
                future::block_on(shutdown_future).unwrap_err();
            })
            .expect("Failed to spawn thread.");

        ThreadState { handle, panicking }
    }

    fn new_internal(
        num_threads: Option<usize>,
        stack_size: Option<usize>,
        thread_name: Option<&str>,
        panic_policy: Option<TaskPoolThreadPanicPolicy>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

        let executor = Arc::new(async_executor::Executor::new());

        let num_threads = num_threads.unwrap_or_else(num_cpus::get);

        let threads = (0..num_threads)
            .map(|i| {
                let thread_name = if let Some(thread_name) = thread_name {
                    format!("{} ({})", thread_name, i)
                } else {
                    format!("TaskPool ({})", i)
                };

                Self::spawn_thread_internal(
                    stack_size,
                    thread_name,
                    executor.clone(),
                    shutdown_rx.clone(),
                )
            })
            .collect();

        let panic_policy = panic_policy.unwrap_or(TaskPoolThreadPanicPolicy::Restart);

        Self {
            executor,
            inner: Arc::new(RwLock::new(TaskPoolInner {
                threads,
                shutdown_tx,
            })),
            panic_policy,
            shutdown_rx,
        }
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.inner.read().threads.len()
    }

    /// Allows spawning non-`static futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
    pub fn scope<'scope, F, T>(&self, f: F) -> Vec<T>
    where
        F: FnOnce(&mut Scope<'scope, T>) + 'scope + Send,
        T: Send + 'static,
    {
        TaskPool::LOCAL_EXECUTOR.with(|local_executor| {
            // SAFETY: This function blocks until all futures complete, so this future must return
            // before this function returns. However, rust has no way of knowing
            // this so we must convert to 'static here to appease the compiler as it is unable to
            // validate safety.
            let executor: &async_executor::Executor = &*self.executor;
            let executor: &'scope async_executor::Executor = unsafe { mem::transmute(executor) };
            let local_executor: &'scope async_executor::LocalExecutor =
                unsafe { mem::transmute(local_executor) };
            let mut scope = Scope {
                executor,
                local_executor,
                spawned: Vec::new(),
            };

            f(&mut scope);

            if scope.spawned.is_empty() {
                Vec::default()
            } else if scope.spawned.len() == 1 {
                vec![future::block_on(&mut scope.spawned[0])]
            } else {
                let fut = async move {
                    let mut results = Vec::with_capacity(scope.spawned.len());
                    for task in scope.spawned {
                        results.push(task.await);
                    }

                    results
                };

                // Pin the futures on the stack.
                pin!(fut);

                // SAFETY: This function blocks until all futures complete, so we do not read/write
                // the data from futures outside of the 'scope lifetime. However,
                // rust has no way of knowing this so we must convert to 'static
                // here to appease the compiler as it is unable to validate safety.
                let fut: Pin<&mut (dyn Future<Output = Vec<T>>)> = fut;
                let fut: Pin<&'static mut (dyn Future<Output = Vec<T>> + 'static)> =
                    unsafe { mem::transmute(fut) };

                // The thread that calls scope() will participate in driving tasks in the pool
                // forward until the tasks that are spawned by this scope() call
                // complete. (If the caller of scope() happens to be a thread in
                // this thread pool, and we only have one thread in the pool, then
                // simply calling future::block_on(spawned) would deadlock.)
                let mut spawned = local_executor.spawn(fut);
                loop {
                    if let Some(result) = future::block_on(future::poll_once(&mut spawned)) {
                        break result;
                    };

                    self.executor.try_tick();
                    local_executor.try_tick();
                }
            }
        })
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future. It can also be
    /// cancelled and "detached" allowing it to continue running without having to be polled by the
    /// end-user.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.executor.spawn(future))
    }

    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> Task<T>
    where
        T: 'static,
    {
        Task::new(TaskPool::LOCAL_EXECUTOR.with(|executor| executor.spawn(future)))
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub fn handle_task_pool_panicking_threads(task_pool: &TaskPool) {
    task_pool.handle_panicking_threads();
}

#[derive(Debug)]
pub struct Scope<'scope, T> {
    executor: &'scope async_executor::Executor<'scope>,
    local_executor: &'scope async_executor::LocalExecutor<'scope>,
    spawned: Vec<async_executor::Task<T>>,
}

impl<'scope, T: Send + 'scope> Scope<'scope, T> {
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let task = self.executor.spawn(f);
        self.spawned.push(task);
    }

    pub fn spawn_local<Fut: Future<Output = T> + 'scope>(&mut self, f: Fut) {
        let task = self.local_executor.spawn(f);
        self.spawned.push(task);
    }
}

#[cfg(test)]
#[allow(clippy::blacklisted_name)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Barrier,
    };

    #[test]
    fn test_spawn() {
        let pool = TaskPool::new();

        let foo = Box::new(42);
        let foo = &*foo;

        let count = Arc::new(AtomicI32::new(0));

        let outputs = pool.scope(|scope| {
            for _ in 0..100 {
                let count_clone = count.clone();
                scope.spawn(async move {
                    if *foo != 42 {
                        panic!("not 42!?!?")
                    } else {
                        count_clone.fetch_add(1, Ordering::Relaxed);
                        *foo
                    }
                });
            }
        });

        for output in &outputs {
            assert_eq!(*output, 42);
        }

        assert_eq!(outputs.len(), 100);
        assert_eq!(count.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn test_mixed_spawn_local_and_spawn() {
        let pool = TaskPool::new();

        let foo = Box::new(42);
        let foo = &*foo;

        let local_count = Arc::new(AtomicI32::new(0));
        let non_local_count = Arc::new(AtomicI32::new(0));

        let outputs = pool.scope(|scope| {
            for i in 0..100 {
                if i % 2 == 0 {
                    let count_clone = non_local_count.clone();
                    scope.spawn(async move {
                        if *foo != 42 {
                            panic!("not 42!?!?")
                        } else {
                            count_clone.fetch_add(1, Ordering::Relaxed);
                            *foo
                        }
                    });
                } else {
                    let count_clone = local_count.clone();
                    scope.spawn_local(async move {
                        if *foo != 42 {
                            panic!("not 42!?!?")
                        } else {
                            count_clone.fetch_add(1, Ordering::Relaxed);
                            *foo
                        }
                    });
                }
            }
        });

        for output in &outputs {
            assert_eq!(*output, 42);
        }

        assert_eq!(outputs.len(), 100);
        assert_eq!(local_count.load(Ordering::Relaxed), 50);
        assert_eq!(non_local_count.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_thread_locality() {
        let pool = Arc::new(TaskPool::new());
        let count = Arc::new(AtomicI32::new(0));
        let barrier = Arc::new(Barrier::new(101));
        let thread_check_failed = Arc::new(AtomicBool::new(false));

        for _ in 0..100 {
            let inner_barrier = barrier.clone();
            let count_clone = count.clone();
            let inner_pool = pool.clone();
            let inner_thread_check_failed = thread_check_failed.clone();
            std::thread::spawn(move || {
                inner_pool.scope(|scope| {
                    let inner_count_clone = count_clone.clone();
                    scope.spawn(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);
                    });
                    let spawner = std::thread::current().id();
                    let inner_count_clone = count_clone.clone();
                    scope.spawn_local(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);
                        if std::thread::current().id() != spawner {
                            // NOTE: This check is using an atomic rather than simply panicing the
                            // thread to avoid deadlocking the barrier on failure
                            inner_thread_check_failed.store(true, Ordering::Release);
                        }
                    });
                });
                inner_barrier.wait();
            });
        }
        barrier.wait();
        assert!(!thread_check_failed.load(Ordering::Acquire));
        assert_eq!(count.load(Ordering::Acquire), 200);
    }

    #[test]
    fn test_restart_panic_policy() {
        std::panic::set_hook(Box::new(|_| {}));

        let pool = Arc::new(
            TaskPoolBuilder::new()
                .panic_policy(TaskPoolThreadPanicPolicy::Restart)
                .num_threads(1)
                .build(),
        );

        pool.spawn(async {
            panic!("oh no!");
        })
        .detach();

        while !pool.any_panicking_threads() {}

        // even though the first thread panicked, we can still queue up a task
        // as the next call to `handle_panicking_threads` will replace the panicked
        // thread with a new once.
        let success = Arc::new(AtomicBool::new(false));
        let success_clone = success.clone();
        pool.spawn(async move {
            success_clone.store(true, Ordering::Release);
        })
        .detach();

        pool.handle_panicking_threads();
        assert!(!pool.any_panicking_threads());

        while !success.load(Ordering::Acquire) {}
    }

    #[test]
    #[should_panic = "bevy"]
    fn test_propagate_panic_policy() {
        std::panic::set_hook(Box::new(|_| {}));

        let pool = Arc::new(
            TaskPoolBuilder::new()
                .panic_policy(TaskPoolThreadPanicPolicy::Propagate)
                .num_threads(1)
                .build(),
        );

        pool.spawn(async {
            panic!("bevy");
        })
        .detach();

        while !pool.any_panicking_threads() {}
        pool.handle_panicking_threads();
    }
}
