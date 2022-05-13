use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::Arc,
    thread::{self, JoinHandle},
};

use futures_lite::{future, pin, FutureExt};

use crate::Task;

/// Used to create a [`TaskPool`]
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct TaskPoolBuilder {
    /// If set, we'll set up the thread pool to use at most n threads. Otherwise use
    /// the logical core count of the system
    compute_threads: Option<usize>,
    async_compute_threads: Option<usize>,
    io_threads: Option<usize>,
    /// If set, we'll use the given stack size rather than the system default
    stack_size: Option<usize>,
    /// Allows customizing the name of the threads - helpful for debugging. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    thread_name: Option<String>,
}

impl TaskPoolBuilder {
    /// Creates a new [`TaskPoolBuilder`] instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the number of compute-priority threads created for the pool. If unset, this default to the number
    /// of logical cores of the system
    pub fn compute_threads(mut self, num_threads: usize) -> Self {
        self.compute_threads = Some(num_threads);
        self
    }

    /// Override the number of async-compute priority threads created for the pool. If unset, this defaults to 0.
    pub fn async_compute_threads(mut self, num_threads: usize) -> Self {
        self.async_compute_threads = Some(num_threads);
        self
    }

    /// Override the number of IO-priority threads created for the pool. If unset, this defaults to 0.
    pub fn io_threads(mut self, num_threads: usize) -> Self {
        self.io_threads = Some(num_threads);
        self
    }

    /// Override the stack size of the threads created for the pool
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Override the name of the threads created for the pool. If set, threads will
    /// be named `<thread_name> (<thread_index>)`, i.e. `MyThreadPool (2)`
    pub fn thread_name(mut self, thread_name: String) -> Self {
        self.thread_name = Some(thread_name);
        self
    }

    /// Creates a new [`TaskPool`] based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal(self)
    }
}

#[derive(Debug)]
struct TaskPoolInner {
    threads: Vec<JoinHandle<()>>,
    shutdown_tx: async_channel::Sender<()>,
}

impl Drop for TaskPoolInner {
    fn drop(&mut self) {
        self.shutdown_tx.close();

        let panicking = thread::panicking();
        for join_handle in self.threads.drain(..) {
            let res = join_handle.join();
            if !panicking {
                res.expect("Task thread panicked while executing.");
            }
        }
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool.
///
/// # Scheduling Semantics
/// Each thread in the pool is assigned to one of three priority groups: Compute, IO, and Async
/// Compute. Compute is higher priority than IO, which are both higher priority than async compute.
/// Every task is assigned to a group upon being spawned. A lower priority thread will always prioritize
/// its specific tasks (i.e. IO tasks on a IO thread), but will run higher priority tasks if it would
/// otherwise be sitting idle.
///
/// For example, under heavy compute workloads, compute tasks will be scheduled to run on the IO and
/// async compute thread groups, but any IO task will take precedence over any compute task on the IO
/// threads. Likewise, async compute tasks will never be scheduled on a compute or IO thread.
///
/// By default, all threads in the pool are dedicated to compute group. Thread counts can be altered
/// via [`TaskPoolBuilder`] when constructing the pool.
#[derive(Debug, Clone)]
pub struct TaskPool {
    /// The executor for the pool
    ///
    /// This has to be separate from TaskPoolInner because we have to create an Arc<Executor> to
    /// pass into the worker threads, and we must create the worker threads before we can create
    /// the Vec<Task<T>> contained within TaskPoolInner
    compute_executor: Arc<async_executor::Executor<'static>>,
    async_compute_executor: Arc<async_executor::Executor<'static>>,
    io_executor: Arc<async_executor::Executor<'static>>,

    /// Inner state of the pool
    inner: Arc<TaskPoolInner>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = async_executor::LocalExecutor::new();
    }

    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
    }

    fn new_internal(builder: TaskPoolBuilder) -> Self {
        let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

        let compute_executor = Arc::new(async_executor::Executor::new());
        let async_compute_executor = Arc::new(async_executor::Executor::new());
        let io_executor = Arc::new(async_executor::Executor::new());

        let compute_threads = builder.compute_threads.unwrap_or_else(num_cpus::get);
        let io_threads = builder.io_threads.unwrap_or(0);
        let async_compute_threads = builder.async_compute_threads.unwrap_or(0);

        let mut threads = Vec::with_capacity(compute_threads + io_threads + async_compute_threads);
        threads.extend((0..compute_threads).map(|i| {
            let thread_builder = make_thread_builder(
                builder.thread_name.as_deref(),
                "Compute",
                i,
                builder.stack_size,
            );
            let compute = Arc::clone(&compute_executor);
            let shutdown_rx = shutdown_rx.clone();

            thread_builder
                .spawn(move || {
                    let future = run_forever(compute).or(async {
                        // Use unwrap_err because we expect a Closed error
                        shutdown_rx.recv().await.unwrap_err();
                    });
                    future::block_on(future);
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..io_threads).map(|i| {
            let thread_builder =
                make_thread_builder(builder.thread_name.as_deref(), "IO", i, builder.stack_size);
            let compute = Arc::clone(&compute_executor);
            let io = Arc::clone(&io_executor);
            let shutdown_rx = shutdown_rx.clone();

            thread_builder
                .spawn(move || {
                    let future = run_forever(io).or(run_forever(compute)).or(async {
                        // Use unwrap_err because we expect a Closed error
                        shutdown_rx.recv().await.unwrap_err();
                    });
                    future::block_on(future);
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..async_compute_threads).map(|i| {
            let thread_builder = make_thread_builder(
                builder.thread_name.as_deref(),
                "Aync Compute",
                i,
                builder.stack_size,
            );
            let compute = Arc::clone(&compute_executor);
            let async_compute = Arc::clone(&compute_executor);
            let io = Arc::clone(&io_executor);
            let shutdown_rx = shutdown_rx.clone();

            thread_builder
                .spawn(move || {
                    let future = run_forever(async_compute)
                        .or(run_forever(compute))
                        .or(run_forever(io))
                        .or(async {
                            // Use unwrap_err because we expect a Closed error
                            shutdown_rx.recv().await.unwrap_err();
                        });
                    future::block_on(future);
                })
                .expect("Failed to spawn thread.")
        }));

        Self {
            compute_executor,
            async_compute_executor,
            io_executor,
            inner: Arc::new(TaskPoolInner {
                threads,
                shutdown_tx,
            }),
        }
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.inner.threads.len()
    }

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
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
            let compute_executor: &async_executor::Executor = &*self.compute_executor;
            let compute_executor: &'scope async_executor::Executor =
                unsafe { mem::transmute(compute_executor) };
            let async_compute_executor: &async_executor::Executor = &*self.async_compute_executor;
            let async_compute_executor: &'scope async_executor::Executor =
                unsafe { mem::transmute(async_compute_executor) };
            let io_executor: &async_executor::Executor = &*self.io_executor;
            let io_executor: &'scope async_executor::Executor =
                unsafe { mem::transmute(io_executor) };
            let local_executor: &'scope async_executor::LocalExecutor =
                unsafe { mem::transmute(local_executor) };
            let mut scope = Scope {
                compute_executor,
                async_compute_executor,
                io_executor,
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

                    self.compute_executor.try_tick();
                    self.async_compute_executor.try_tick();
                    self.io_executor.try_tick();
                    local_executor.try_tick();
                }
            }
        })
    }

    /// Spawns a static future onto the thread pool with "compute" priority. The returned Task is a future.
    /// It can also be cancelled and "detached" allowing it to continue running without having to be polled
    /// by the end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.compute_executor.spawn(future))
    }

    /// Spawns a static future onto the thread pool with "async compute" priority. The returned Task is a future.
    /// It can also be cancelled and "detached" allowing it to continue running without having to be polled
    /// by the end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn_async_compute<T>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.async_compute_executor.spawn(future))
    }

    /// Spawns a static future onto the thread pool with "IO" priority. The returned Task is a future.
    /// It can also be cancelled and "detached" allowing it to continue running without having to be polled
    /// by the end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn_io<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.io_executor.spawn(future))
    }

    /// Spawns a static future on the thread-local async executor for the current thread. The task
    /// will run entirely on the thread the task was spawned on.  The returned Task is a future.
    /// It can also be cancelled and "detached" allowing it to continue running without having
    /// to be polled by the end-user. Users should generally prefer to use [`TaskPool::spawn`]
    /// instead, unless the provided future is not `Send`.
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

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, T> {
    compute_executor: &'scope async_executor::Executor<'scope>,
    async_compute_executor: &'scope async_executor::Executor<'scope>,
    io_executor: &'scope async_executor::Executor<'scope>,
    local_executor: &'scope async_executor::LocalExecutor<'scope>,
    spawned: Vec<async_executor::Task<T>>,
}

impl<'scope, T: Send + 'scope> Scope<'scope, T> {
    /// Spawns a scoped future onto the thread pool with "compute" priority. The scope
    /// *must* outlive the provided future. The results of the future will be returned
    /// as a part of [`TaskPool::scope`]'s return value.
    ///
    /// If the provided future is non-`Send`, [`Scope::spawn_local`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let task = self.compute_executor.spawn(f);
        self.spawned.push(task);
    }

    /// Spawns a scoped future onto the thread pool with "async compute" priority. The scope
    /// *must* outlive the provided future. The results of the future will be returned as a
    /// part of [`TaskPool::scope`]'s return value.
    ///
    /// If the provided future is non-`Send`, [`Scope::spawn_local`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_async_compute<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let task = self.async_compute_executor.spawn(f);
        self.spawned.push(task);
    }

    /// Spawns a scoped future onto the thread pool with "IO" priority. The scope
    /// *must* outlive the provided future. The results of the future will be returned as a
    /// part of [`TaskPool::scope`]'s return value.
    ///
    /// If the provided future is non-`Send`, [`Scope::spawn_local`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_io<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let task = self.io_executor.spawn(f);
        self.spawned.push(task);
    }

    /// Spawns a scoped future onto the thread-local executor. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future is not `Send`.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_local<Fut: Future<Output = T> + 'scope>(&mut self, f: Fut) {
        let task = self.local_executor.spawn(f);
        self.spawned.push(task);
    }
}

fn make_thread_builder(
    thread_name: Option<&str>,
    prefix: &'static str,
    idx: usize,
    stack_size: Option<usize>,
) -> thread::Builder {
    // miri does not support setting thread names
    // TODO: change back when https://github.com/rust-lang/miri/issues/1717 is fixed
    #[cfg(not(miri))]
    let mut thread_builder = {
        let thread_name = if let Some(ref thread_name) = thread_name {
            format!("{} ({}, {})", thread_name, prefix, idx)
        } else {
            format!("TaskPool ({}, {})", prefix, idx)
        };
        thread::Builder::new().name(thread_name)
    };

    #[cfg(miri)]
    let mut thread_builder = {
        let _ = idx;
        let _ = thread_name;
        thread::Builder::new()
    };

    if let Some(stack_size) = stack_size {
        thread_builder = thread_builder.stack_size(stack_size);
    }

    thread_builder
}

async fn run_forever(executor: Arc<async_executor::Executor<'static>>) {
    loop {
        while executor.try_tick() {}
        future::yield_now().await;
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
}
