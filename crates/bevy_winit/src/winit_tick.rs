use std::sync::atomic::{AtomicBool, Ordering};

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
