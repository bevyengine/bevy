//! Definitions for a few common task pools that we want. Generally the determining factor for what
//! kind of work should go in each pool is latency requirements.
//!
//! For CPU-intensive work (tasks that generally spin until completion) we have a standard Compute
//! pool and an AsyncCompute pool. Work that does not need to be completed to present the next
//! frame should go to the AsyncCompute pool
//!
//! For IO-intensive work (tasks that spend very little time in a "woken" state) we have an IO
//! task pool. The tasks here are expected to complete very quickly. Generally they should just
//! await receiving data from somewhere (i.e. disk) and signal other systems when the data is ready
//! for consumption. (likely via channels)

use super::TaskPool;
use std::ops::Deref;

/// A newtype for a task pool for CPU-intensive work that must be completed to deliver the next
/// frame
#[derive(Clone, Debug)]
pub struct ComputeTaskPool(pub TaskPool);

impl Deref for ComputeTaskPool {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype for a task pool for CPU-intensive work that may span across multiple frames
#[derive(Clone, Debug)]
pub struct AsyncComputeTaskPool(pub TaskPool);

impl Deref for AsyncComputeTaskPool {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype for a task pool for IO-intensive work (i.e. tasks that spend very little time in a
/// "woken" state)
#[derive(Clone, Debug)]
pub struct IoTaskPool(pub TaskPool);

impl Deref for IoTaskPool {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
