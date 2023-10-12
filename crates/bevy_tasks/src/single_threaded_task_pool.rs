use std::sync::Arc;
use std::{cell::RefCell, future::Future, marker::PhantomData, mem, rc::Rc};

thread_local! {
    static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = async_executor::LocalExecutor::new();
}

/// Used to create a [`TaskPool`].
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
        Self::default()
    }
}

impl TaskPoolBuilder {
    /// Creates a new `TaskPoolBuilder` instance
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
    /// Just create a new `ThreadExecutor` for wasm
    pub fn get_thread_executor() -> Arc<ThreadExecutor<'static>> {
        Arc::new(ThreadExecutor::new())
    }

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

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
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

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
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

        let results: RefCell<Vec<Rc<RefCell<Option<T>>>>> = RefCell::new(Vec::new());
        let results: &'env RefCell<Vec<Rc<RefCell<Option<T>>>>> =
            unsafe { mem::transmute(&results) };

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

        let results = scope.results.borrow();
        results
            .iter()
            .map(|result| result.borrow_mut().take().unwrap())
            .collect()
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future. It can also be
    /// cancelled and "detached" allowing it to continue running without having to be polled by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + 'static) -> FakeTask
    where
        T: 'static,
    {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            future.await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        {
            LOCAL_EXECUTOR.with(|executor| {
                let _task = executor.spawn(future);
                // Loop until all tasks are done
                while executor.try_tick() {}
            });
        }

        FakeTask
    }

    /// Spawns a static future on the JS event loop. This is exactly the same as [`TaskSpool::spawn`].
    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> FakeTask
    where
        T: 'static,
    {
        self.spawn(future)
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
        LOCAL_EXECUTOR.with(f)
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
    results: &'env RefCell<Vec<Rc<RefCell<Option<T>>>>>,

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
        let result = Rc::new(RefCell::new(None));
        self.results.borrow_mut().push(result.clone());
        let f = async move {
            let temp_result = f.await;
            result.borrow_mut().replace(temp_result);
        };
        self.executor.spawn(f).detach();
    }
}
