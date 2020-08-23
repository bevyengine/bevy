use multitask::{Executor, Task};
use parking::Unparker;
use std::{
    fmt::{self, Debug},
    future::Future,
    mem,
    pin::Pin,
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

macro_rules! pin_mut {
    ($($x:ident),*) => { $(
        // Move the value to ensure that it is owned
        let mut $x = $x;
        // Shadow the original binding so that it can't be directly accessed
        // ever again.
        #[allow(unused_mut)]
        let mut $x = unsafe {
            Pin::new_unchecked(&mut $x)
        };
    )* }
}

static GLOBAL_TASK_POOL: AtomicPtr<TaskPool> = AtomicPtr::new(ptr::null_mut());

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

    pub fn install(self) {
        let mut pool = Box::new(self.build());
        if !GLOBAL_TASK_POOL
            .compare_and_swap(ptr::null_mut(), &mut*pool, Ordering::SeqCst)
            .is_null()
        {
            panic!("GLOBAL_TASK_POOL can only be set once");
        }
        mem::forget(pool);
    }
}

pub struct TaskPool {
    executor: Arc<Executor>,
    threads: Vec<(JoinHandle<()>, Arc<Unparker>)>,
    shutdown_flag: Arc<AtomicBool>,
}

impl TaskPool {
    // Create a `TaskPool` with the default configuration.
    pub fn new() -> TaskPool {
        TaskPoolBuilder::new().build()
    }

    #[inline(always)]
    pub fn global() -> &'static TaskPool {
        #[inline(never)]
        #[cold]
        fn do_panic() {
            panic!(
                "A global task pool must be installed before `TaskPool::global()` can be called."
            );
        }

        let ptr = GLOBAL_TASK_POOL.load(Ordering::Acquire);
        if ptr.is_null() {
            do_panic();
        }
        unsafe { &*ptr }
    }

    fn new_internal(
        num_threads: Option<usize>,
        stack_size: Option<usize>,
        thread_name: Option<&str>,
    ) -> Self {
        let executor = Arc::new(Executor::new());
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
            threads,
            shutdown_flag,
        }
    }

    pub fn thread_num(&self) -> usize {
        self.threads.len()
    }

    pub fn scope<'scope, F, T>(&self, f: F) -> Vec<T>
    where
        F: FnOnce(&mut Scope<'scope, T>) + 'scope + Send,
        T: Send + 'static,
    {
        // let ex = Arc::clone(&self.executor);
        let executor: &'scope Executor = unsafe { mem::transmute(&*self.executor) };

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

        pin_mut!(fut);

        // let fut: Pin<&mut (dyn Future<Output=()> + Send)> = fut;
        let fut: Pin<&'static mut (dyn Future<Output = Vec<T>> + Send + 'static)> =
            unsafe { mem::transmute(fut as Pin<&mut (dyn Future<Output = Vec<T>> + Send)>) };

        pollster::block_on(self.executor.spawn(fut))
    }

    pub fn shutdown(self) -> Result<(), ThreadPanicked> {
        let mut this = self;
        this.shutdown_internal()
    }

    fn shutdown_internal(&mut self) -> Result<(), ThreadPanicked> {
        self.shutdown_flag.store(true, Ordering::Release);

        for (_, unparker) in &self.threads {
            unparker.unpark();
        }
        for (join_handle, _) in self.threads.drain(..) {
            join_handle
                .join()
                .expect("task thread panicked while executing");
        }
        Ok(())
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.shutdown_internal().unwrap();
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ThreadPanicked(());

impl Debug for ThreadPanicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a task thread panicked during execution")
    }
}

pub struct Scope<'scope, T> {
    executor: &'scope Executor,
    spawned: Vec<Task<T>>,
}

impl<'scope, T: Send + 'static> Scope<'scope, T> {
    pub fn spawn<Fut: Future<Output = T> + 'scope + Send>(&mut self, f: Fut) {
        let fut: Pin<Box<dyn Future<Output = T> + 'scope + Send>> = Box::pin(f);
        let fut: Pin<Box<dyn Future<Output = T> + 'static + Send>> = unsafe { mem::transmute(fut) };

        let task = self.executor.spawn(fut);
        self.spawned.push(task);
    }
}

pub fn scope<'scope, F, T>(f: F) -> Vec<T>
where
    F: FnOnce(&mut Scope<'scope, T>) + 'scope + Send,
    T: Send + 'static,
{
    TaskPool::global().scope(f)
}

pub fn global_thread_num() -> usize {
    TaskPool::global().thread_num()
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

    #[test]
    pub fn test_global_spawn() {
        TaskPoolBuilder::new().install();

        let foo = Box::new(42);
        let foo = &*foo;

        let outputs = scope(|scope| {
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
