use crate::executor::{Executor, LocalExecutor};
pub use crate::task_pool_builder::*;
use futures_lite::{future, pin};
use once_cell::sync::OnceCell;
use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::{Task, TaskGroup};

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
    shutdown_tx: async_channel::Sender<()>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: LocalExecutor<'static> = LocalExecutor::new();
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
        let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

        let mut groups = Groups::default();
        let total_threads =
            crate::logical_core_count().clamp(builder.min_total_threads, builder.max_total_threads);
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
            let shutdown_rx = shutdown_rx.clone();
            let executor = executor.clone();
            make_thread_builder(&builder, "Compute", i)
                .spawn(move || {
                    let future =
                        executor.run(TaskGroup::Compute.to_priority(), i, shutdown_rx.recv());
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(future).unwrap_err();
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..groups.io).map(|i| {
            let shutdown_rx = shutdown_rx.clone();
            let executor = executor.clone();
            make_thread_builder(&builder, "IO", i)
                .spawn(move || {
                    let future = executor.run(TaskGroup::IO.to_priority(), i, shutdown_rx.recv());
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(future).unwrap_err();
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..groups.async_compute).map(|i| {
            let shutdown_rx = shutdown_rx.clone();
            let executor = executor.clone();
            make_thread_builder(&builder, "Async Compute", i)
                .spawn(move || {
                    let future =
                        executor.run(TaskGroup::AsyncCompute.to_priority(), i, shutdown_rx.recv());
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(future).unwrap_err();
                })
                .expect("Failed to spawn thread.")
        }));

        Self {
            executor,
            groups,
            threads,
            shutdown_tx,
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
    pub fn scope<'scope, F, T>(&self, group: TaskGroup, f: F) -> Vec<T>
    where
        F: FnOnce(&mut Scope<'scope, T>) + 'scope + Send,
        T: Send + 'static,
    {
        if self.thread_count_for(group) == 0 {
            tracing::error!("Attempting to use TaskPool::scope with the {:?} task group, but there are no threads for it!",
                            group);
        }
        // SAFETY: This function blocks until all futures complete, so this future must return
        // before this function returns. However, rust has no way of knowing
        // this so we must convert to 'static here to appease the compiler as it is unable to
        // validate safety.
        let executor: &Executor = &self.executor;
        let executor: &'scope Executor = unsafe { mem::transmute(executor) };
        TaskPool::LOCAL_EXECUTOR.with(|local_executor| {
            let local_executor: &'scope LocalExecutor = unsafe { mem::transmute(local_executor) };
            let mut scope = Scope {
                group,
                executor,
                local_executor,
                spawned: Vec::new(),
            };

            f(&mut scope);

            match scope.spawned.len() {
                0 => Vec::new(),
                1 => vec![future::block_on(&mut scope.spawned[0])],
                _ => {
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

                        executor.try_tick(group.to_priority());
                        local_executor.try_tick();
                    }
                }
            }
        })
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
}

impl Default for TaskPool {
    fn default() -> Self {
        TaskPoolBuilder::new().build()
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
pub struct Scope<'scope, T> {
    group: TaskGroup,
    executor: &'scope Executor<'scope>,
    local_executor: &'scope LocalExecutor<'scope>,
    spawned: Vec<crate::executor::Task<T>>,
}

impl<'scope, T: Send + 'scope> Scope<'scope, T> {
    /// Spawns a scoped future onto the thread pool into the scope's group. The scope
    /// *must* outlive the provided future. The results of the future will be returned
    /// as a part of [`TaskPool::scope`]'s return value.
    ///
    /// If the provided future is non-`Send`, [`Scope::spawn_local`] should be used
    /// instead.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let task = self.executor.spawn(self.group.to_priority(), f);
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
#[allow(clippy::blacklisted_name)]
mod tests {
    use super::*;
    use crate::TaskGroup;
    use std::sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Barrier,
    };

    #[test]
    fn test_spawn() {
        let pool = TaskPool::default();

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
        let pool = TaskPool::default();

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
        let pool = Arc::new(TaskPool::default());
        let count = Arc::new(AtomicI32::new(0));
        let barrier = Arc::new(Barrier::new(101));
        let thread_check_failed = Arc::new(AtomicBool::new(false));

        for _ in 0..100 {
            let inner_barrier = barrier.clone();
            let count_clone = count.clone();
            let inner_pool = pool.clone();
            let inner_thread_check_failed = thread_check_failed.clone();
            std::thread::spawn(move || {
                inner_pool.scope(TaskGroup::Compute, |scope| {
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
