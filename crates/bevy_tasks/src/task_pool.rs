use std::{
    future::Future,
    marker::PhantomData,
    mem,
    sync::Arc,
    thread::{self, JoinHandle},
};

use async_task::FallibleTask;
use concurrent_queue::ConcurrentQueue;
use futures_lite::{future, pin, FutureExt};

use crate::Task;

/// Used to create a [`TaskPool`]
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct TaskPoolBuilder {
    /// If set, we'll set up the thread pool to use at most `num_threads` threads.
    /// Otherwise use the logical core count of the system
    num_threads: Option<usize>,
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

    /// Creates a new [`TaskPool`] based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal(
            self.num_threads,
            self.stack_size,
            self.thread_name.as_deref(),
        )
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool.
#[derive(Debug)]
pub struct TaskPool {
    /// The executor for the pool
    ///
    /// This has to be separate from TaskPoolInner because we have to create an `Arc<Executor>` to
    /// pass into the worker threads, and we must create the worker threads before we can create
    /// the `Vec<Task<T>>` contained within `TaskPoolInner`
    executor: Arc<async_executor::Executor<'static>>,

    /// Inner state of the pool
    threads: Vec<JoinHandle<()>>,
    shutdown_tx: async_channel::Sender<()>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = async_executor::LocalExecutor::new();
    }

    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
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
                    format!("{thread_name} ({i})")
                } else {
                    format!("TaskPool ({i})")
                };
                let mut thread_builder = thread::Builder::new().name(thread_name);

                if let Some(stack_size) = stack_size {
                    thread_builder = thread_builder.stack_size(stack_size);
                }

                thread_builder
                    .spawn(move || {
                        TaskPool::LOCAL_EXECUTOR.with(|local_executor| {
                            loop {
                                let res = std::panic::catch_unwind(|| {
                                    let tick_forever = async move {
                                        loop {
                                            local_executor.tick().await;
                                        }
                                    };
                                    future::block_on(ex.run(tick_forever.or(shutdown_rx.recv())))
                                });
                                if let Ok(value) = res {
                                    // Use unwrap_err because we expect a Closed error
                                    value.unwrap_err();
                                    break;
                                }
                            }
                        });
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
    ///     s.spawn(async { 0  });
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
    ///
    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the Scope
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        let executor: &async_executor::Executor = &self.executor;
        let executor: &'env async_executor::Executor = unsafe { mem::transmute(executor) };
        let task_scope_executor = &async_executor::Executor::default();
        let task_scope_executor: &'env async_executor::Executor =
            unsafe { mem::transmute(task_scope_executor) };
        let spawned: ConcurrentQueue<FallibleTask<T>> = ConcurrentQueue::unbounded();
        let spawned_ref: &'env ConcurrentQueue<FallibleTask<T>> =
            unsafe { mem::transmute(&spawned) };

        let scope = Scope {
            executor,
            task_scope_executor,
            spawned: spawned_ref,
            scope: PhantomData,
            env: PhantomData,
        };

        let scope_ref: &'env Scope<'_, 'env, T> = unsafe { mem::transmute(&scope) };

        f(scope_ref);

        if spawned.is_empty() {
            Vec::new()
        } else {
            let get_results = async {
                let mut results = Vec::with_capacity(spawned_ref.len());
                while let Ok(task) = spawned_ref.pop() {
                    results.push(task.await.unwrap());
                }

                results
            };

            // Pin the futures on the stack.
            pin!(get_results);

            loop {
                if let Some(result) = future::block_on(future::poll_once(&mut get_results)) {
                    break result;
                };

                std::panic::catch_unwind(|| {
                    executor.try_tick();
                    task_scope_executor.try_tick();
                })
                .ok();
            }
        }
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future. It can also be
    /// cancelled and "detached" allowing it to continue running without having to be polled by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.executor.spawn(future))
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

    /// Runs a function with the local executor. Typically used to tick
    /// the local executor on the main thread as it needs to share time with
    /// other things.
    ///
    /// ```rust
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
        Self::LOCAL_EXECUTOR.with(f)
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TaskPool {
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

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope, T> {
    executor: &'scope async_executor::Executor<'scope>,
    task_scope_executor: &'scope async_executor::Executor<'scope>,
    spawned: &'scope ConcurrentQueue<FallibleTask<T>>,
    // make `Scope` invariant over 'scope and 'env
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env, T: Send + 'scope> Scope<'scope, 'env, T> {
    /// Spawns a scoped future onto the thread pool. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// For futures that should run on the thread `scope` is called on [`Scope::spawn_on_scope`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        let task = self.executor.spawn(f).fallible();
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbouded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }

    /// Spawns a scoped future onto the thread the scope is run on. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the scope's thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        let task = self.task_scope_executor.spawn(f).fallible();
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbouded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }
}

impl<'scope, 'env, T> Drop for Scope<'scope, 'env, T>
where
    T: 'scope,
{
    fn drop(&mut self) {
        future::block_on(async {
            while let Ok(task) = self.spawned.pop() {
                task.cancel().await;
            }
        });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
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
    fn test_mixed_spawn_on_scope_and_spawn() {
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
                    scope.spawn_on_scope(async move {
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
                    scope.spawn_on_scope(async move {
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
    fn test_nested_spawn() {
        let pool = TaskPool::new();

        let foo = Box::new(42);
        let foo = &*foo;

        let count = Arc::new(AtomicI32::new(0));

        let outputs: Vec<i32> = pool.scope(|scope| {
            for _ in 0..10 {
                let count_clone = count.clone();
                scope.spawn(async move {
                    for _ in 0..10 {
                        let count_clone_clone = count_clone.clone();
                        scope.spawn(async move {
                            if *foo != 42 {
                                panic!("not 42!?!?")
                            } else {
                                count_clone_clone.fetch_add(1, Ordering::Relaxed);
                                *foo
                            }
                        });
                    }
                    *foo
                });
            }
        });

        for output in &outputs {
            assert_eq!(*output, 42);
        }

        // the inner loop runs 100 times and the outer one runs 10. 100 + 10
        assert_eq!(outputs.len(), 110);
        assert_eq!(count.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn test_nested_locality() {
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
                    let spawner = std::thread::current().id();
                    let inner_count_clone = count_clone.clone();
                    scope.spawn(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);

                        // spawning on the scope from another thread runs the futures on the scope's thread
                        scope.spawn_on_scope(async move {
                            inner_count_clone.fetch_add(1, Ordering::Release);
                            if std::thread::current().id() != spawner {
                                // NOTE: This check is using an atomic rather than simply panicing the
                                // thread to avoid deadlocking the barrier on failure
                                inner_thread_check_failed.store(true, Ordering::Release);
                            }
                        });
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
