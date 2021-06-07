/// The policy used when a task pool's internal thread panics
#[derive(Copy, Clone, Debug)]
pub enum TaskPoolThreadPanicPolicy {
    /// Propagate the panic to the main thread, causing the main
    /// thread to panic as well.
    Propagate,
    /// Restart the thread by joining the panicked thread and
    /// spawning another one in it's place.
    Restart,
}
