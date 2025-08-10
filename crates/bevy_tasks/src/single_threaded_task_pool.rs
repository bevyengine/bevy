use alloc::{string::String, vec::Vec};
use bevy_platform::sync::Arc;
use core::{cell::{RefCell, Cell}, future::Future, marker::PhantomData, mem};

use crate::executor::LocalExecutor;
use crate::{block_on, Task};

crate::cfg::std! {
    if {
        use std::thread_local;

        use crate::executor::LocalExecutor as Executor;

        thread_local! {
            static LOCAL_EXECUTOR: Executor<'static> = const { Executor::new() };
        }
    } else {

        // Because we do not have thread-locals without std, we cannot use LocalExecutor here.
        use crate::executor::Executor;

        static LOCAL_EXECUTOR: Executor<'static> = const { Executor::new() };
    }
}

/// Used to create a [`TaskPool`].
#[derive(Debug, Default, Clone)]
pub struct TaskPoolBuilder {}

/// This is a dummy struct for wasm support to provide the same api as with the multithreaded
/// task pool. In the case of the multithreaded task pool this struct is used to spawn
/// tasks on a specific thread. But the wasm task pool just calls
/// `wasm_bindgen_futures::spawn_local` for spawning which just runs tasks on the main thread
/// and so the [`ThreadSpawner`] does nothing.
#[derive(Clone)]
pub struct ThreadSpawner<'a>(PhantomData<&'a ()>);

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
    pub fn current_thread_spawner(&self) -> ThreadSpawner<'static> {
        ThreadSpawner(PhantomData)
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
        F: for<'scope> FnOnce(&'scope mut Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        self.scope_with_executor(None, f)
    }

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
    #[expect(unsafe_code, reason = "Required to transmute lifetimes.")]
    pub fn scope_with_executor<'env, F, T>(
        &self,
        _thread_executor: Option<ThreadSpawner>,
        f: F,
    ) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope mut Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        // SAFETY: This safety comment applies to all references transmuted to 'env.
        // Any futures spawned with these references need to return before this function completes.
        // This is guaranteed because we drive all the futures spawned onto the Scope
        // to completion in this function. However, rust has no way of knowing this so we
        // transmute the lifetimes to 'env here to appease the compiler as it is unable to validate safety.
        // Any usages of the references passed into `Scope` must be accessed through
        // the transmuted reference for the rest of this function.

        let executor = LocalExecutor::new();
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let executor_ref: &'env LocalExecutor<'env> = unsafe { mem::transmute(&executor) };

        let results: RefCell<Vec<Option<T>>> = RefCell::new(Vec::new());
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let results_ref: &'env RefCell<Vec<Option<T>>> = unsafe { mem::transmute(&results) };

        let pending_tasks: Cell<usize> = Cell::new(0);
        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let pending_tasks: &'env Cell<usize> = unsafe { mem::transmute(&pending_tasks) };

        let mut scope = Scope {
            executor_ref,
            pending_tasks,
            results_ref,
            scope: PhantomData,
            env: PhantomData,
        };

        // SAFETY: As above, all futures must complete in this function so we can change the lifetime
        let scope_ref: &'env mut Scope<'_, 'env, T> = unsafe { mem::transmute(&mut scope) };

        f(scope_ref);

        // Wait until the scope is complete
        block_on(executor.run(async {
            while pending_tasks.get() != 0 {
                futures_lite::future::yield_now().await;
            }
        }));

        results
            .take()
            .into_iter()
            .map(|result| result.unwrap())
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
        crate::cfg::switch! {{
            crate::cfg::web => {
                Task::wrap_future(future)
            } 
            _ => {
                let task = EXECUTOR.spawn_local(future);
                // Loop until all tasks are done
                while Self::try_tick_local() {}

                Task::new(task)
            }
        }}
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

    crate::cfg::web! {
        if {} else {
            pub(crate) fn try_tick_local() -> bool {
                crate::cfg::bevy_executor! {
                    if {
                        crate::bevy_executor::Executor::try_tick_local()
                    } else {
                        EXECUTOR.try_tick()
                    }
                }
            }
        }
    }

    fn flush_local() {
        crate::cfg::bevy_executor! {
            if {
                crate::bevy_executor::Executor::flush_local();
            } else {
                while EXECUTOR.try_tick() {}
            }
        }
    }
}

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope, T> {
    executor_ref: &'scope LocalExecutor<'scope>,
    // The number of pending tasks spawned on the scope
    pending_tasks: &'scope Cell<usize>,
    // Vector to gather results of all futures spawned during scope run
    results_ref: &'env RefCell<Vec<Option<T>>>,

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

    #[allow(unsafe_code, reason = "Executor::spawn_local_scoped is unsafe")]
    /// Spawns a scoped future that runs on the thread the scope called from. The
    /// scope *must* outlive the provided future. The results of the future will be
    /// returned as a part of [`TaskPool::scope`]'s return value.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + MaybeSend>(&self, f: Fut) {
        // increment the number of pending tasks
        let pending_tasks = self.pending_tasks;
        pending_tasks.update(|i| i + 1);

        // add a spot to keep the result, and record the index
        let results_ref = self.results_ref;
        let mut results = results_ref.borrow_mut();
        let task_number = results.len();
        results.push(None);
        drop(results);

        // create the job closure
        let f = async move {
            let result = f.await;

            // store the result in the allocated slot
            let mut results = results_ref.borrow_mut();
            results[task_number] = Some(result);
            drop(results);

            // decrement the pending tasks count
            pending_tasks.update(|i| i - 1);
        };

        crate::cfg::bevy_executor! {
            if {
                // SAFETY: The surrounding scope will not terminate until all local tasks are done
                // ensuring that the borrowed variables do not outlive the detached task.
                unsafe {
                    self.executor.spawn_local_scoped(f).detach()
                };
            } else {
                self.executor.spawn_local(f).detach();
            }
        }
    }
}

crate::cfg::std! {
    if {
        pub trait MaybeSend {}
        impl<T> MaybeSend for T {}
    
        pub trait MaybeSync {}
        impl<T> MaybeSync for T {}
    } else {
        pub trait MaybeSend: Send {}
        impl<T: Send> MaybeSend for T {}
    
        pub trait MaybeSync: Sync {}
        impl<T: Sync> MaybeSync for T {}
    }
}

#[cfg(test)]
mod test {
    use std::{time, thread};

    use super::*;

    /// This test creates a scope with a single task that goes to sleep for a
    /// nontrivial amount of time. At one point, the scope would (incorrectly)
    /// return early under these conditions, causing a crash.
    ///
    /// The correct behavior is for the scope to block until the receiver is
    /// woken by the external thread.
    #[test]
    fn scoped_spawn() {
        let (sender, recever) = async_channel::unbounded();
        let task_pool = TaskPool {};
        let thread = thread::spawn(move || {
            let duration = time::Duration::from_millis(50);
            thread::sleep(duration);
            let _ = sender.send(0);
        });
        task_pool.scope(|scope| {
            scope.spawn(async {
                recever.recv().await
            });
        });
    }
}
