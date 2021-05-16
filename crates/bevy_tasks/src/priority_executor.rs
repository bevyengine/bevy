use std::future::Future;

use async_executor::{Executor, Task};
use event_listener::Event;
use futures_lite::FutureExt;

/// Task priority.
#[derive(Debug, Clone, Copy)]
pub enum Priority {
    FinishWithinFrame = 0,
    AcrossFrame = 1,
    IO = 2,
}

#[derive(Debug)]
pub struct PriorityExecutor<'a> {
    ex: [Executor<'a>; 3],
    event: Event,
}

impl<'a> Default for PriorityExecutor<'a> {
    fn default() -> Self {
        PriorityExecutor::new()
    }
}

impl<'a> PriorityExecutor<'a> {
    /// Creates a new executor.
    pub fn new() -> PriorityExecutor<'a> {
        PriorityExecutor {
            ex: [Executor::new(), Executor::new(), Executor::new()],
            event: Event::new(),
        }
    }

    pub fn spawn<T: Send + 'a>(
        &self,
        priority: Priority,
        future: impl Future<Output = T> + Send + 'a,
    ) -> Task<T> {
        self.ex[priority as usize].spawn(future)
    }

    pub async fn run<T>(&self, future: impl Future<Output = T>) -> T {
        let run_forever = async {
            loop {
                while self.ex[0].try_tick() {}

                let t1 = self.ex[1].tick();
                let t2 = self.ex[2].tick();
                self.event.listen().or(t1).or(t2).await
            }
        };

        future.or(run_forever).await
    }

    pub fn tick(&self) {
        self.event.notify(usize::MAX);
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::time::Duration;

    use futures_lite::future;
    use instant::Instant;

    use super::*;

    #[inline(never)]
    async fn a_task(duration: Duration, priority: Priority, counters: Arc<RwLock<[usize; 3]>>) {
        let start = Instant::now();
        while Instant::now() - start < duration {}
        counters.write().unwrap()[priority as usize] += 1;
    }

    #[test]
    fn test_priority_executor() {
        let tasks_completed: Arc<RwLock<[usize; 3]>> = Arc::new(RwLock::new([0, 0, 0]));
        let mut tasks_dispatched: [usize; 3] = [0, 0, 0];
        let executor = PriorityExecutor::new();
        let executor = Arc::new(executor);
        let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

        let num_cpus = num_cpus::get();
        let mut job_handles = (0..num_cpus)
            .map(|_| {
                let executor = executor.clone();
                let shutdown_rx = shutdown_rx.clone();
                // Spawn a thread running the executor forever.

                thread::spawn(move || {
                    let shutdown_future = executor.run(shutdown_rx.recv());
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(shutdown_future).unwrap_err();
                })
            })
            .collect::<Vec<_>>();

        let mut tasks: Vec<(Task<_>, Priority)> = Vec::new();
        let priority_choice = [
            Priority::FinishWithinFrame,
            Priority::AcrossFrame,
            Priority::IO,
        ];

        let time_choice = [
            Duration::new(0, 500_000),
            Duration::new(0, 50_000_000),
            Duration::new(0, 500_000_000),
        ];

        let epochs: [u32; 3] = [num_cpus as u32 * 30 / 4, num_cpus as u32 * 2 / 4, 1];
        let task_periods: [u32; 3] = [1, 6, 36];

        let mut task_period_counter = [0, 0, 0];
        let mut frame_counter = 0;

        loop {
            if frame_counter >= 60 {
                break;
            }

            for i in 0..3 {
                task_period_counter[i] += 1;
                if task_period_counter[i] < task_periods[i] {
                    continue;
                }
                let priority = priority_choice[i];
                let time = time_choice[i];
                let epoch = epochs[i];

                tasks_dispatched[priority as usize] += epoch as usize;
                for _ in 0..epoch {
                    tasks.push((
                        executor.spawn(priority, a_task(time, priority, tasks_completed.clone())),
                        priority,
                    ));
                }
                task_period_counter[i] = 0;
            }

            frame_counter += 1;
            executor.tick();

            let mut i = 0;
            while i < tasks.len() {
                match tasks[i].1 {
                    Priority::FinishWithinFrame => {
                        let task = tasks.swap_remove(i);
                        future::block_on(task.0);
                    }
                    Priority::AcrossFrame => {
                        i += 1;
                    }
                    Priority::IO => {
                        i += 1;
                    }
                }
            }
            let tasks_completed = *tasks_completed.read().unwrap();

            assert_eq!(tasks_completed[0], tasks_dispatched[0]);
        }
        for task in tasks.drain(..) {
            future::block_on(async { task.0.await });
        }
        shutdown_tx.close();
        for (task_completed, task_dispatched) in tasks_completed
            .read()
            .unwrap()
            .iter()
            .zip(tasks_dispatched.iter())
        {
            assert_eq!(*task_completed, *task_dispatched)
        }

        // if the test does not finish, there is potential deadlock
        for job_handle in job_handles.drain(..) {
            job_handle
                .join()
                .expect("Task thread panicked while executing.");
        }
    }
}
