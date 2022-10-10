use std::mem;

use futures_lite::Future;

use crate::Task;

#[cfg(not(target_arch = "wasm32"))]
use super::task_pool_non_wasm::{PlatformScope, PlatformTaskPool, PlatformTaskPoolBuilder};

#[cfg(target_arch = "wasm32")]
use super::task_pool_wasm::{PlatformScope, PlatformTaskPool, PlatformTaskPoolBuilder};

/// Used to create a [`TaskPool`]
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct TaskPoolBuilder(PlatformTaskPoolBuilder);

impl TaskPoolBuilder {
    /// Creates a new [`TaskPoolBuilder`] instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the number of threads created for the pool. If unset, we default to the number
    /// of logical cores of the system
    pub fn num_threads(self, num_threads: usize) -> Self {
        TaskPoolBuilder(self.0.num_threads(num_threads))
    }

    /// Override the stack size of the threads created for the pool
    pub fn stack_size(self, stack_size: usize) -> Self {
        TaskPoolBuilder(self.0.stack_size(stack_size))
    }

    /// Override the name of the threads created for the pool. If set, threads will
    /// be named `<thread_name> (<thread_index>)`, i.e. `MyThreadPool (2)`
    pub fn thread_name(self, thread_name: String) -> Self {
        TaskPoolBuilder(self.0.thread_name(thread_name))
    }

    /// Creates a new [`TaskPool`] based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool(self.0.build())
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool.
#[derive(Debug, Default)]
pub struct TaskPool(PlatformTaskPool);

impl TaskPool {
    /// Create a [`TaskPool`] with the default configuration.
    pub fn new() -> Self {
        TaskPool(PlatformTaskPool::new())
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.0.thread_num()
    }

    /// Allows spawning non-`'static` futures on the thread pool. The function takes a callback,
    /// passing a scope object into it. The scope object provided to the callback can be used
    /// to spawn tasks. This function will await the completion of all tasks before returning.
    ///
    /// This is similar to `rayon::scope` and `crossbeam::scope`
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
    /// // results are returned in the order the tasks are spawned in.
    /// // Note: the ordering may become non-deterministic if you spawn from within tasks.
    /// // the ordering is only guaranteed when tasks are spawned directly from the main closure.
    /// assert_eq!(&results[..], &[0, 1]);
    /// // can access x after scope runs
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
    ///
    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        self.0.scope(|s| {
            let scope = Scope(s);
            let scope_ref: &'env Scope<'_, 'env, T> = unsafe { mem::transmute(&scope) };
            f(scope_ref);
        })
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future. It can also be
    /// cancelled and "detached" allowing it to continue running without having to be polled by the
    /// end-user.
    ///
    /// If the provided future is non-`Send`, [`TaskPool::spawn_local`] should be used instead.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.0.spawn(future)
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
        self.0.spawn_local(future)
    }
}

/// A `TaskPool` scope for running one or more non-`'static` futures.
///
/// For more information, see [`TaskPool::scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env, T>(pub(crate) &'scope PlatformScope<'scope, 'env, T>);

impl<'scope, 'env, T: Send + 'scope> Scope<'scope, 'env, T> {
    /// Spawns a scoped future onto the thread pool. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`PlatformTaskPool::scope`]'s return value.
    ///
    /// For futures that should run on the thread `scope` is called on [`Scope::spawn_on_scope`] should be used
    /// instead.
    ///
    /// For more information, see [`PlatformTaskPool::scope`].
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        self.0.spawn(f);
    }

    /// Spawns a scoped future onto the thread the scope is run on. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the scope's thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    pub fn spawn_on_scope<Fut: Future<Output = T> + 'scope + Send>(&self, f: Fut) {
        self.0.spawn_on_scope(f);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Barrier,
    };

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
            std::thread::spawn(move || {
                inner_pool.scope(|scope| {
                    let inner_count_clone = count_clone.clone();
                    scope.spawn(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);
                    });
                    let spawner = std::thread::current().id();
                    let inner_count_clone = count_clone.clone();
                    scope.spawn_on_scope(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);
                        if std::thread::current().id() != spawner {
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
            std::thread::spawn(move || {
                inner_pool.scope(|scope| {
                    let spawner = std::thread::current().id();
                    let inner_count_clone = count_clone.clone();
                    scope.spawn(async move {
                        inner_count_clone.fetch_add(1, Ordering::Release);

                        scope.spawn_on_scope(async move {
                            inner_count_clone.fetch_add(1, Ordering::Release);
                            if std::thread::current().id() != spawner {
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
