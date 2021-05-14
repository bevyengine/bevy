use std::future::Future;
use std::sync::atomic::AtomicUsize;

use async_executor::{Executor, Task};
use event_listener::Event;
use futures_lite::FutureExt;

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
    event: Event,
    worker: AtomicUsize,
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
            worker: AtomicUsize::new(0),
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
                loop {
                    if !self.ex[0].try_tick() {
                        break;
                    }
                }

                let t1 = self.ex[1].tick();
                let t2 = self.ex[2].tick();
                self.event.listen().or(t1).or(t2).await
            }
        };

        let result = future.or(run_forever).await;
        result
    }

    pub fn tick(&self) {
        self.event.notify(usize::MAX);
    }
}
