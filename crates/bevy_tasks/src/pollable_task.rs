use crate::Task;
use async_channel::{Receiver, TryRecvError};

/// A pollable task whose result readiness can be checked in system functions
/// on every frame update without blocking on a future
#[derive(Debug)]
pub struct PollableTask<T> {
    receiver: Receiver<T>,
    // this is to keep the task alive
    _task: Task<()>,
}

impl<T> PollableTask<T> {
    pub(crate) fn new(receiver: Receiver<T>, task: Task<()>) -> Self {
        Self {
            receiver,
            _task: task,
        }
    }

    /// poll to see whether the task finished
    pub fn poll(&self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(value) => Some(value),
            Err(try_error) => match try_error {
                TryRecvError::Empty => None,
                TryRecvError::Closed => {
                    panic!("Polling on the task failed because the connection was already closed.")
                }
            },
        }
    }
}
