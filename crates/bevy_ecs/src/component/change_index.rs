use alloc::vec::Vec;
use core::{
    ops::Range,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{change_detection::Tick, storage::TableRow};

#[cfg(feature = "big_pages")]
pub(crate) const PAGE_SIZE: u32 = 4096;
#[cfg(not(feature = "big_pages"))]
pub(crate) const PAGE_SIZE: u32 = 256;

#[derive(Default, Debug)]
pub struct ChangeIndex {
    pub page_table: Vec<AtomicU32>,
}

impl Clone for ChangeIndex {
    fn clone(&self) -> Self {
        Self {
            page_table: self
                .page_table
                .iter()
                .map(|word| AtomicU32::new(word.load(Ordering::Relaxed)))
                .collect(),
        }
    }
}

impl ChangeIndex {
    pub(crate) fn note_added(&mut self, row: TableRow, tick: Tick) {
        let page = row.index_u32() / PAGE_SIZE;
        // TODO: reserve
        while (page as usize) >= self.page_table.len() {
            self.page_table.push(AtomicU32::new(tick.get()));
        }
        self.page_table[page as usize].store(tick.get(), Ordering::Relaxed);
    }

    pub(crate) fn note_changed(&self, row: TableRow, tick: Tick) {
        let page = row.index_u32() / PAGE_SIZE;
        debug_assert!((page as usize) < self.page_table.len());
        self.page_table[page as usize].store(tick.get(), Ordering::Relaxed);
    }

    pub(crate) fn advance_row(&self, mut rows: Range<u32>, since: Tick, now: Tick) -> u32 {
        while !rows.is_empty() {
            let page = rows.start / PAGE_SIZE;
            let next_page_row_index = ((page + 1) * PAGE_SIZE).min(rows.end);
            let Some(page_tick) = self.page_table.get(page as usize) else {
                break;
            };
            if Tick::new(page_tick.load(Ordering::Relaxed)).is_newer_than(since, now) {
                break;
            }

            // This page is clean. Skip to the first entity in the next page.
            rows.start = next_page_row_index;
        }

        rows.start
    }
}
