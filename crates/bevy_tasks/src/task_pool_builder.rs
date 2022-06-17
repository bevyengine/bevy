use crate::TaskPool;

/// Defines a simple way to determine how many threads to use given the number of remaining cores
/// and number of total cores
#[derive(Debug, Clone)]
pub struct TaskGroupBuilder {
    /// Force using at least this many threads
    pub(crate) min_threads: usize,
    /// Under no circumstance use more than this many threads for this pool
    pub(crate) max_threads: usize,
    /// Target using this percentage of total cores, clamped by min_threads and max_threads. It is
    /// permitted to use 1.0 to try to use all remaining threads
    pub(crate) percent: f32,
}

impl TaskGroupBuilder {
    /// Force using exactly this many threads
    pub fn threads(&mut self, thread_count: usize) -> &mut Self {
        self.min_threads(thread_count).max_threads(thread_count)
    }

    /// Force using at least this many threads
    pub fn min_threads(&mut self, thread_count: usize) -> &mut Self {
        self.min_threads = thread_count;
        self
    }

    /// Under no circumstance use more than this many threads for this pool
    pub fn max_threads(&mut self, thread_count: usize) -> &mut Self {
        self.max_threads = thread_count;
        self
    }

    /// Target using this percentage of total cores in the range `[0.0, 1.0]`, clamped by
    /// `min_threads` and `max_threads`. Use 1.0 to try to use all remaining threads.
    pub fn percent(&mut self, percent: f32) -> &mut Self {
        self.percent = percent;
        self
    }

    /// Determine the number of threads to use for this task pool
    #[allow(dead_code)] // This is unused on wasm32 platforms
    pub(crate) fn get_number_of_threads(
        &self,
        remaining_threads: usize,
        total_threads: usize,
    ) -> usize {
        assert!(self.percent >= 0.0);
        let mut desired = (total_threads as f32 * self.percent).round() as usize;

        // Limit ourselves to the number of cores available
        desired = desired.min(remaining_threads);

        // Clamp by min_threads, max_threads. (This may result in us using more threads than are
        // available, this is intended. An example case where this might happen is a device with
        // <= 2 threads.
        desired.clamp(self.min_threads, self.max_threads)
    }
}

/// Used to create a [`TaskPool`]
#[derive(Debug, Clone)]
#[must_use]
pub struct TaskPoolBuilder {
    /// If the number of physical cores is less than min_total_threads, force using
    /// min_total_threads
    pub(crate) min_total_threads: usize,
    /// If the number of physical cores is grater than max_total_threads, force using
    /// max_total_threads
    pub(crate) max_total_threads: usize,

    /// Used to determine number of IO threads to allocate
    pub(crate) io: TaskGroupBuilder,
    /// Used to determine number of async compute threads to allocate
    pub(crate) async_compute: TaskGroupBuilder,
    /// Used to determine number of compute threads to allocate
    pub(crate) compute: TaskGroupBuilder,
    /// If set, we'll use the given stack size rather than the system default
    pub(crate) stack_size: Option<usize>,
    /// Allows customizing the name of the threads - helpful for debugging. If set, threads will
    /// be named <thread_name> (<thread_index>), i.e. "MyThreadPool (2)"
    pub(crate) thread_name: Option<String>,
}

impl Default for TaskPoolBuilder {
    fn default() -> Self {
        Self {
            // By default, use however many cores are available on the system
            min_total_threads: 1,
            max_total_threads: std::usize::MAX,

            stack_size: None,
            thread_name: None,

            // Use 25% of cores for IO, at least 1, no more than 4
            io: TaskGroupBuilder {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },

            // Use 25% of cores for async compute, at least 1, no more than 4
            async_compute: TaskGroupBuilder {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },

            // Use all remaining cores for compute (at least 1)
            compute: TaskGroupBuilder {
                min_threads: 1,
                max_threads: std::usize::MAX,
                percent: 1.0, // This 1.0 here means "whatever is left over"
            },
        }
    }
}

impl TaskPoolBuilder {
    /// Creates a new [`TaskPoolBuilder`] instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Force using exactly this many threads
    pub fn threads(self, thread_count: usize) -> Self {
        self.min_threads(thread_count).max_threads(thread_count)
    }

    /// Force using at least this many threads
    pub fn min_threads(mut self, thread_count: usize) -> Self {
        self.min_total_threads = thread_count;
        self
    }

    /// Under no circumstance use more than this many threads for this pool
    pub fn max_threads(mut self, thread_count: usize) -> Self {
        self.max_total_threads = thread_count;
        self
    }

    /// Configure the group options for [`TaskGroup::Compute`].
    ///
    /// [`TaskGroup::Compute`]: crate::TaskGroup::Compute
    pub fn compute<F: FnOnce(&mut TaskGroupBuilder)>(mut self, builder: F) -> Self {
        builder(&mut self.compute);
        self
    }

    /// Configure the group options for [`TaskGroup::AsyncCompute`].
    ///
    /// [`TaskGroup::AsyncCompute`]: crate::TaskGroup::AsyncCompute
    pub fn async_compute<F: FnOnce(&mut TaskGroupBuilder)>(mut self, builder: F) -> Self {
        builder(&mut self.async_compute);
        self
    }

    /// Configure the group options for [`TaskGroup::IO`].
    ///
    /// [`TaskGroup::IO`]: crate::TaskGroup::IO
    pub fn io<F: FnOnce(&mut TaskGroupBuilder)>(mut self, builder: F) -> Self {
        builder(&mut self.io);
        self
    }

    /// Override the name of the threads created for the pool. If set, threads will
    /// be named `<thread_name> (<thread_group>, <thread_index>)`, i.e. `MyThreadPool (IO, 2)`
    pub fn thread_name(mut self, name: impl Into<String>) -> Self {
        self.thread_name = Some(name.into());
        self
    }

    /// Override the stack size of the threads created for the pool
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Creates a new [`TaskPool`] based on the current options.
    pub fn build(self) -> TaskPool {
        TaskPool::new_internal(self)
    }
}
