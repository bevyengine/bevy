use alloc::{string::String, vec::Vec};
use bevy_platform::sync::Arc;
use core::{cell::RefCell, future::Future, marker::PhantomData, mem};

use crate::Task;

#[cfg(feature = "std")]
use std::thread_local;

#[cfg(not(feature = "std"))]
use bevy_platform::sync::{Mutex, PoisonError};

#[cfg(feature = "std")]
use crate::executor::LocalExecutor;

#[cfg(not(feature = "std"))]
use crate::executor::Executor as LocalExecutor;

#[cfg(feature = "std")]
thread_local! {
    static LOCAL_EXECUTOR: LocalExecutor<'static> = const { LocalExecutor::new() };
}

#[cfg(not(feature = "std"))]
static LOCAL_EXECUTOR: LocalExecutor<'static> = const { LocalExecutor::new() };

#[cfg(feature = "std")]
type ScopeResult<T> = alloc::rc::Rc<RefCell<Option<T>>>;

#[cfg(not(feature = "std"))]
type ScopeResult<T> = Arc<Mutex<Option<T>>>;

/// Used to create a [`TaskPool`].
#[derive(Debug, Default, Clone)]
pub struct TaskPoolBuilder {}

/// This is a dummy struct for wasm support to provide the same api as with the multithreaded
/// task pool. In the case of the multithreaded task pool this struct is used to spawn
/// tasks on a specific thread. But the wasm task pool just calls
/// `wasm_bindgen_futures::spawn_local` for spawning which just runs tasks on the main thread
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

    /// No op on the single threaded task pool
    pub fn on_thread_spawn(self, _f: impl Fn() + Send + Sync + 'static) -> Self {
        self
    }

    /// No op on the single threaded task pool
    pub fn on_thread_destroy(self, _f: impl Fn() + Send + Sync + 'static) -> Self {
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
    #[expect(unsafe_code, reason = "Required to transmute lifetimes.")]
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
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the Scope
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        // Any usages of the references passed into `Scope` must be accessed through
        // the transmuted reference for the rest of this function.

        let executor = &LocalExecutor::new();
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let executor: &'env LocalExecutor<'env> = unsafe { mem::transmute(executor) };

        let results: RefCell<Vec<ScopeResult<T>>> = RefCell::new(Vec::new());
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let results: &'env RefCell<Vec<ScopeResult<T>>> = unsafe { mem::transmute(&results) };

        let mut scope = Scope {
            executor,
            results,
            scope: PhantomData,
            env: PhantomData,
        };

        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let scope_ref: &'env mut Scope<'_, 'env, T> = unsafe { mem::transmute(&mut scope) };

        f(scope_ref);

        // Loop until all tasks are done
        while executor.try_tick() {}

        let results = scope.results.borrow();
        results
            .iter()
            .map(|result| {
                #[cfg(feature = "std")]
                return result.borrow_mut().take().unwrap();

                #[cfg(not(feature = "std"))]
                {
                    let mut lock = result.lock().unwrap_or_else(PoisonError::into_inner);
                    lock.take().unwrap()
                }
            })
            .collect()
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future, which can be polled
    /// to retrieve the output of the original future. Dropping the task will attempt to cancel it.
    /// It can also be "detached", allowing it to continue running without having to be polled by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn<T>(
        &self,
        future: impl Future<Output = T> + 'static + MaybeSend + MaybeSync,
    ) -> Task<T>
    where
        T: 'static + MaybeSend + MaybeSync,
    {
        cfg_if::cfg_if! {
            if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
                Task::wrap_future(future)
            } else if #[cfg(feature = "std")] {
                LOCAL_EXECUTOR.with(|executor| {
                    let task = executor.spawn(future);
                    // Loop until all tasks are done
                    while executor.try_tick() {}

                    Task::new(task)
                })
            } else {
                {
                    let task = LOCAL_EXECUTOR.spawn(future);
                    // Loop until all tasks are done
                    while LOCAL_EXECUTOR.try_tick() {}

                    Task::new(task)
                }
            }
        }
    }

    /// Spawns a static future on the JS event loop. This is exactly the same as [`TaskPool::spawn`].
    pub fn spawn_local<T>(
        &self,
        future: impl Future<Output = T> + 'static + MaybeSend + MaybeSync,
    ) -> Task<T>
    where
        T: 'static + MaybeSend + MaybeSync,
    {
        self.spawn(future)
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
        F: FnOnce(&LocalExecutor) -> R,
    {
        #[cfg(feature = "std")]
        return LOCAL_EXECUTOR.with(f);

        #[cfg(not(feature = "std"))]
        return f(&LOCAL_EXECUTOR);
    }
}

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope, T> {
    executor: &'scope LocalExecutor<'scope>,
    // Vector to gather results of all futures spawned during scope run
    results: &'env RefCell<Vec<ScopeResult<T>>>,

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
    pub fn spawn<Fut: Future<Output = T> + 'scope + MaybeSend>(&self, f: Fut) {
        self.spawn_on_scope(f);
    }

    /// Spawns a scoped future onto the executor. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.
    ///
    /// On the single threaded task pool, it just calls [`Scope::spawn_on_scope`].
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_external<Fut: Future<Output = T> + 'scope + MaybeSend>(&self, f: Fut) {
        self.spawn_on_scope(f);
    }

    /// Spawns a scoped future that runs on the thread the scope called from. The
    /// scope *must* outlive the provided future. The results of the future will be
    /// returned as a part of [`TaskPool::scope`]'s return value.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + MaybeSend>(&self, f: Fut) {
        let result = ScopeResult::<T>::default();
        self.results.borrow_mut().push(result.clone());
        let f = async move {
            let temp_result = f.await;

            #[cfg(feature = "std")]
            result.borrow_mut().replace(temp_result);

            #[cfg(not(feature = "std"))]
            {
                let mut lock = result.lock().unwrap_or_else(PoisonError::into_inner);
                *lock = Some(temp_result);
            }
        };
        self.executor.spawn(f).detach();
    }
}

#[cfg(feature = "std")]
mod send_sync_bounds {
    pub trait MaybeSend {}
    impl<T> MaybeSend for T {}

    pub trait MaybeSync {}
    impl<T> MaybeSync for T {}
}

#[cfg(not(feature = "std"))]
mod send_sync_bounds {
    pub trait MaybeSend: Send {}
    impl<T: Send> MaybeSend for T {}

    pub trait MaybeSync: Sync {}
    impl<T: Sync> MaybeSync for T {}
}

use send_sync_bounds::{MaybeSend, MaybeSync};
