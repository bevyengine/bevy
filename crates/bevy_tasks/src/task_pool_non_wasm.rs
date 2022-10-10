//! Task pool implementation for non-wasm platforms.

use std::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    sync::Arc,
    thread::{self, JoinHandle},
};

use concurrent_queue::ConcurrentQueue;
use futures_lite::{future, pin};

use crate::Task;

/// Used to create a [`PlatformTaskPool`]
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct PlatformTaskPoolBuilder {
    /// If set, we'll set up the thread pool to use at most `num_threads` threads.
    /// Otherwise use the logical core count of the system
    num_threads: Option<usize>,
    /// If set, we'll use the given stack size rather than the system default
    stack_size: Option<usize>,
    /// Allows customizing the name of the threads - helpful for debugging. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    thread_name: Option<String>,
}

impl PlatformTaskPoolBuilder {
    /// Creates a new [`PlatformTaskPoolBuilder`] instance
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
    /// be named `<thread_name> (<thread_index>)`, i.e. `MyThreadPool (2)`
    pub fn thread_name(mut self, thread_name: String) -> Self {
        self.thread_name = Some(thread_name);
        self
    }

    /// Creates a new [`PlatformTaskPool`] based on the current options.
    pub fn build(self) -> PlatformTaskPool {
        PlatformTaskPool::new_internal(
            self.num_threads,
            self.stack_size,
            self.thread_name.as_deref(),
        )
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool.
#[derive(Debug)]
pub struct PlatformTaskPool {
    /// The executor for the pool
    ///
    /// This has to be separate from TaskPoolInner because we have to create an Arc<Executor> to
    /// pass into the worker threads, and we must create the worker threads before we can create
    /// the Vec<Task<T>> contained within TaskPoolInner
    executor: Arc<async_executor::Executor<'static>>,

    /// Inner state of the pool
    threads: Vec<JoinHandle<()>>,
    shutdown_tx: async_channel::Sender<()>,
}

impl PlatformTaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = async_executor::LocalExecutor::new();
    }

    /// Create a [`PlatformTaskPool`] with the default configuration.
    pub fn new() -> Self {
        PlatformTaskPoolBuilder::new().build()
    }

    fn new_internal(
        num_threads: Option<usize>,
        stack_size: Option<usize>,
        thread_name: Option<&str>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

        let executor = Arc::new(async_executor::Executor::new());

        let num_threads = num_threads.unwrap_or_else(crate::available_parallelism);

        let threads = (0..num_threads)
            .map(|i| {
                let ex = Arc::clone(&executor);
                let shutdown_rx = shutdown_rx.clone();

                let thread_name = if let Some(thread_name) = thread_name {
                    format!("{} ({})", thread_name, i)
                } else {
                    format!("TaskPool ({})", i)
                };
                let mut thread_builder = thread::Builder::new().name(thread_name);

                if let Some(stack_size) = stack_size {
                    thread_builder = thread_builder.stack_size(stack_size);
                }

                thread_builder
                    .spawn(move || {
                        let shutdown_future = ex.run(shutdown_rx.recv());
                        // Use unwrap_err because we expect a Closed error
                        future::block_on(shutdown_future).unwrap_err();
                    })
                    .expect("Failed to spawn thread.")
            })
            .collect();

        Self {
            executor,
            threads,
            shutdown_tx,
        }
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.threads.len()
    }

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope PlatformScope<'scope, 'env, T>),
        T: Send + 'static,
    {
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the [`PlatformScope`]
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        let executor: &async_executor::Executor = &*self.executor;
        let executor: &'env async_executor::Executor = unsafe { mem::transmute(executor) };
        let task_scope_executor = &async_executor::Executor::default();
        let task_scope_executor: &'env async_executor::Executor =
            unsafe { mem::transmute(task_scope_executor) };
        let spawned: ConcurrentQueue<async_executor::Task<T>> = ConcurrentQueue::unbounded();
        let spawned_ref: &'env ConcurrentQueue<async_executor::Task<T>> =
            unsafe { mem::transmute(&spawned) };

        let scope = PlatformScope {
            executor,
            task_scope_executor,
            spawned: spawned_ref,
            scope: PhantomData,
            env: PhantomData,
        };

        let scope_ref: &'env PlatformScope<'_, 'env, T> = unsafe { mem::transmute(&scope) };

        f(scope_ref);

        if spawned.is_empty() {
            Vec::new()
        } else {
            let get_results = async move {
                let mut results = Vec::with_capacity(spawned.len());
                while let Ok(task) = spawned.pop() {
                    results.push(task.await);
                }

                results
            };

            // Pin the futures on the stack.
            pin!(get_results);

            // SAFETY: This function blocks until all futures complete, so we do not read/write
            // the data from futures outside of the 'scope lifetime. However,
            // rust has no way of knowing this so we must convert to 'static
            // here to appease the compiler as it is unable to validate safety.
            let get_results: Pin<&mut (dyn Future<Output = Vec<T>> + 'static + Send)> = get_results;
            let get_results: Pin<&'static mut (dyn Future<Output = Vec<T>> + 'static + Send)> =
                unsafe { mem::transmute(get_results) };

            // The thread that calls scope() will participate in driving tasks in the pool
            // forward until the tasks that are spawned by this scope() call
            // complete. (If the caller of scope() happens to be a thread in
            // this thread pool, and we only have one thread in the pool, then
            // simply calling future::block_on(spawned) would deadlock.)
            let mut spawned = task_scope_executor.spawn(get_results);

            loop {
                if let Some(result) = future::block_on(future::poll_once(&mut spawned)) {
                    break result;
                };

                self.executor.try_tick();
                task_scope_executor.try_tick();
            }
        }
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future. It can also be
    /// cancelled and "detached" allowing it to continue running without having to be polled by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`PlatformTaskPool::spawn_local`] should be used instead.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.executor.spawn(future))
    }

    /// Spawns a static future on the thread-local async executor for the current thread. The task
    /// will run entirely on the thread the task was spawned on.  The returned Task is a future.
    /// It can also be cancelled and "detached" allowing it to continue running without having
    /// to be polled by the end-user. Users should generally prefer to use [`PlatformTaskPool::spawn`]
    /// instead, unless the provided future is not `Send`.
    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> Task<T>
    where
        T: 'static,
    {
        Task::new(PlatformTaskPool::LOCAL_EXECUTOR.with(|executor| executor.spawn(future)))
    }
}

impl Default for PlatformTaskPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PlatformTaskPool {
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

/// A [`PlatformTaskPool`] scope for running one or more non-`'static` futures.
///
/// For more information, see [`PlatformTaskPool::scope`].
#[derive(Debug)]
pub struct PlatformScope<'scope, 'env: 'scope, T> {
    executor: &'scope async_executor::Executor<'scope>,
    task_scope_executor: &'scope async_executor::Executor<'scope>,
    spawned: &'scope ConcurrentQueue<async_executor::Task<T>>,
    // make [`PlatformScope`] invariant over 'scope and 'env
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env, T: Send + 'scope> PlatformScope<'scope, 'env, T> {
    /// Spawns a scoped future onto the thread pool. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`PlatformTaskPool::scope`]'s return value.
    ///
    /// For futures that should run on the thread `scope` is called on [`PlatformScope::spawn_on_scope`] should be used
    /// instead.
    ///
    /// For more information, see [`PlatformTaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        let task = self.executor.spawn(f);
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbouded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }

    /// Spawns a scoped future onto the thread the scope is run on. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`PlatformTaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`PlatformScope::spawn`] instead, unless the provided future needs to run on the scope's thread.
    ///
    /// For more information, see [`PlatformTaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        let task = self.task_scope_executor.spawn(f);
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbouded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }
}
