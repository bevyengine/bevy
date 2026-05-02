use core::sync::atomic::{AtomicU32, Ordering};

use crate::change_detection::{Tick, MAX_CHANGE_AGE};

#[derive(Default, Debug)]
pub struct ChangeIndex {
    pub page_table: AtomicU32,
}

impl Clone for ChangeIndex {
    fn clone(&self) -> Self {
        Self {
            page_table: AtomicU32::new(self.page_table.load(Ordering::Relaxed)),
        }
    }
}

impl ChangeIndex {
    pub(crate) fn note_added(&mut self, tick: Tick) {
        // FIXME: Use CAS here to avoid running backwards!
        self.page_table.store(tick.get(), Ordering::Relaxed);
    }

    pub(crate) fn note_changed(&self, _: Tick, now: Tick) {
        let mut then = self.page_table.load(Ordering::Relaxed);
        while then != now.get() && now.get().wrapping_sub(then) < MAX_CHANGE_AGE {
            match self.page_table.compare_exchange_weak(
                then,
                now.get(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(prev) => then = prev,
            }
        }
    }

    pub(crate) fn is_dirty(&self, since: Tick, now: Tick) -> bool {
        let last_changed = Tick::new(self.page_table.load(Ordering::Relaxed));
        last_changed.is_newer_than(since, now)
    }
}
