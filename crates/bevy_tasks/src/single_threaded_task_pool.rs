use std::{
    future::Future,
    marker::PhantomData,
    mem,
    sync::{Arc, Mutex},
};

/// Used to create a TaskPool
#[derive(Debug, Default, Clone)]
pub struct TaskPoolBuilder {}

/// This is a dummy struct for wasm support to provide the same api as with the multithreaded
/// task pool. In the case of the multithreaded task pool this struct is used to spawn
/// tasks on a specific thread. But the wasm task pool just calls
/// [`wasm_bindgen_futures::spawn_local`] for spawning which just runs tasks on the main thread
/// and so the [`ThreadExecutor`] does nothing.
#[derive(Default)]
pub struct ThreadExecutor<'a>(PhantomData<&'a ()>);
impl<'a> ThreadExecutor<'a> {
    /// Creates a new `ThreadExecutor`
    pub fn new() -> Self {
        Self(PhantomData::default())
    }
}

impl TaskPoolBuilder {
    /// Creates a new TaskPoolBuilder instance
    pub fn new() -> Self {
        Self::default()
    }

    /// No op on the single threaded task pool
    pub fn num_threads(self, _num_threads: usize) -> Self {
        self
    }

    /// No op on the single threaded task pool
    pub fn stack_size(self, _stack_size: usize) -> Self {
        self
    }

    /// No op on the single threaded task pool
    pub fn thread_name(self, _thread_name: String) -> Self {
        self
    }

    /// Creates a new [`TaskPool`]
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal()
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool. In this case - main thread only.
#[derive(Debug, Default, Clone)]
pub struct TaskPool {}

impl TaskPool {
    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
    }

    #[allow(unused_variables)]
    fn new_internal() -> Self {
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
    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'env mut Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        self.scope_with_executor(false, None, f)
    }

    /// Allows spawning non-`static futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
    pub fn scope_with_executor<'env, F, T>(
        &self,
        _tick_task_pool_executor: bool,
        _thread_executor: Option<&ThreadExecutor>,
        f: F,
    ) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'env mut Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        let executor = &async_executor::LocalExecutor::new();
        let executor: &'env async_executor::LocalExecutor<'env> =
            unsafe { mem::transmute(executor) };

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
    /// future is running on JS event loop, Task is running on async_executor::LocalExecutor
    /// so some proxy future is needed. Moreover currently we don't have long-living
    /// LocalExecutor here (above `spawn` implementation creates temporary one)
    /// But for typical use cases it seems that current implementation should be sufficient:
    /// caller can spawn long-running future writing results to some channel / event queue
    /// and simply call detach on returned Task (like AssetServer does) - spawned future
    /// can write results to some channel / event queue.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + 'static) -> FakeTask
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
        self.spawn(future)
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
    executor: &'env async_executor::LocalExecutor<'env>,
    // Vector to gather results of all futures spawned during scope run
    results: &'env Mutex<Vec<Arc<Mutex<Option<T>>>>>,

    // make `Scope` invariant over 'scope and 'env
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env, T: Send + 'env> Scope<'scope, 'env, T> {
    /// Spawns a scoped future onto the executor. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// On the single threaded task pool, it just calls [`Scope::spawn_on_scope`].
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'env>(&self, f: Fut) {
        self.spawn_on_scope(f);
    }

    /// Spawns a scoped future onto the executor. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// On the single threaded task pool, it just calls [`Scope::spawn_on_scope`].
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_external<Fut: Future<Output = T> + 'env>(&self, f: Fut) {
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
