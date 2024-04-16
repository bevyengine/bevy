use super::{TaskPool, TaskPoolBuilder};
use crate::{
    block_on,
    thread_executor::{ThreadExecutor, ThreadExecutorTicker},
    Task,
};
use async_executor::StaticExecutor;
use async_task::FallibleTask;
use concurrent_queue::ConcurrentQueue;
use futures_lite::FutureExt;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use std::thread::JoinHandle;

/// A [`TaskPool`] optimized for use in `static` variables.
#[derive(Debug)]
pub struct StaticTaskPool {
    executor: StaticExecutor,
    threads: Mutex<Vec<JoinHandle<()>>>,
    thread_count: AtomicUsize,
}

impl StaticTaskPool {
    /// The number of threads active in the TaskPool.
    pub fn thread_num(&self) -> usize {
        self.thread_count.load(Ordering::Relaxed)
    }

    /// Initializes the task pool with the configuration in
    ///
    /// # Panics
    /// Panics if the task pool was already initialized or provided a TaskPool
    pub fn init(&'static self, builder: TaskPoolBuilder) {
        let mut join_handles = self.threads.lock().unwrap();

        if !join_handles.is_empty() {
            drop(join_handles);
            // TODO: figure out a way to support reconfiguring/reinitializing StaticTaskPools.
            panic!("The TaskPool was already initialized.");
        }

        let num_threads = builder
            .num_threads
            .unwrap_or_else(crate::available_parallelism);

        if num_threads == 0 {
            drop(join_handles);
            panic!("Tried to initialize a TaskPool with zero threads.");
        }

        *join_handles = (0..num_threads)
            .map(|i| {
                let thread_name = if let Some(thread_name) = builder.thread_name.as_deref() {
                    format!("{thread_name} ({i})")
                } else {
                    format!("TaskPool ({i})")
                };
                let mut thread_builder = std::thread::Builder::new().name(thread_name);

                if let Some(stack_size) = builder.stack_size {
                    thread_builder = thread_builder.stack_size(stack_size);
                }

                let on_thread_spawn = builder.on_thread_spawn.clone();

                thread_builder
                    .spawn(move || {
                        TaskPool::LOCAL_EXECUTOR.with(|local_executor| {
                            if let Some(on_thread_spawn) = on_thread_spawn {
                                on_thread_spawn();
                                drop(on_thread_spawn);
                            }
                            loop {
                                let res = std::panic::catch_unwind(|| {
                                    let tick_forever = async move {
                                        loop {
                                            local_executor.tick().await;
                                        }
                                    };
                                    block_on(self.executor.run(tick_forever))
                                });
                                if res.is_ok() {
                                    break;
                                }
                            }
                        });
                    })
                    .expect("Failed to spawn thread.")
            })
            .collect();
        self.thread_count.store(num_threads, Ordering::Relaxed);
    }

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to [`thread::scope`] and `rayon::scope`.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_tasks::TaskPool;
    ///
    /// let pool = TaskPool::new();
    /// let mut x = 0;
    /// let results = pool.scope(|s| {
    ///     s.spawn(async {
    ///         // you can borrow the spawner inside a task and spawn tasks from within the task
    ///         s.spawn(async {
    ///             // borrow x and mutate it.
    ///             x = 2;
    ///             // return a value from the task
    ///             1
    ///         });
    ///         // return some other value from the first task
    ///         0
    ///     });
    /// });
    ///
    /// // The ordering of results is non-deterministic if you spawn from within tasks as above.
    /// // If you're doing this, you'll have to write your code to not depend on the ordering.
    /// assert!(results.contains(&0));
    /// assert!(results.contains(&1));
    ///
    /// // The ordering is deterministic if you only spawn directly from the closure function.
    /// let results = pool.scope(|s| {
    ///     s.spawn(async { 0 });
    ///     s.spawn(async { 1 });
    /// });
    /// assert_eq!(&results[..], &[0, 1]);
    ///
    /// // You can access x after scope runs, since it was only temporarily borrowed in the scope.
    /// assert_eq!(x, 2);
    /// ```
    ///
    /// # Lifetimes
    ///
    /// The [`Scope`] object takes two lifetimes: `'scope` and `'env`.
    ///
    /// The `'scope` lifetime represents the lifetime of the scope. That is the time during
    /// which the provided closure and tasks that are spawned into the scope are run.
    ///
    /// The `'env` lifetime represents the lifetime of whatever is borrowed by the scope.
    /// Thus this lifetime must outlive `'scope`.
    ///
    /// ```compile_fail
    /// use bevy_tasks::TaskPool;
    /// fn scope_escapes_closure() {
    ///     let pool = TaskPool::new();
    ///     let foo = Box::new(42);
    ///     pool.scope(|scope| {
    ///         std::thread::spawn(move || {
    ///             // UB. This could spawn on the scope after `.scope` returns and the internal Scope is dropped.
    ///             scope.spawn(async move {
    ///                 assert_eq!(*foo, 42);
    ///             });
    ///         });
    ///     });
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use bevy_tasks::TaskPool;
    /// fn cannot_borrow_from_closure() {
    ///     let pool = TaskPool::new();
    ///     pool.scope(|scope| {
    ///         let x = 1;
    ///         let y = &x;
    ///         scope.spawn(async move {
    ///             assert_eq!(*y, 1);
    ///         });
    ///     });
    /// }
    pub fn scope<'env, F, T>(&'static self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope StaticScope<'scope, 'env, T>),
        T: Send + 'static,
    {
        TaskPool::THREAD_EXECUTOR.with(|scope_executor| {
            self.scope_with_executor_inner(true, scope_executor, scope_executor, f)
        })
    }

    /// This allows passing an external executor to spawn tasks on. When you pass an external executor
    /// [`Scope::spawn_on_scope`] spawns is then run on the thread that [`ThreadExecutor`] is being ticked on.
    /// If [`None`] is passed the scope will use a [`ThreadExecutor`] that is ticked on the current thread.
    ///
    /// When `tick_task_pool_executor` is set to `true`, the multithreaded task stealing executor is ticked on the scope
    /// thread. Disabling this can be useful when finishing the scope is latency sensitive. Pulling tasks from
    /// global executor can run tasks unrelated to the scope and delay when the scope returns.
    ///
    /// See [`Self::scope`] for more details in general about how scopes work.
    pub fn scope_with_executor<'env, F, T>(
        &'static self,
        tick_task_pool_executor: bool,
        external_executor: Option<&ThreadExecutor>,
        f: F,
    ) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope StaticScope<'scope, 'env, T>),
        T: Send + 'static,
    {
        TaskPool::THREAD_EXECUTOR.with(|scope_executor| {
            // If a `external_executor` is passed use that. Otherwise get the executor stored
            // in the `THREAD_EXECUTOR` thread local.
            if let Some(external_executor) = external_executor {
                self.scope_with_executor_inner(
                    tick_task_pool_executor,
                    external_executor,
                    scope_executor,
                    f,
                )
            } else {
                self.scope_with_executor_inner(
                    tick_task_pool_executor,
                    scope_executor,
                    scope_executor,
                    f,
                )
            }
        })
    }

    #[allow(unsafe_code)]
    fn scope_with_executor_inner<'env, F, T>(
        &'static self,
        tick_task_pool_executor: bool,
        external_executor: &ThreadExecutor,
        scope_executor: &ThreadExecutor,
        f: F,
    ) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope StaticScope<'scope, 'env, T>),
        T: Send + 'static,
    {
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the Scope
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        // Any usages of the references passed into `Scope` must be accessed through
        // the transmuted reference for the rest of this function.
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let external_executor: &'env ThreadExecutor<'env> =
            unsafe { std::mem::transmute(external_executor) };
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let scope_executor: &'env ThreadExecutor<'env> =
            unsafe { std::mem::transmute(scope_executor) };
        let spawned: ConcurrentQueue<FallibleTask<Result<T, Box<(dyn std::any::Any + Send)>>>> =
            ConcurrentQueue::unbounded();
        // shadow the variable so that the owned value cannot be used for the rest of the function
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let spawned: &'env ConcurrentQueue<
            FallibleTask<Result<T, Box<(dyn std::any::Any + Send)>>>,
        > = unsafe { std::mem::transmute(&spawned) };

        let scope = StaticScope {
            executor: &self.executor,
            external_executor,
            scope_executor,
            spawned,
            scope: PhantomData,
            env: PhantomData,
        };

        // shadow the variable so that the owned value cannot be used for the rest of the function
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let scope: &'env StaticScope<'_, 'env, T> = unsafe { std::mem::transmute(&scope) };

        f(scope);

        if spawned.is_empty() {
            Vec::new()
        } else {
            block_on(async move {
                let get_results = async {
                    let mut results = Vec::with_capacity(spawned.len());
                    while let Ok(task) = spawned.pop() {
                        if let Some(res) = task.await {
                            match res {
                                Ok(res) => results.push(res),
                                Err(payload) => std::panic::resume_unwind(payload),
                            }
                        } else {
                            panic!("Failed to catch panic!");
                        }
                    }
                    results
                };

                let tick_task_pool_executor = tick_task_pool_executor || self.thread_num() == 0;

                // we get this from a thread local so we should always be on the scope executors thread.
                // note: it is possible `scope_executor` and `external_executor` is the same executor,
                // in that case, we should only tick one of them, otherwise, it may cause deadlock.
                let scope_ticker = scope_executor.ticker().unwrap();
                let external_ticker = if !external_executor.is_same(scope_executor) {
                    external_executor.ticker()
                } else {
                    None
                };

                match (external_ticker, tick_task_pool_executor) {
                    (Some(external_ticker), true) => {
                        Self::execute_global_external_scope(
                            &self.executor,
                            external_ticker,
                            scope_ticker,
                            get_results,
                        )
                        .await
                    }
                    (Some(external_ticker), false) => {
                        Self::execute_external_scope(external_ticker, scope_ticker, get_results)
                            .await
                    }
                    // either external_executor is none or it is same as scope_executor
                    (None, true) => {
                        Self::execute_global_scope(&self.executor, scope_ticker, get_results).await
                    }
                    (None, false) => Self::execute_scope(scope_ticker, get_results).await,
                }
            })
        }
    }

    #[inline]
    async fn execute_global_external_scope<'scope, 'ticker, T>(
        executor: &'static StaticExecutor,
        external_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        scope_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        get_results: impl Future<Output = Vec<T>>,
    ) -> Vec<T> {
        // we restart the executors if a task errors. if a scoped
        // task errors it will panic the scope on the call to get_results
        let execute_forever = async move {
            loop {
                let tick_forever = async {
                    loop {
                        external_ticker.tick().or(scope_ticker.tick()).await;
                    }
                };
                // we don't care if it errors. If a scoped task errors it will propagate
                // to get_results
                let _result = AssertUnwindSafe(executor.run(tick_forever))
                    .catch_unwind()
                    .await
                    .is_ok();
            }
        };
        execute_forever.or(get_results).await
    }

    #[inline]
    async fn execute_external_scope<'scope, 'ticker, T>(
        external_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        scope_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        get_results: impl Future<Output = Vec<T>>,
    ) -> Vec<T> {
        let execute_forever = async {
            loop {
                let tick_forever = async {
                    loop {
                        external_ticker.tick().or(scope_ticker.tick()).await;
                    }
                };
                let _result = AssertUnwindSafe(tick_forever).catch_unwind().await.is_ok();
            }
        };
        execute_forever.or(get_results).await
    }

    #[inline]
    async fn execute_global_scope<'scope, 'ticker, T>(
        executor: &'static StaticExecutor,
        scope_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        get_results: impl Future<Output = Vec<T>>,
    ) -> Vec<T> {
        let execute_forever = async {
            loop {
                let tick_forever = async {
                    loop {
                        scope_ticker.tick().await;
                    }
                };
                let _result = AssertUnwindSafe(executor.run(tick_forever))
                    .catch_unwind()
                    .await
                    .is_ok();
            }
        };
        execute_forever.or(get_results).await
    }

    #[inline]
    async fn execute_scope<'scope, 'ticker, T>(
        scope_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        get_results: impl Future<Output = Vec<T>>,
    ) -> Vec<T> {
        let execute_forever = async {
            loop {
                let tick_forever = async {
                    loop {
                        scope_ticker.tick().await;
                    }
                };
                let _result = AssertUnwindSafe(tick_forever).catch_unwind().await.is_ok();
            }
        };
        execute_forever.or(get_results).await
    }

    /// Spawns a static future onto the thread pool. The returned [`Task`] is a
    /// future that can be polled for the result. It can also be canceled and
    /// "detached", allowing the task to continue running even if dropped. In
    /// any case, the pool will execute the task even without polling by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should
    /// be used instead.
    pub fn spawn<T>(&'static self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.executor.spawn(future))
    }

    /// Spawns a static future on the thread-local async executor for the
    /// current thread. The task will run entirely on the thread the task was
    /// spawned on.
    ///
    /// The returned [`Task`] is a future that can be polled for the
    /// result. It can also be canceled and "detached", allowing the task to
    /// continue running even if dropped. In any case, the pool will execute the
    /// task even without polling by the end-user.
    ///
    /// Users should generally prefer to use [`TaskPool::spawn`] instead,
    /// unless the provided future is not `Send`.
    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> Task<T>
    where
        T: 'static,
    {
        Task::new(TaskPool::LOCAL_EXECUTOR.with(|executor| executor.spawn(future)))
    }

    /// Runs a function with the local executor. Typically used to tick
    /// the local executor on the main thread as it needs to share time with
    /// other things.
    ///
    /// ```
    /// use bevy_tasks::TaskPool;
    ///
    /// TaskPool::new().with_local_executor(|local_executor| {
    ///     local_executor.try_tick();
    /// });
    /// ```
    pub fn with_local_executor<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&async_executor::LocalExecutor) -> R,
    {
        TaskPool::LOCAL_EXECUTOR.with(f)
    }
}

/// A [`TaskPool`] scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct StaticScope<'scope, 'env: 'scope, T> {
    executor: &'static StaticExecutor,
    external_executor: &'scope ThreadExecutor<'scope>,
    scope_executor: &'scope ThreadExecutor<'scope>,
    spawned: &'scope ConcurrentQueue<FallibleTask<Result<T, Box<(dyn std::any::Any + Send)>>>>,
    // make `Scope` invariant over 'scope and 'env
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env, T: Send + 'static> StaticScope<'scope, 'env, T> {
    /// Spawns a scoped future onto the thread pool. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// For futures that should run on the thread `scope` is called on [`Scope::spawn_on_scope`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    #[allow(unsafe_code)]
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        // SAFETY: T lasts for the full 'static lifetime.
        let task = unsafe {
            self.executor
                .spawn_scoped(AssertUnwindSafe(f).catch_unwind())
                .fallible()
        };
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbounded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }

    /// Spawns a scoped future onto the thread the scope is run on. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the scope's thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        let task = self
            .scope_executor
            .spawn(AssertUnwindSafe(f).catch_unwind())
            .fallible();
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbounded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }

    /// Spawns a scoped future onto the thread of the external thread executor.
    /// This is typically the main thread. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the external thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_external<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        let task = self
            .external_executor
            .spawn(AssertUnwindSafe(f).catch_unwind())
            .fallible();
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbounded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }
}

impl<'scope, 'env, T> Drop for StaticScope<'scope, 'env, T>
where
    T: 'scope,
{
    fn drop(&mut self) {
        block_on(async {
            while let Ok(task) = self.spawned.pop() {
                task.cancel().await;
            }
        });
    }
}

macro_rules! taskpool {
    ($(#[$attr:meta])* ($static:ident, $type:ident)) => {
        static $static: $type = $type(StaticTaskPool {
            executor: StaticExecutor::new(),
            threads: Mutex::new(Vec::new()),
            thread_count: AtomicUsize::new(0),
        });

        $(#[$attr])*
        #[derive(Debug)]
        pub struct $type(StaticTaskPool);

        impl $type {
            #[doc = concat!(" Gets the global [`", stringify!($type), "`] instance.")]
            pub fn get() -> &'static StaticTaskPool {
                &$static.0
            }
        }
    };
}

taskpool! {
    /// A newtype for a task pool for CPU-intensive work that must be completed to
    /// deliver the next frame
    ///
    /// See [`TaskPool`] documentation for details on Bevy tasks.
    /// [`AsyncComputeTaskPool`] should be preferred if the work does not have to be
    /// completed before the next frame.
    (COMPUTE_TASK_POOL, ComputeTaskPool)
}

taskpool! {
    /// A newtype for a task pool for CPU-intensive work that may span across multiple frames
    ///
    /// See [`TaskPool`] documentation for details on Bevy tasks.
    /// Use [`ComputeTaskPool`] if the work must be complete before advancing to the next frame.
    (ASYNC_COMPUTE_TASK_POOL, AsyncComputeTaskPool)
}

taskpool! {
    /// A newtype for a task pool for IO-intensive work (i.e. tasks that spend very little time in a
    /// "woken" state)
    ///
    /// See [`TaskPool`] documentation for details on Bevy tasks.
    (IO_TASK_POOL, IoTaskPool)
}

/// A function used by `bevy_core` to tick the global tasks pools on the main thread.
/// This will run a maximum of 100 local tasks per executor per call to this function.
///
/// # Warning
///
/// This function *must* be called on the main thread, or the task pools will not be updated appropriately.
#[cfg(not(target_arch = "wasm32"))]
pub fn tick_global_task_pools_on_main_thread() {
    COMPUTE_TASK_POOL.0.with_local_executor(|compute_local_executor| {
        ASYNC_COMPUTE_TASK_POOL.0.with_local_executor(|async_local_executor| {
            IO_TASK_POOL.0.with_local_executor(|io_local_executor| {
                for _ in 0..100 {
                    compute_local_executor.try_tick();
                    async_local_executor.try_tick();
                    io_local_executor.try_tick();
                }
            });
        });
    });
}
