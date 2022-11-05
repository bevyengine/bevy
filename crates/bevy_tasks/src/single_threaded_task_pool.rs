use crate::local_executor::LocalExecutor;
pub use crate::task_pool_builder::TaskPoolBuilder;
use crate::TaskGroup;
use once_cell::sync::OnceCell;
use std::{
    future::Future,
    marker::PhantomData,
    mem,
    sync::{Arc, Mutex},
};

static GLOBAL_TASK_POOL: OnceCell<TaskPool> = OnceCell::new();

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool. In this case - main thread only.
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
#[derive(Debug, Default, Clone)]
pub struct TaskPool {}

impl TaskPool {
    /// Initializes the global [`TaskPool`] instance.
    pub fn init(f: impl FnOnce() -> TaskPool) -> &'static Self {
        GLOBAL_TASK_POOL.get_or_init(f)
    }

    /// Gets the global [`ComputeTaskPool`] instance.
    ///
    /// # Panics
    /// Panics if no pool has been initialized yet.
    pub fn get() -> &'static Self {
        GLOBAL_TASK_POOL.get().expect(
            "A TaskPool has not been initialized yet. Please call \
             TaskPool::init beforehand.",
        )
    }

    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
    }

    pub(crate) fn new_internal(_: TaskPoolBuilder) -> Self {
        Self {}
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        1
    }

    /// Allows spawning non-`static futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
    pub fn scope<'env, F, T>(&self, _: TaskGroup, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'env mut Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        let executor = &LocalExecutor::new();
        let executor: &'env LocalExecutor<'env> = unsafe { mem::transmute(executor) };

        let results: Mutex<Vec<Arc<Mutex<Option<T>>>>> = Mutex::new(Vec::new());
        let results: &'env Mutex<Vec<Arc<Mutex<Option<T>>>>> = unsafe { mem::transmute(&results) };

        let mut scope = Scope {
            executor,
            results,
            scope: PhantomData,
            env: PhantomData,
        };

        let scope_ref: &'env mut Scope<'_, 'env, T> = unsafe { mem::transmute(&mut scope) };

        f(scope_ref);

        // Loop until all tasks are done
        while executor.try_tick() {}

        let results = scope.results.lock().unwrap();
        results
            .iter()
            .map(|result| result.lock().unwrap().take().unwrap())
            .collect()
    }

    /// Spawns a static future onto the JS event loop. For now it is returning FakeTask
    /// instance with no-op detach method. Returning real Task is possible here, but tricky:
    /// future is running on JS event loop, Task is running on LocalExecutor
    /// so some proxy future is needed. Moreover currently we don't have long-living
    /// LocalExecutor here (above `spawn` implementation creates temporary one)
    /// But for typical use cases it seems that current implementation should be sufficient:
    /// caller can spawn long-running future writing results to some channel / event queue
    /// and simply call detach on returned Task (like AssetServer does) - spawned future
    /// can write results to some channel / event queue.
    #[inline]
    pub fn spawn<T>(&self, _: TaskGroup, future: impl Future<Output = T> + 'static) -> FakeTask
    where
        T: 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            future.await;
        });
        FakeTask
    }

    /// Spawns a static future on the JS event loop. This is exactly the same as [`TaskSpool::spawn`].
    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> FakeTask
    where
        T: 'static,
    {
        self.spawn(TaskGroup::Compute, future)
    }
}

#[derive(Debug)]
pub struct FakeTask;

impl FakeTask {
    /// No op on the single threaded task pool
    pub fn detach(self) {}
}

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope, T> {
    executor: &'env LocalExecutor<'env>,
    // Vector to gather results of all futures spawned during scope run
    results: &'env Mutex<Vec<Arc<Mutex<Option<T>>>>>,

    // make `Scope` invariant over 'scope and 'env
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env, T: Send + 'env> Scope<'scope, 'env, T> {
    /// Spawns a scoped future onto the thread-local executor. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// On the single threaded task pool, it just calls [`Scope::spawn_local`].
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'env>(&self, f: Fut) {
        self.spawn_on_scope(f);
    }

    /// Spawns a scoped future that runs on the thread the scope called from. The
    /// scope *must* outlive the provided future. The results of the future will be
    /// returned as a part of [`TaskPool::scope`]'s return value.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'env>(&self, f: Fut) {
        let result = Arc::new(Mutex::new(None));
        self.results.lock().unwrap().push(result.clone());
        let f = async move {
            result.lock().unwrap().replace(f.await);
        };
        self.executor.spawn(f).detach();
    }
}
