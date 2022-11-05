pub use crate::task_pool_builder::*;
use crate::{executor::Executor, local_executor::LocalExecutor, Task, TaskGroup};
use concurrent_queue::ConcurrentQueue;
use event_listener::Event;
use futures_lite::{future, pin};
use once_cell::sync::OnceCell;
use std::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    sync::Arc,
    thread::{self, JoinHandle},
};

static GLOBAL_TASK_POOL: OnceCell<TaskPool> = OnceCell::new();

#[derive(Debug, Default)]
struct Groups {
    compute: usize,
    async_compute: usize,
    io: usize,
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
///
/// # Drop Behavior
/// Dropping the task pool will immeddiately cancel all scheduled tasks and join all threads contained
/// within.
#[derive(Debug)]
pub struct TaskPool {
    /// Inner state of the pool
    executor: Arc<Executor<'static>>,

    /// Inner state of the pool
    groups: Groups,
    threads: Vec<JoinHandle<()>>,
    shutdown: Arc<Event>,
}

impl TaskPool {
    thread_local! {
        pub(crate) static LOCAL_EXECUTOR: LocalExecutor<'static> = LocalExecutor::new();
    }

    /// Initializes the global [`TaskPool`] instance.
    pub fn init(f: impl FnOnce() -> TaskPool) -> &'static Self {
        GLOBAL_TASK_POOL.get_or_init(f)
    }

    /// Gets the global [`TaskPool`] instance.
    ///
    /// # Panics
    /// Panics if no pool has been initialized yet.
    pub fn get() -> &'static Self {
        GLOBAL_TASK_POOL.get().expect(
            "A TaskPool has not been initialized yet. Please call \
             TaskPool::init beforehand.",
        )
    }

    /// Get a [`TaskPoolBuilder`] for custom configuration.
    pub fn build() -> TaskPoolBuilder {
        TaskPoolBuilder::new()
    }

    pub(crate) fn new_internal(builder: TaskPoolBuilder) -> Self {
        let shutdown = Arc::new(Event::new());
        let mut groups = Groups::default();
        let total_threads = crate::available_parallelism()
            .clamp(builder.min_total_threads, builder.max_total_threads);
        tracing::trace!("Assigning {} cores to default task pools", total_threads);

        let mut remaining_threads = total_threads;

        // Determine the number of IO threads we will use
        groups.io = builder
            .io
            .get_number_of_threads(remaining_threads, total_threads);

        tracing::trace!("IO Threads: {}", groups.io);
        remaining_threads = remaining_threads.saturating_sub(groups.io);

        // Determine the number of async compute threads we will use
        groups.async_compute = builder
            .async_compute
            .get_number_of_threads(remaining_threads, total_threads);

        tracing::trace!("Async Compute Threads: {}", groups.async_compute);
        remaining_threads = remaining_threads.saturating_sub(groups.async_compute);

        // Determine the number of compute threads we will use
        // This is intentionally last so that an end user can specify 1.0 as the percent
        groups.compute = builder
            .compute
            .get_number_of_threads(remaining_threads, total_threads);
        tracing::trace!("Compute Threads: {}", groups.compute);

        let mut thread_counts = vec![0; TaskGroup::MAX_PRIORITY];
        thread_counts[TaskGroup::Compute.to_priority()] = groups.compute;
        thread_counts[TaskGroup::IO.to_priority()] = groups.io;
        thread_counts[TaskGroup::AsyncCompute.to_priority()] = groups.async_compute;
        let executor = Arc::new(Executor::new(&thread_counts));
        let mut threads = Vec::with_capacity(total_threads);
        threads.extend((0..groups.compute).map(|i| {
            let shutdown = Arc::clone(&shutdown);
            let executor = executor.clone();
            make_thread_builder(&builder, "Compute", i)
                .spawn(move || loop {
                    let shutdown_listener = shutdown.listen();
                    let res = std::panic::catch_unwind(|| {
                        future::block_on(executor.run(
                            TaskGroup::Compute.to_priority(),
                            i,
                            shutdown_listener,
                        ));
                    });
                    if res.is_ok() {
                        break;
                    }
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..groups.io).map(|i| {
            let shutdown = Arc::clone(&shutdown);
            let executor = executor.clone();
            make_thread_builder(&builder, "IO", i)
                .spawn(move || loop {
                    let shutdown_listener = shutdown.listen();
                    let res = std::panic::catch_unwind(|| {
                        future::block_on(executor.run(
                            TaskGroup::IO.to_priority(),
                            i,
                            shutdown_listener,
                        ));
                    });
                    if res.is_ok() {
                        break;
                    }
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..groups.async_compute).map(|i| {
            let shutdown = Arc::clone(&shutdown);
            let executor = executor.clone();
            make_thread_builder(&builder, "Async Compute", i)
                .spawn(move || loop {
                    let shutdown_listener = shutdown.listen();
                    let res = std::panic::catch_unwind(|| {
                        future::block_on(executor.run(
                            TaskGroup::AsyncCompute.to_priority(),
                            i,
                            shutdown_listener,
                        ));
                    });
                    if res.is_ok() {
                        break;
                    }
                })
                .expect("Failed to spawn thread.")
        }));

        Self {
            executor,
            groups,
            threads,
            shutdown,
        }
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.threads.len()
    }

    /// Return the number of threads that can run a given [`TaskGroup`] in the task pool
    pub fn thread_count_for(&self, group: TaskGroup) -> usize {
        let groups = &self.groups;
        match group {
            TaskGroup::Compute => self.thread_num(),
            TaskGroup::IO => groups.io + groups.async_compute,
            TaskGroup::AsyncCompute => groups.async_compute,
        }
    }

    /// Allows spawning non-`'static` futures on the thread pool in a specific task group. The
    /// function takes a callback, passing a scope object into it. The scope object provided
    /// to the callback can be used to spawn tasks. This function will await the completion of
    /// all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_tasks::{TaskPool, TaskGroup};
    ///
    /// let pool = TaskPool::init(TaskPool::default);
    /// let mut x = 0;
    /// let results = pool.scope(TaskGroup::Compute, |s| {
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
    /// let results = pool.scope(TaskGroup::Compute, |s| {
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
    /// use bevy_tasks::{TaskPool, TaskGroup};
    /// fn scope_escapes_closure() {
    ///     let pool = TaskPool::init(TaskPool::default);
    ///     let foo = Box::new(42);
    ///     pool.scope(TaskGroup::Compute, |scope| {
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
    /// use bevy_tasks::{TaskPool, TaskGroup};
    /// fn cannot_borrow_from_closure() {
    ///     let pool = TaskPool::init(TaskPool::default);
    ///     pool.scope(TaskGroup::Compute, |scope| {
    ///         let x = 1;
    ///         let y = &x;
    ///         scope.spawn(async move {
    ///             assert_eq!(*y, 1);
    ///         });
    ///     });
    /// }
    ///
    pub fn scope<'env, F, T>(&self, group: TaskGroup, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        if self.thread_count_for(group) == 0 {
            tracing::error!("Attempting to use TaskPool::scope with the {:?} task group, but there are no threads for it!",
                            group);
        }

        let mut dummy_thread_counts = vec![0; TaskGroup::MAX_PRIORITY];
        dummy_thread_counts[group.to_priority()] = 1;
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the Scope
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        let executor: &Executor = &self.executor;
        let executor: &'env Executor = unsafe { mem::transmute(executor) };
        let task_scope_executor = &Executor::new(&dummy_thread_counts);
        let task_scope_executor: &'env Executor = unsafe { mem::transmute(task_scope_executor) };
        let spawned: ConcurrentQueue<async_task::Task<T>> = ConcurrentQueue::unbounded();
        let spawned_ref: &'env ConcurrentQueue<async_task::Task<T>> =
            unsafe { mem::transmute(&spawned) };

        let scope = Scope {
            group,
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
            let mut spawned = executor.spawn(group.to_priority(), get_results);

            loop {
                if let Some(result) = future::block_on(future::poll_once(&mut spawned)) {
                    break result;
                };

                self.executor.try_tick(group.to_priority());
                task_scope_executor.try_tick(group.to_priority());
            }
        }
    }

    /// Spawns a static future onto the thread pool in a group. The returned Task is a future.
    /// It can also be cancelled and "detached" allowing it to continue running without having to be polled
    /// by the end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    #[inline]
    pub fn spawn<T>(
        &self,
        group: TaskGroup,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T>
    where
        T: Send + 'static,
    {
        if self.thread_count_for(group) == 0 {
            tracing::error!("Attempted to use TaskPool::spawn with the {:?} task group, but there are no threads for it!",
                            group);
        }
        Task::new(self.executor.spawn(group.to_priority(), future))
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

    /// Flushes all local tasks on the current thread from the `TaskPool`.
    /// This function will continue running until the local executor for the
    /// current thread is empty.
    pub fn flush_local_tasks() {
        Self::LOCAL_EXECUTOR.with(|local_executor| while local_executor.try_tick() {});
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        TaskPoolBuilder::new().build()
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.shutdown.notify_additional_relaxed(usize::MAX);

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
    group: TaskGroup,
    executor: &'scope Executor<'scope>,
    task_scope_executor: &'scope Executor<'scope>,
    spawned: &'scope ConcurrentQueue<async_task::Task<T>>,
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
        let task = self.executor.spawn(self.group.to_priority(), f);
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
        let task = self.task_scope_executor.spawn(self.group.to_priority(), f);
        // ConcurrentQueue only errors when closed or full, but we never
        // close and use an unbouded queue, so it is safe to unwrap
        self.spawned.push(task).unwrap();
    }
}

fn make_thread_builder(
    builder: &TaskPoolBuilder,
    prefix: &'static str,
    idx: usize,
) -> thread::Builder {
    let mut thread_builder = {
        let thread_name = if let Some(ref thread_name) = builder.thread_name {
            format!("{} ({}, {})", thread_name, prefix, idx)
        } else {
            format!("TaskPool ({}, {})", prefix, idx)
        };
        thread::Builder::new().name(thread_name)
    };

    if let Some(stack_size) = builder.stack_size {
        thread_builder = thread_builder.stack_size(stack_size);
    }

    thread_builder
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
mod tests {
    use super::*;
    use crate::TaskGroup;
    use std::sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Barrier,
    };

    #[test]
    fn test_spawn() {
        let pool = TaskPool::init(TaskPool::default);

        let foo = Box::new(42);
        let foo = &*foo;

        let count = Arc::new(AtomicI32::new(0));

        let outputs = pool.scope(TaskGroup::Compute, |scope| {
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
        let pool = TaskPool::init(TaskPool::default);

        let foo = Box::new(42);
        let foo = &*foo;

        let local_count = Arc::new(AtomicI32::new(0));
        let non_local_count = Arc::new(AtomicI32::new(0));

        let outputs = pool.scope(TaskGroup::Compute, |scope| {
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
        let pool = TaskPool::init(TaskPool::default);
        let count = Arc::new(AtomicI32::new(0));
        let barrier = Arc::new(Barrier::new(101));
        let thread_check_failed = Arc::new(AtomicBool::new(false));

        for _ in 0..100 {
            let inner_barrier = barrier.clone();
            let count_clone = count.clone();
            let inner_thread_check_failed = thread_check_failed.clone();
            std::thread::spawn(move || {
                pool.scope(TaskGroup::Compute, |scope| {
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
        let pool = TaskPool::init(TaskPool::default);

        let foo = Box::new(42);
        let foo = &*foo;

        let count = Arc::new(AtomicI32::new(0));

        let outputs: Vec<i32> = pool.scope(TaskGroup::Compute, |scope| {
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
        let pool = TaskPool::init(TaskPool::default);
        let count = Arc::new(AtomicI32::new(0));
        let barrier = Arc::new(Barrier::new(101));
        let thread_check_failed = Arc::new(AtomicBool::new(false));

        for _ in 0..100 {
            let inner_barrier = barrier.clone();
            let count_clone = count.clone();
            let inner_thread_check_failed = thread_check_failed.clone();
            std::thread::spawn(move || {
                pool.scope(TaskGroup::Compute, |scope| {
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
