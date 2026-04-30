use core::sync::atomic::{AtomicU32, Ordering};

use crate::change_detection::Tick;

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

    pub(crate) fn note_changed(&self, tick: Tick) {
        let mut val = self.page_table.load(Ordering::Relaxed);
        // FIXME: this CAS loop needs to be smarter and do the "is newer than" dance
        if val < tick.get() {
            loop {
                match self.page_table.compare_exchange_weak(
                    val,
                    tick.get(),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(old) => val = old,
                }
            }
        }
    }

    pub(crate) fn is_dirty(&self, since: Tick, now: Tick) -> bool {
        let last_changed = Tick::new(self.page_table.load(Ordering::Relaxed));
        last_changed.is_newer_than(since, now)
    }
}
