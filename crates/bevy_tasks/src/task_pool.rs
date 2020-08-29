use parking::Unparker;
use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

/// Used to create a TaskPool
#[derive(Debug, Default, Clone)]
pub struct TaskPoolBuilder {
    /// If set, we'll set up the thread pool to use at most n threads. Otherwise use
    /// the logical core count of the system
    num_threads: Option<usize>,
    /// If set, we'll use the given stack size rather than the system default
    stack_size: Option<usize>,
    /// Allows customizing the name of the threads - helpful for debugging. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    thread_name: Option<String>,
}

impl TaskPoolBuilder {
    /// Creates a new TaskPoolBuilder instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the number of threads created for the pool. If unset, we default to the number
    /// of logical cores of the system
    pub fn num_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }

    /// Override the stack size of the threads created for the pool
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Override the name of the threads created for the pool. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    pub fn thread_name(mut self, thread_name: String) -> Self {
        self.thread_name = Some(thread_name);
        self
    }

    /// Creates a new ThreadPoolBuilder based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal(
            self.num_threads,
            self.stack_size,
            self.thread_name.as_deref(),
        )
    }
}

struct TaskPoolInner {
    threads: Vec<(JoinHandle<()>, Arc<Unparker>)>,
    shutdown_flag: Arc<AtomicBool>,
}

impl Drop for TaskPoolInner {
    fn drop(&mut self) {
        self.shutdown_flag.store(true, Ordering::Release);

        for (_, unparker) in &self.threads {
            unparker.unpark();
        }
        for (join_handle, _) in self.threads.drain(..) {
            join_handle
                .join()
                .expect("task thread panicked while executing");
        }
    }
}

/// A thread pool for executing tasks. Tasks are futures that are being automatically driven by
/// the pool on threads owned by the pool.
#[derive(Clone)]
pub struct TaskPool {
    /// The executor for the pool
    ///
    /// This has to be separate from TaskPoolInner because we have to create an Arc<Executor> to
    /// pass into the worker threads, and we must create the worker threads before we can create the
    /// Vec<Task<T>> contained within TaskPoolInner
    executor: Arc<multitask::Executor>,

    /// Inner state of the pool
    inner: Arc<TaskPoolInner>,
}

impl TaskPool {
    /// Create a `TaskPool` with the default configuration.
    pub fn new() -> Self {
        TaskPoolBuilder::new().build()
    }

    fn new_internal(
        num_threads: Option<usize>,
        stack_size: Option<usize>,
        thread_name: Option<&str>,
    ) -> Self {
        let executor = Arc::new(multitask::Executor::new());
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let num_threads = num_threads.unwrap_or_else(num_cpus::get);

        let threads = (0..num_threads)
            .map(|i| {
                let ex = Arc::clone(&executor);
                let flag = Arc::clone(&shutdown_flag);
                let (p, u) = parking::pair();
                let unparker = Arc::new(u);
                let u = Arc::clone(&unparker);
                // Run an executor thread.

                let thread_name = if let Some(thread_name) = thread_name {
                    format!("{} ({})", thread_name, i)
                } else {
                    format!("TaskPool ({})", i)
                };

                let mut thread_builder = thread::Builder::new().name(thread_name);

                if let Some(stack_size) = stack_size {
                    thread_builder = thread_builder.stack_size(stack_size);
                }

                let handle = thread_builder
                    .spawn(move || {
                        let ticker = ex.ticker(move || u.unpark());
                        loop {
                            if flag.load(Ordering::Acquire) {
                                break;
                            }

                            if !ticker.tick() {
                                p.park();
                            }
                        }
                    })
                    .expect("failed to spawn thread");

                (handle, unparker)
            })
            .collect();

        Self {
            executor,
            inner: Arc::new(TaskPoolInner {
                threads,
                shutdown_flag,
            }),
        }
    }

    /// Return the number of threads owned by the task pool
    pub fn thread_num(&self) -> usize {
        self.inner.threads.len()
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
        // SAFETY: This function blocks until all futures complete, so this future must return
        // before this function returns. However, rust has no way of knowing
        // this so we must convert to 'static here to appease the compiler as it is unable to
        // validate safety.
        let executor: &multitask::Executor = &*self.executor as &multitask::Executor;
        let executor: &'scope multitask::Executor = unsafe { mem::transmute(executor) };

        let fut = async move {
            let mut scope = Scope {
                executor,
                spawned: Vec::new(),
            };

            f(&mut scope);

            let mut results = Vec::with_capacity(scope.spawned.len());
            for task in scope.spawned {
                results.push(task.await);
            }

            results
        };

        // Move the value to ensure that it is owned
        let mut fut = fut;

        // Shadow the original binding so that it can't be directly accessed
        // ever again.
        let fut = unsafe { Pin::new_unchecked(&mut fut) };

        // SAFETY: This function blocks until all futures complete, so we do not read/write the
        // data from futures outside of the 'scope lifetime. However, rust has no way of knowing
        // this so we must convert to 'static here to appease the compiler as it is unable to
        // validate safety.
        let fut: Pin<&mut (dyn Future<Output = Vec<T>> + Send)> = fut;
        let fut: Pin<&'static mut (dyn Future<Output = Vec<T>> + Send + 'static)> =
            unsafe { mem::transmute(fut) };

        pollster::block_on(self.executor.spawn(fut))
    }

    /// Spawns a static future onto the thread pool. The returned Task is a future. It can also be
    /// cancelled and "detached" allowing it to continue running without having to be polled by the
    /// end-user.
    pub fn spawn<T>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> impl Future<Output = T> + Send
    where
        T: Send + 'static,
    {
        self.executor.spawn(future)
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Scope<'scope, T> {
    executor: &'scope multitask::Executor,
    spawned: Vec<multitask::Task<T>>,
}

impl<'scope, T: Send + 'static> Scope<'scope, T> {
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        // SAFETY: This function blocks until all futures complete, so we do not read/write the
        // data from futures outside of the 'scope lifetime. However, rust has no way of knowing
        // this so we must convert to 'static here to appease the compiler as it is unable to
        // validate safety.
        let fut: Pin<Box<dyn Future<Output = T> + 'scope + Send>> = Box::pin(f);
        let fut: Pin<Box<dyn Future<Output = T> + 'static + Send>> = unsafe { mem::transmute(fut) };

        let task = self.executor.spawn(fut);
        self.spawned.push(task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_spawn() {
        let pool = TaskPool::new();

        let foo = Box::new(42);
        let foo = &*foo;

        let outputs = pool.scope(|scope| {
            for i in 0..100 {
                scope.spawn(async move {
                    println!("task {}", i);
                    if *foo != 42 {
                        panic!("not 42!?!?")
                    } else {
                        *foo
                    }
                });
            }
        });

        for output in outputs {
            assert_eq!(output, 42);
        }
    }
}
