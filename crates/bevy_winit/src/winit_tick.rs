use std::sync::atomic::{AtomicBool, Ordering};

/// A resource for dynamically control of game tick.
pub struct WinitTick {
    default_poll_tick: bool,
    should_poll_tick: AtomicBool,
}

impl Default for WinitTick {
    fn default() -> Self {
        Self::new(true)
    }
}

impl WinitTick {
    /// Create a `WinitTick` resource with the specified `default_poll_tick`.
    /// `false` is recommended for GUI applications, animation systems can call
    /// `poll_tick` with `WinitTick` resource to poll game tick for the next frame.
    ///
    /// Default value of `default_poll_tick` parameter is `true`.
    pub fn new(default_poll_tick: bool) -> Self {
        Self {
            default_poll_tick,
            should_poll_tick: AtomicBool::new(default_poll_tick),
        }
    }

    pub fn poll_tick(&self) {
        self.should_poll_tick.store(true, Ordering::Relaxed);
    }

    pub fn finish(&self) -> bool {
        self.should_poll_tick
            .swap(self.default_poll_tick, Ordering::Relaxed)
    }
}
