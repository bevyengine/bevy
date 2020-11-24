use std::{
    future::Future,
    mem,
    sync::{Arc, Mutex},
};

/// Used to create a TaskPool
#[derive(Debug, Default, Clone)]
pub struct TaskPoolBuilder {}

impl TaskPoolBuilder {
    /// Creates a new TaskPoolBuilder instance
    pub fn new() -> Self {
        Self::default()
    }

    pub fn num_threads(self, _num_threads: usize) -> Self {
        self
    }

    pub fn stack_size(self, _stack_size: usize) -> Self {
        self
    }

    pub fn thread_name(self, _thread_name: String) -> Self {
        self
    }

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
    pub fn scope<'scope, F, T>(&self, f: F) -> Vec<T>
    where
        F: FnOnce(&mut Scope<'scope, T>) + 'scope + Send,
        T: Send + 'static,
    {
        let executor = &async_executor::LocalExecutor::new();
        let executor: &'scope async_executor::LocalExecutor<'scope> =
            unsafe { mem::transmute(executor) };

        let mut scope = Scope {
            executor,
            results: Vec::new(),
        };

        f(&mut scope);

        // Loop until all tasks are done
        while executor.try_tick() {}

        scope
            .results
            .iter()
            .map(|result| result.lock().unwrap().take().unwrap())
            .collect()
    }

    // Spawns a static future onto the JS event loop. For now it is returning FakeTask
    // instance with no-op detach method. Returning real Task is possible here, but tricky:
    // future is running on JS event loop, Task is running on async_executor::LocalExecutor
    // so some proxy future is needed. Moreover currently we don't have long-living
    // LocalExecutor here (above `spawn` implementation creates temporary one)
    // But for typical use cases it seems that current implementation should be sufficient:
    // caller can spawn long-running future writing results to some channel / event queue
    // and simply call detach on returned Task (like AssetServer does) - spawned future
    // can write results to some channel / event queue.

    pub fn spawn<T>(&self, future: impl Future<Output = T> + 'static) -> FakeTask
    where
        T: 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            future.await;
        });
        FakeTask
    }
}

#[derive(Debug)]
pub struct FakeTask;

impl FakeTask {
    pub fn detach(self) {}
}

#[derive(Debug)]
pub struct Scope<'scope, T> {
    executor: &'scope async_executor::LocalExecutor<'scope>,
    // Vector to gather results of all futures spawned during scope run
    results: Vec<Arc<Mutex<Option<T>>>>,
}

impl<'scope, T: Send + 'scope> Scope<'scope, T> {
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let result = Arc::new(Mutex::new(None));
        self.results.push(result.clone());
        let f = async move {
            result.lock().unwrap().replace(f.await);
        };
        self.executor.spawn(f).detach();
    }
}
