use crate::Task;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, TryLockResult};

#[derive(Debug)]
pub struct PollableTask<T> {
    result: Arc<RwLock<Option<T>>>,
    // this is to keep the task alive
    _task: Task<()>,
}

impl<T> PollableTask<T> {
    pub(crate) fn new(result: Arc<RwLock<Option<T>>>, task: Task<()>) -> Self {
        Self {
            result,
            _task: task,
        }
    }

    pub fn poll(&self) -> LockResult<RwLockReadGuard<Option<T>>> {
        self.result.read()
    }

    pub fn try_poll(&self) -> TryLockResult<RwLockReadGuard<Option<T>>> {
        self.result.try_read()
    }
}
