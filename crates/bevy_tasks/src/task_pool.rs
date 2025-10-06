use alloc::{boxed::Box, format, string::String, vec::Vec};
use core::{future::Future, marker::PhantomData, mem, panic::AssertUnwindSafe};
use std::thread::{self, JoinHandle};

use crate::bevy_executor::Executor;
use async_task::FallibleTask;
use bevy_platform::sync::Arc;
use concurrent_queue::ConcurrentQueue;
use futures_lite::FutureExt;

use crate::{block_on, Task};

pub use crate::bevy_executor::LocalTaskSpawner;

struct CallOnDrop(Option<Arc<dyn Fn() + Send + Sync + 'static>>);

impl Drop for CallOnDrop {
    fn drop(&mut self) {
        if let Some(call) = self.0.as_ref() {
            call();
        }
    }
}

/// Used to create a [`TaskPool`]
#[derive(Default)]
#[must_use]
pub struct TaskPoolBuilder {
    /// If set, we'll set up the thread pool to use at most `num_threads` threads.
    /// Otherwise use the logical core count of the system
    num_threads: Option<usize>,
    /// If set, we'll use the given stack size rather than the system default
    stack_size: Option<usize>,
    /// Allows customizing the name of the threads - helpful for debugging. If set, threads will
    /// be named `<thread_name> (<thread_index>)`, i.e. `"MyThreadPool (2)"`.
    thread_name: Option<String>,

    on_thread_spawn: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    on_thread_destroy: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
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

    /// Sets a callback that is invoked once for every created thread as it starts.
    ///
    /// This is called on the thread itself and has access to all thread-local storage.
    /// This will block running async tasks on the thread until the callback completes.
    pub fn on_thread_spawn(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        let arc = Arc::new(f);

        #[cfg(not(target_has_atomic = "ptr"))]
        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // SAFETY:
        // - Coercion from `impl Fn` to `dyn Fn` is valid
        // - `Arc::from_raw` receives a valid pointer from a previous call to `Arc::into_raw`
        let arc = unsafe {
            Arc::from_raw(Arc::into_raw(arc) as *const (dyn Fn() + Send + Sync + 'static))
        };

        self.on_thread_spawn = Some(arc);
        self
    }

    /// Sets a callback that is invoked once for every created thread as it terminates.
    ///
    /// This is called on the thread itself and has access to all thread-local storage.
    /// This will block thread termination until the callback completes.
    pub fn on_thread_destroy(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        let arc = Arc::new(f);

        #[cfg(not(target_has_atomic = "ptr"))]
        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // SAFETY:
        // - Coercion from `impl Fn` to `dyn Fn` is valid
        // - `Arc::from_raw` receives a valid pointer from a previous call to `Arc::into_raw`
        let arc = unsafe {
            Arc::from_raw(Arc::into_raw(arc) as *const (dyn Fn() + Send + Sync + 'static))
        };

        self.on_thread_destroy = Some(arc);
        self
    }

    /// Creates a new [`TaskPool`] based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal(self, Box::leak(Box::new(Executor::new())))
    }

    pub(crate) fn build_static(self, executor: &'static Executor) -> TaskPool {
        TaskPool::new_internal(self, executor)
    }
}

/// A thread pool for executing tasks.
///
/// While futures usually need to be polled to be executed, Bevy tasks are being
/// automatically driven by the pool on threads owned by the pool. The [`Task`]
/// future only needs to be polled in order to receive the result. (For that
/// purpose, it is often stored in a component or resource, see the
/// `async_compute` example.)
///
/// If the result is not required, one may also use [`Task::detach`] and the pool
/// will still execute a task, even if it is dropped.
#[derive(Debug)]
pub struct TaskPool {
    /// The executor for the pool.
    executor: &'static Executor,

    // The inner state of the pool.
    threads: Vec<JoinHandle<()>>,
    shutdown_tx: async_channel::Sender<()>,
}

impl TaskPool {
    /// Creates a [`LocalTaskSpawner`] for this current thread of execution.
    /// Can be used to spawn new tasks to execute exclusively on this thread.
    pub fn current_thread_spawner(&self) -> LocalTaskSpawner {
        self.executor.current_thread_spawner()
    }

    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
    }

    fn new_internal(builder: TaskPoolBuilder, executor: &'static Executor) -> Self {
        let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

        let num_threads = builder
            .num_threads
            .unwrap_or_else(crate::available_parallelism);

        let threads = (0..num_threads)
            .map(|i| {
                let shutdown_rx = shutdown_rx.clone();

                let thread_name = if let Some(thread_name) = builder.thread_name.as_deref() {
                    format!("{thread_name} ({i})")
                } else {
                    format!("TaskPool ({i})")
                };
                let mut thread_builder = thread::Builder::new().name(thread_name);

                if let Some(stack_size) = builder.stack_size {
                    thread_builder = thread_builder.stack_size(stack_size);
                }

                let on_thread_spawn = builder.on_thread_spawn.clone();
                let on_thread_destroy = builder.on_thread_destroy.clone();

                thread_builder
                    .spawn(move || {
                        crate::bevy_executor::install_runtime_into_current_thread(executor);

                        if let Some(on_thread_spawn) = on_thread_spawn {
                            on_thread_spawn();
                            drop(on_thread_spawn);
                        }
                        let _destructor = CallOnDrop(on_thread_destroy);
                        loop {
                            let res =
                                std::panic::catch_unwind(|| block_on(executor.run(shutdown_rx.recv())));
                            if let Ok(value) = res {
                                // Use unwrap_err because we expect a Closed error
                                value.unwrap_err();
                                break;
                            }
                        }
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
    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        let scope_spawner = self.current_thread_spawner();
        self.scope_with_executor_inner(scope_spawner.clone(), scope_spawner, f)
    }

    /// This allows passing an external [`LocalTaskSpawner`] to spawn tasks to. When you pass an external spawner
    /// [`Scope::spawn_on_scope`] spawns is then run on the thread that [`LocalTaskSpawner`] originated from.
    /// If [`None`] is passed the scope will use a [`LocalTaskSpawner`] that is ticked on the current thread.
    ///
    /// See [`Self::scope`] for more details in general about how scopes work.
    pub fn scope_with_executor<'env, F, T>(
        &self,
        external_spawner: Option<LocalTaskSpawner>,
        f: F,
    ) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        let scope_spawner = self.executor.current_thread_spawner();
        // If an `external_executor` is passed, use that. Otherwise, get the executor stored
        // in the `THREAD_EXECUTOR` thread local.
        if let Some(external_spawner) = external_spawner {
            self.scope_with_executor_inner(external_spawner, scope_spawner, f)
        } else {
            self.scope_with_executor_inner(scope_spawner.clone(), scope_spawner, f)
        }
    }

    #[expect(unsafe_code, reason = "Required to transmute lifetimes.")]
    fn scope_with_executor_inner<'env, F, T>(
        &self,
        external_spawner: LocalTaskSpawner,
        scope_spawner: LocalTaskSpawner,
        f: F,
    ) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the Scope
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        // Any usages of the references passed into `Scope` must be accessed through
        // the transmuted reference for the rest of this function.
        let spawned: ConcurrentQueue<FallibleTask<Result<T, Box<dyn core::any::Any + Send>>>> =
            ConcurrentQueue::unbounded();
        // shadow the variable so that the owned value cannot be used for the rest of the function
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let spawned: &'env ConcurrentQueue<FallibleTask<Result<T, Box<dyn core::any::Any + Send>>>> =
            unsafe { mem::transmute(&spawned) };

        let scope = Scope {
            executor: self.executor,
            external_spawner,
            scope_spawner,
            spawned,
            scope: PhantomData,
            env: PhantomData,
        };

        // shadow the variable so that the owned value cannot be used for the rest of the function
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let scope: &'env Scope<'_, 'env, T> = unsafe { mem::transmute(&scope) };

        f(scope);

        if spawned.is_empty() {
            Vec::new()
        } else {
            block_on(self.executor.run(async move {
                let mut results = Vec::with_capacity(spawned.len());
                while let Ok(task) = spawned.pop() {
                    match task.await {
                        Some(Ok(res)) => results.push(res),
                        Some(Err(payload)) => std::panic::resume_unwind(payload),
                        None => panic!("Failed to catch panic!"),
                    }
                }
                results
            }))
        }
    }

    /// Spawns a static future onto the thread pool. The returned [`Task`] is a
    /// future that can be polled for the result. It can also be canceled and
    /// "detached", allowing the task to continue running even if dropped. In
    /// any case, the pool will execute the task even without polling by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should
    /// be used instead.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
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
        Task::new(self.executor.spawn_local(future))
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

/// A [`TaskPool`] scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope, T> {
    executor: &'static Executor,
    external_spawner: LocalTaskSpawner,
    scope_spawner: LocalTaskSpawner,
    spawned: &'scope ConcurrentQueue<FallibleTask<Result<T, Box<dyn core::any::Any + Send>>>>,
    // make `Scope` invariant over 'scope and 'env
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env, T: Send + 'scope> Scope<'scope, 'env, T> {
    #[expect(
        unsafe_code,
        reason = "Executor::spawn otherwise requires 'static Futures"
    )]
    /// Spawns a scoped future onto the thread pool. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// For futures that should run on the thread `scope` is called on [`Scope::spawn_on_scope`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        // SAFETY: The scope call that generated this `Scope` ensures that the created
        // Task does not outlive 'scope.
        let task = unsafe {
            self.executor
                .spawn_scoped(AssertUnwindSafe(f).catch_unwind())
                .fallible()
        };
        let result = self.spawned.push(task);
        debug_assert!(result.is_ok());
    }

    #[expect(
        unsafe_code,
        reason = "LocalTaskSpawner::spawn otherwise requires 'static Futures"
    )]
    /// Spawns a scoped future onto the thread the scope is run on. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the scope's thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        // SAFETY: The scope call that generated this `Scope` ensures that the created
        // Task does not outlive 'scope.
        let task = unsafe {
            self.scope_spawner
                .spawn_scoped(AssertUnwindSafe(f).catch_unwind())
                .fallible()
        };
        let result = self.spawned.push(task);
        debug_assert!(result.is_ok());
    }

    #[expect(
        unsafe_code,
        reason = "LocalTaskSpawner::spawn otherwise requires 'static Futures"
    )]
    /// Spawns a scoped future onto the thread of the external thread executor.
    /// This is typically the main thread. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the external thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_external<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        // SAFETY: The scope call that generated this `Scope` ensures that the created
        // Task does not outlive 'scope.
        let task = unsafe {
            self.external_spawner
                .spawn_scoped(AssertUnwindSafe(f).catch_unwind())
                .fallible()
        };
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbounded queue, so pushing should always succeed.
        let result = self.spawned.push(task);
        debug_assert!(result.is_ok());
    }
}

impl<'scope, 'env, T> Drop for Scope<'scope, 'env, T>
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

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};
    use std::sync::Barrier;

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
    fn test_thread_callbacks() {
        let counter = Arc::new(AtomicI32::new(0));
        static EX: Executor = Executor::new();
        let start_counter = counter.clone();
        {
            let barrier = Arc::new(Barrier::new(11));
            let last_barrier = barrier.clone();
            // Build and immediately drop to terminate
            let _pool = TaskPoolBuilder::new()
                .num_threads(10)
                .on_thread_spawn(move || {
                    start_counter.fetch_add(1, Ordering::Relaxed);
                    barrier.clone().wait();
                })
                .build_static(&EX);
            last_barrier.wait();
            assert_eq!(10, counter.load(Ordering::Relaxed));
        }
        assert_eq!(10, counter.load(Ordering::Relaxed));
        let end_counter = counter.clone();
        {
            let _pool = TaskPoolBuilder::new()
                .num_threads(20)
                .on_thread_destroy(move || {
                    end_counter.fetch_sub(1, Ordering::Relaxed);
                })
                .build_static(&EX);
            assert_eq!(10, counter.load(Ordering::Relaxed));
        }
        assert_eq!(-10, counter.load(Ordering::Relaxed));
        let start_counter = counter.clone();
        let end_counter = counter.clone();
        {
            let barrier = Arc::new(Barrier::new(6));
            let last_barrier = barrier.clone();
            let _pool = TaskPoolBuilder::new()
                .num_threads(5)
                .on_thread_spawn(move || {
                    start_counter.fetch_add(1, Ordering::Relaxed);
                    barrier.wait();
                })
                .on_thread_destroy(move || {
                    end_counter.fetch_sub(1, Ordering::Relaxed);
                })
                .build_static(&EX);
            last_barrier.wait();
            assert_eq!(-5, counter.load(Ordering::Relaxed));
        }
        assert_eq!(-10, counter.load(Ordering::Relaxed));
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
            thread::spawn(move || {
                inner_pool.scope(|scope| {
                    let inner_count_clone = count_clone.clone();
                    scope.spawn(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);
                    });
                    let spawner = thread::current().id();
                    let inner_count_clone = count_clone.clone();
                    scope.spawn_on_scope(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);
                        if thread::current().id() != spawner {
                            // NOTE: This check is using an atomic rather than simply panicking the
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
            thread::spawn(move || {
                inner_pool.scope(|scope| {
                    let spawner = thread::current().id();
                    let inner_count_clone = count_clone.clone();
                    scope.spawn(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);

                        // spawning on the scope from another thread runs the futures on the scope's thread
                        scope.spawn_on_scope(async move {
                            inner_count_clone.fetch_add(1, Ordering::Release);
                            if thread::current().id() != spawner {
                                // NOTE: This check is using an atomic rather than simply panicking the
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

    // This test will often freeze on other executors.
    #[test]
    fn test_nested_scopes() {
        let pool = TaskPool::new();
        let count = Arc::new(AtomicI32::new(0));

        pool.scope(|scope| {
            scope.spawn(async {
                pool.scope(|scope| {
                    scope.spawn(async {
                        count.fetch_add(1, Ordering::Relaxed);
                    });
                });
            });
        });

        assert_eq!(count.load(Ordering::Acquire), 1);
    }
}
