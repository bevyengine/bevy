//! Types for controlling batching behavior during parallel processing.

use std::ops::Range;

/// Dictates how a parallel query chunks up large tables/archetypes
/// during iteration.
///
/// A parallel query will chunk up large tables and archetypes into
/// chunks of at most a certain batch size.
///
/// By default, this batch size is automatically determined by dividing
/// the size of the largest matched archetype by the number
/// of threads (rounded up). This attempts to minimize the overhead of scheduling
/// tasks onto multiple threads, but assumes each entity has roughly the
/// same amount of work to be done, which may not hold true in every
/// workload.
///
/// See [`Query::par_iter`] for more information.
///
/// [`Query::par_iter`]: crate::system::Query::par_iter
#[derive(Clone)]
pub struct BatchingStrategy {
    /// The upper and lower limits for how large a batch of entities.
    ///
    /// Setting the bounds to the same value will result in a fixed
    /// batch size.
    ///
    /// Defaults to `[1, usize::MAX]`.
    pub batch_size_limits: Range<usize>,
    /// The number of batches per thread in the [`ComputeTaskPool`].
    /// Increasing this value will decrease the batch size, which may
    /// increase the scheduling overhead for the iteration.
    ///
    /// Defaults to 1.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub batches_per_thread: usize,
}

impl BatchingStrategy {
    /// Creates a new unconstrained default batching strategy.
    pub const fn new() -> Self {
        Self {
            batch_size_limits: 1..usize::MAX,
            batches_per_thread: 1,
        }
    }

    /// Declares a batching strategy with a fixed batch size.
    pub const fn fixed(batch_size: usize) -> Self {
        Self {
            batch_size_limits: batch_size..batch_size,
            batches_per_thread: 1,
        }
    }

    /// Configures the minimum allowed batch size of this instance.
    pub const fn min_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size_limits.start = batch_size;
        self
    }

    /// Configures the maximum allowed batch size of this instance.
    pub const fn max_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size_limits.end = batch_size;
        self
    }

    /// Configures the number of batches to assign to each thread for this instance.
    pub fn batches_per_thread(mut self, batches_per_thread: usize) -> Self {
        assert!(
            batches_per_thread > 0,
            "The number of batches per thread must be non-zero."
        );
        self.batches_per_thread = batches_per_thread;
        self
    }
}
