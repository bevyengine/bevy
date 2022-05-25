use std::{
    future::Future,
    mem,
    sync::Arc,
    thread::{self, JoinHandle},
};

pub use crate::task_pool_builder::*;
use futures_lite::{future, FutureExt};

use crate::{Task, TaskGroup};

#[derive(Debug, Default)]
struct GroupInfo {
    executor: async_executor::Executor<'static>,
    threads: usize,
}

#[derive(Debug, Default)]
struct Groups {
    compute: GroupInfo,
    async_compute: GroupInfo,
    io: GroupInfo,
}

impl Groups {
    fn get(&self, group: TaskGroup) -> &GroupInfo {
        match group {
            TaskGroup::Compute => &self.compute,
            TaskGroup::AsyncCompute => &self.async_compute,
            TaskGroup::IO => &self.io,
        }
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
    /// The groups for the pool
    ///
    /// This has to be separate from TaskPoolInner because we have to create an Arc to
    /// pass into the worker threads, and we must create the worker threads before we can create
    /// the Vec<JoinHandle<()>> contained within TaskPoolInner
    groups: Arc<Groups>,

    /// Inner state of the pool
    _inner: Arc<TaskPoolInner>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = async_executor::LocalExecutor::new();
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
        groups.io.threads = builder
            .io
            .get_number_of_threads(remaining_threads, total_threads);

        tracing::trace!("IO Threads: {}", groups.io.threads);
        remaining_threads = remaining_threads.saturating_sub(groups.io.threads);

        // Determine the number of async compute threads we will use
        groups.async_compute.threads = builder
            .async_compute
            .get_number_of_threads(remaining_threads, total_threads);

        tracing::trace!("Async Compute Threads: {}", groups.async_compute.threads);
        remaining_threads = remaining_threads.saturating_sub(groups.async_compute.threads);

        // Determine the number of compute threads we will use
        // This is intentionally last so that an end user can specify 1.0 as the percent
        groups.compute.threads = builder
            .compute
            .get_number_of_threads(remaining_threads, total_threads);
        tracing::trace!("Compute Threads: {}", groups.compute.threads);

        let groups = Arc::new(groups);
        let mut threads = Vec::with_capacity(total_threads);
        threads.extend((0..groups.compute.threads).map(|i| {
            let groups = Arc::clone(&groups);
            let shutdown_rx = shutdown_rx.clone();
            make_thread_builder(&builder, "Compute", i)
                .spawn(move || {
                    let compute = &groups.compute.executor;
                    let future = async {
                        loop {
                            compute.tick().await;
                        }
                    };
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(shutdown_rx.recv().or(future)).unwrap_err();
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..groups.io.threads).map(|i| {
            let groups = Arc::clone(&groups);
            let shutdown_rx = shutdown_rx.clone();
            make_thread_builder(&builder, "IO", i)
                .spawn(move || {
                    let compute = &groups.compute.executor;
                    let io = &groups.io.executor;
                    let future = async {
                        loop {
                            io.tick().or(compute.tick()).await;
                        }
                    };
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(shutdown_rx.recv().or(future)).unwrap_err();
                })
                .expect("Failed to spawn thread.")
        }));
        threads.extend((0..groups.async_compute.threads).map(|i| {
            let groups = Arc::clone(&groups);
            let shutdown_rx = shutdown_rx.clone();
            make_thread_builder(&builder, "Async Compute", i)
                .spawn(move || {
                    let compute = &groups.compute.executor;
                    let async_compute = &groups.async_compute.executor;
                    let io = &groups.io.executor;
                    let future = async {
                        loop {
                            async_compute.tick().or(compute.tick()).or(io.tick()).await;
                        }
                    };
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(shutdown_rx.recv().or(future)).unwrap_err();
                })
                .expect("Failed to spawn thread.")
        }));

        Self {
            groups,
            _inner: Arc::new(TaskPoolInner {
                threads,
                shutdown_tx,
            }),
        }
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.thread_count_for(TaskGroup::Compute)
            + self.thread_count_for(TaskGroup::AsyncCompute)
            + self.thread_count_for(TaskGroup::IO)
    }

    /// Return the number of threads owned by a given group in the task pool
    pub fn thread_count_for(&self, group: TaskGroup) -> usize {
        self.groups.get(group).threads
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
        let executor: &async_executor::Executor = &self.groups.get(group).executor;
        let executor: &'scope async_executor::Executor = unsafe { mem::transmute(executor) };
        TaskPool::LOCAL_EXECUTOR.with(|local_executor| {
            let local_executor: &'scope async_executor::LocalExecutor =
                unsafe { mem::transmute(local_executor) };
            let mut scope = Scope {
                executor,
                local_executor,
                spawned: Vec::new(),
            };

            f(&mut scope);

            match scope.spawned.len() {
                0 => Vec::new(),
                1 => vec![future::block_on(&mut scope.spawned[0])],
                _ => future::block_on(async move {
                    let get_results = async move {
                        let mut results = Vec::with_capacity(scope.spawned.len());
                        for task in scope.spawned {
                            results.push(task.await);
                        }
                        results
                    };

                    let tick_forever = async move {
                        loop {
                            local_executor.tick().or(executor.tick()).await;
                        }
                    };

                    get_results.or(tick_forever).await
                }),
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
        Task::new(self.groups.get(group).executor.spawn(future))
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

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, T> {
    executor: &'scope async_executor::Executor<'scope>,
    local_executor: &'scope async_executor::LocalExecutor<'scope>,
    spawned: Vec<async_executor::Task<T>>,
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
        let task = self.executor.spawn(f);
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
