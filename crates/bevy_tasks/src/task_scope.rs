use crate::ComputeTaskPool;

#[cfg(feature = "multi_threaded")]
use alloc::vec::Vec;
#[cfg(feature = "multi_threaded")]
use concurrent_queue::{ConcurrentQueue, PopError};
#[cfg(feature = "multi_threaded")]
use core::mem::replace;
#[cfg(feature = "multi_threaded")]
use core::sync::atomic::{AtomicU32, Ordering};

#[cfg(not(feature = "multi_threaded"))]
use alloc::vec::Vec;

#[cfg(feature = "multi_threaded")]
use Send as ConditionalSend;
#[cfg(feature = "multi_threaded")]
use Sync as ConditionalSync;

#[cfg(not(feature = "multi_threaded"))]
pub trait ConditionalSend {}
#[cfg(not(feature = "multi_threaded"))]
impl<T: ?Sized> ConditionalSend for T {}

#[cfg(not(feature = "multi_threaded"))]
pub trait ConditionalSync {}
#[cfg(not(feature = "multi_threaded"))]
impl<T: ?Sized> ConditionalSync for T {}

/// A task runner.
///
/// In order to run a task in a `task_scope`
pub trait TaskRunner: Sized + ConditionalSync {
    /// The type of tasks for this runner.
    type Task: ConditionalSend;

    /// The type of some worker local data this runner can use.
    type WorkerLocal<'w>;

    /// Initialize the worker local data for the current worker.
    fn init_local<'w>(&'w self) -> Self::WorkerLocal<'w>;

    /// Runs a task. The task can spawn more tasks on the given `scope`.
    fn run(&self, task: Self::Task, scope: TaskScope<'_, '_, Self>);

    // Dummy method as otherwise the compiler requires having `type WorkerLocal<'w> where Self: 'w`,
    // which is more restrictive than necessary.
    #[doc(hidden)]
    fn __dummy(&self, _: Self::WorkerLocal<'_>) {}
}

/// A scope object for creating new tasks.
pub struct TaskScope<'s, 'l, R: TaskRunner + 'l> {
    local: &'s mut R::WorkerLocal<'l>,

    #[cfg(feature = "multi_threaded")]
    queue: &'s ConcurrentQueue<Vec<R::Task>>,
    #[cfg(feature = "multi_threaded")]
    inflight_tasks: &'s AtomicU32,
    #[cfg(feature = "multi_threaded")]
    chunk_size: usize,
    #[cfg(feature = "multi_threaded")]
    outbox: &'s mut Vec<R::Task>,

    #[cfg(not(feature = "multi_threaded"))]
    runner: &'s R,
}

impl<'s, 'l, R: TaskRunner> TaskScope<'s, 'l, R> {
    /// Returns a reference to the worker local data of this scope.
    pub fn worker_local(&mut self) -> &mut R::WorkerLocal<'l> {
        &mut *self.local
    }

    /// Add a task to the task scope.
    ///
    /// The task may be executed immediately or at a later time.
    pub fn add(&mut self, task: R::Task) {
        #[cfg(feature = "multi_threaded")]
        {
            // Our outbox is full, push the previous outbox and replace it with a new one.
            if self.outbox.len() == self.chunk_size {
                let next_outbox = Vec::with_capacity(self.chunk_size);
                let prev_outbox = replace(self.outbox, next_outbox);
                self.inflight_tasks.fetch_add(1, Ordering::Relaxed);
                self.queue.push(prev_outbox).ok();
            }

            self.outbox.push(task);
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            // When multithreading is disabled, just immediately run the task.
            let scope = TaskScope {
                local: &mut *self.local,
                runner: self.runner,
            };
            self.runner.run(task, scope);
        }
    }

    /// Efficiently add multiple tasks to the task scope.
    ///
    /// The tasks may be executed immediately or at a later time.
    pub fn add_many(&mut self, tasks: impl IntoIterator<Item = R::Task>) {
        #[cfg(feature = "multi_threaded")]
        {
            let mut tasks = tasks.into_iter();

            let remaining_capacity = self.chunk_size - self.outbox.len();
            self.outbox.extend(tasks.by_ref().take(remaining_capacity));

            // As long as the current outbox is full we might have more tasks we didn't push into it.
            while self.outbox.len() >= self.chunk_size {
                // See if there are more tasks.
                let mut next_outbox = Vec::with_capacity(self.chunk_size);
                next_outbox.extend(tasks.by_ref().take(self.chunk_size));

                // If there are no more tasks then exit.
                if next_outbox.is_empty() {
                    break;
                }

                // If there are more tasks then push the previous outbox and keep the new.
                // This is important to always keep some work for the current worker.
                let prev_outbox = replace(self.outbox, next_outbox);

                self.inflight_tasks.fetch_add(1, Ordering::Relaxed);
                self.queue.push(prev_outbox).ok();
            }
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            // When multithreading is disabled, just immediately run the tasks.
            for task in tasks {
                let scope = TaskScope {
                    local: &mut *self.local,
                    runner: self.runner,
                };
                self.runner.run(task, scope);
            }
        }
    }
}

#[cfg(feature = "multi_threaded")]
pub struct InflightTaskGuard<'s, T> {
    queue: &'s ConcurrentQueue<T>,
    inflight_tasks: &'s AtomicU32,
}
#[cfg(feature = "multi_threaded")]
impl<'s, T> InflightTaskGuard<'s, T> {
    pub fn new(queue: &'s ConcurrentQueue<T>, inflight_tasks: &'s AtomicU32) -> Self {
        InflightTaskGuard {
            queue,
            inflight_tasks,
        }
    }
}
#[cfg(feature = "multi_threaded")]
impl<T> Drop for InflightTaskGuard<'_, T> {
    fn drop(&mut self) {
        // We can now mark this inflight task as completed.
        let prev_inflight_tasks = self.inflight_tasks.fetch_sub(1, Ordering::Relaxed);

        // If this was the last task, then close the queue to signal that we're done.
        if prev_inflight_tasks == 1 {
            self.queue.close();
        }
    }
}

/// A worker that is ready to execute.
pub struct TaskWorker<'s, R: TaskRunner> {
    #[cfg(feature = "multi_threaded")]
    queue: &'s ConcurrentQueue<Vec<R::Task>>,
    #[cfg(feature = "multi_threaded")]
    inflight_tasks: &'s AtomicU32,
    #[cfg(feature = "multi_threaded")]
    chunk_size: usize,

    // TODO: Find a way to avoid collecting the tasks in a `Vec` but also avoid including TS in the generics of `TaskWorker`.
    #[cfg(not(feature = "multi_threaded"))]
    tasks: Vec<R::Task>,

    runner: &'s R,
}

impl<R: TaskRunner> TaskWorker<'_, R> {
    /// Executes this worker.
    pub fn run(self) {
        #[cfg(feature = "multi_threaded")]
        {
            let mut outbox = Vec::with_capacity(self.chunk_size);
            let mut local = self.runner.init_local();

            loop {
                // Try getting more tasks
                let mut tasks = match self.queue.pop() {
                    Ok(tasks) => tasks,
                    Err(PopError::Empty) => {
                        // Workers might still produce new tasks, spin waiting for more work.
                        core::hint::spin_loop();
                        continue;
                    }
                    Err(PopError::Closed) => return,
                };

                let guard = InflightTaskGuard::new(self.queue, self.inflight_tasks);

                loop {
                    // Process each task.
                    for task in tasks.drain(..) {
                        let scope = TaskScope {
                            local: &mut local,
                            queue: self.queue,
                            inflight_tasks: self.inflight_tasks,
                            chunk_size: self.chunk_size,
                            outbox: &mut outbox,
                        };
                        self.runner.run(task, scope);
                    }

                    // We finished out list of tasks, check if there are more in the outbox
                    // to avoid contending on the main queue.
                    if outbox.is_empty() {
                        break;
                    }
                    tasks = replace(&mut outbox, tasks);
                }

                // We can now mark this task as not inflight.
                drop(guard);
            }
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            let mut local = self.runner.init_local();

            for task in self.tasks {
                let scope = TaskScope {
                    local: &mut local,
                    runner: self.runner,
                };
                self.runner.run(task, scope)
            }
        }
    }
}

/// Start a task scope, running a list of `tasks` which can potentially spawn other recursive tasks.
///
/// A task scope is a more efficient way of spawning many small tasks of the same type, in particular
/// it batches small tasks together and avoids other tasks interrupting.
pub fn task_scope<TS, TP, W, R>(
    tasks: TS,
    chunk_size: usize,
    task_pool: TP,
    worker_hook: W,
    runner: R,
) where
    TS: IntoIterator<Item = R::Task>,
    TP: FnOnce() -> &'static ComputeTaskPool,
    W: Fn(TaskWorker<'_, R>) + Sync,
    R: TaskRunner,
{
    #[cfg(feature = "multi_threaded")]
    {
        // Have chunk size be at least 1.
        let chunk_size = chunk_size.max(1);

        let queue = ConcurrentQueue::unbounded();
        let inflight_tasks = AtomicU32::new(0);

        // Send the first tasks to the queue.
        let mut tasks = tasks.into_iter();

        // Produce the first chunk of tasks, if there's no task then exit early.
        let mut chunk = Vec::with_capacity(chunk_size);
        chunk.extend(tasks.by_ref().take(chunk_size));
        if chunk.is_empty() {
            // There's no work, exit.
            return;
        }

        let run_worker = || {
            let worker = TaskWorker {
                queue: &queue,
                inflight_tasks: &inflight_tasks,
                chunk_size,
                runner: &runner,
            };

            worker_hook(worker);
        };

        let task_pool = task_pool();
        task_pool.scope(|s| {
            // Fake an inflight task to prevent the workers from stopping while we send the tasks.
            inflight_tasks.fetch_add(1, Ordering::Relaxed);

            let guard = InflightTaskGuard::new(&queue, &inflight_tasks);

            (1..task_pool.thread_num()) // First worker is run locally instead of the task pool.
                .for_each(|_| {
                    s.spawn(async { run_worker() });
                });

            // Produce here the rest of the chunks, so that the workers can start processing
            // before all chunks are sent.
            while !chunk.is_empty() {
                inflight_tasks.fetch_add(1, Ordering::Relaxed);
                queue.push(chunk).ok();
                chunk = Vec::with_capacity(chunk_size);
                chunk.extend(tasks.by_ref().take(chunk_size));
            }

            drop(guard);

            run_worker();
        });
    }

    #[cfg(not(feature = "multi_threaded"))]
    {
        _ = chunk_size;
        _ = task_pool;

        worker_hook(TaskWorker {
            tasks: tasks.into_iter().collect(),
            runner: &runner,
        })
    }
}
