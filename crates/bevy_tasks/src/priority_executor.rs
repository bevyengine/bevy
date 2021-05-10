use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_executor::{Executor, Task};
use event_listener::Event;
use futures_lite::{future, FutureExt};

/// Task priority.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum Priority {
    FinishWithinFrame = 0,
    AcrossFrame = 1,
    IO = 2,
}

#[derive(Debug)]
pub struct PriorityExecutor<'a> {
    ex: [Executor<'a>; 3],
    event: Arc<Event>,
    worker: AtomicUsize,
}

impl<'a> PriorityExecutor<'a> {
    /// Creates a new executor.
    pub fn new() -> PriorityExecutor<'a> {
        PriorityExecutor {
            ex: [Executor::new(), Executor::new(), Executor::new()],
            event: Arc::new(Event::new()),
            worker: AtomicUsize::new(0),
        }
    }

    pub fn spawn<T: Send + 'a>(
        &self,
        priority: Priority,
        future: impl Future<Output=T> + Send + 'a,
    ) -> Task<T> {
        self.ex[priority as usize].spawn(future)
    }

    pub async fn run<T>(&self, future: impl Future<Output=T>) -> T {
        let event = self.event.clone();

        self.worker.fetch_add(1, Ordering::SeqCst);

        let run_forever = async {
            loop {
                // idling
                future::block_on(async {
                    let listener = event.listen();
                    future::yield_now()
                        .or(listener)
                        .await
                });

                future::block_on(async {
                    loop {
                        if !self.ex[0].try_tick() {
                            break;
                        }
                    }
                    let t1 = self.ex[1].tick();
                    let t2 = self.ex[2].tick();

                    future::yield_now()
                        .or(t1)
                        .or(t2)
                        .await;
                });
            }
        };

        // Run `future` and `run_forever` concurrently until `future` completes.
        let result = future.or(run_forever).await;
        self.worker.fetch_sub(1, Ordering::SeqCst);
        result
    }

    pub fn tick(&self) {
        self.event.notify(self.worker.load(Ordering::SeqCst));
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
        {
            let executor = PriorityExecutor::new();
            let executor = Arc::new(executor);
            let (shutdown_tx, shutdown_rx) = async_channel::unbounded::<()>();

            let num_cpus = num_cpus::get();

            for _ in 0..num_cpus {
                let executor = executor.clone();
                let shutdown_rx = shutdown_rx.clone();
                // Spawn a thread running the executor forever.

                thread::spawn(move || {
                    let shutdown_future = executor.run(shutdown_rx.recv());
                    // Use unwrap_err because we expect a Closed error
                    future::block_on(shutdown_future).unwrap_err();
                    //println!("worker thread {} exits", x)
                });
            }

            let mut tasks: Vec<(Task<_>, Priority)> = Vec::new();
            let priority_choice = [Priority::FinishWithinFrame, Priority::AcrossFrame, Priority::IO];

            let time_choice = [
                Duration::new(0, 500_000),
                Duration::new(0, 50_000_000),
                Duration::new(0, 500_000_000),
            ];

            let epochs: [u32; 3] = [num_cpus as u32 * 30 / 4, num_cpus as u32 * 2 / 4, 1];
            let task_periods: [u32; 3] = [1, 6, 36];
            println!("Task epochs {:?}", epochs);
            println!("Task periods {:?}", task_periods);

            let mut task_period_counter = [0, 0, 0];
            let mut frame_time = Instant::now();
            let mut frame_counter = 0;

            loop {
                if frame_counter >= 60 {
                    break;
                }
                frame_time = Instant::now();
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
                        tasks.push(
                            (executor.spawn(
                                priority,
                                a_task(time, priority, tasks_completed.clone()),
                            ), priority)
                        );
                    }
                    task_period_counter[i] = 0;
                }
                //println!("{}: {:?}", tasks.len(), tasks);

                frame_counter += 1;
                //println!("main sender: {}", frame_counter);
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
                println!("Frame time: {:?}", Instant::now() - frame_time);
                println!("Tasks Dispatched: {:?}", tasks_dispatched);
                println!("Tasks Completed:  {:?}", tasks_completed);
            }
        }
        //println!("exited main loop");

        // dropping of priority executor will not close all workers thread,
        // they will exit after all their jobs are done
        std::thread::sleep(Duration::new(1, 0));

        for (task_completed, task_dispatched) in tasks_completed
            .read()
            .unwrap()
            .iter()
            .zip(tasks_dispatched.iter()) {
            assert_eq!(*task_completed, *task_dispatched)
        }
        //println!("Tasks Dispatched: {:?}", tasks_dispatched);
        //println!("Tasks Completed:  {:?}", *tasks_completed.read().unwrap());
    }
}