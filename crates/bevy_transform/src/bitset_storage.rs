#![expect(unsafe_code, reason = "Implementing an atomic bitset is ")]

#[cfg(feature = "multi_threaded")]
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(feature = "multi_threaded"))]
use core::cell::Cell;

use alloc::{boxed::Box, vec::Vec};
use bevy_ecs::storage::{TableId, TableRow};

type Chunk = cfg_select! {
    all(feature = "multi_threaded", target_has_atomic = "64") => AtomicU64,
    all(feature = "multi_threaded", target_has_atomic = "32") => AtomicU32,
    all(feature = "multi_threaded", target_has_atomic = "16") => AtomicU16,
    all(feature = "multi_threaded", target_has_atomic = "8") => AtomicU8,
    not(feature = "multi_threaded") => Cell::<u64>,
};

const BITS: usize = 8 * size_of::<Chunk>();

/// A chunk for a bitset.
///
/// The zeroed bit pattern is a valid pattern for this type.
#[repr(transparent)]
struct BitsetChunk {
    chunk: Chunk,
}

impl BitsetChunk {
    /// Creates a new zeroed `BitsetChunk`
    #[inline(always)]
    pub const fn zeroed() -> Self {
        Self {
            chunk: Chunk::new(0),
        }
    }

    #[inline(always)]
    pub fn get(&self, row: TableRow) -> bool {
        #[cfg(feature = "multi_threaded")]
        let bits = self.chunk.load(Ordering::Relaxed);

        #[cfg(not(feature = "multi_threaded"))]
        let bits = self.chunk.get();

        bits & (1 << BitsetChunk::index_in_chunk_for(row)) != 0
    }

    #[inline(always)]
    pub fn set(&self, row: TableRow) {
        let mask = 1 << BitsetChunk::index_in_chunk_for(row);

        #[cfg(feature = "multi_threaded")]
        {
            self.chunk.fetch_or(mask, Ordering::Relaxed);
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            self.chunk.set(self.chunk.get() | mask)
        }
    }

    #[inline(always)]
    pub const fn index_for_chunk(row: TableRow) -> usize {
        row.index() / BITS
    }

    #[inline(always)]
    pub const fn index_in_chunk_for(row: TableRow) -> usize {
        row.index() % BITS
    }
}

/// A dense table storage based on bitsets.
#[derive(Default)]
pub struct BitsetStorage {
    /// A two level map `TableId` -> `TableRow` chunk index -> chunk containing the bits for that table row
    bitsets: Vec<Box<[BitsetChunk]>>,
}

impl BitsetStorage {
    /// Creates a new bitset storage.
    pub fn new() -> Self {
        Self {
            bitsets: Vec::new(),
        }
    }

    /// Resets the storage for the table `table` for `len` entities.
    ///
    /// After this method has returned all rows less than `len` for the table `table` will be in bounds.
    pub fn reset_table(&mut self, table: TableId, len: usize) {
        if self.bitsets.len() <= table.as_usize() {
            self.bitsets
                .resize_with(table.as_usize() + 1, || Box::new([]));
        }

        // SAFETY: We just resized the Vec to have at least table.as_usize() + 1 entries,
        // so table.as_usize() is valid index.
        let bitset = unsafe { self.bitsets.get_unchecked_mut(table.as_usize()) };

        let chunks_len = len.div_ceil(BITS);

        if bitset.len() < chunks_len {
            // We should resize the bitset here, but since we also want to reset it to 0
            // we don't need to reuse the existing memory, which will cause it to be copied if

            // Free the existing bitset
            *bitset = Box::new([]);
            // Allocate a new bitset of the right size
            // SAFETY: zero is a valid bit pattern for BitsetChunk
            *bitset = unsafe { Box::new_zeroed_slice(chunks_len).assume_init() };
        } else {
            // If the table length is less than the bitset just fill the necessary bits with 0.
            // No need to fill the whole bitset if we're not gonna use the extra bits.
            bitset[..chunks_len].fill_with(|| const { BitsetChunk::zeroed() });
        }
    }

    /// Returns the chunk storage containing the bit for the row `row` of the table `table`.
    ///
    /// # Safety
    ///
    /// `table` and `row` must be in bounds for this bitset storage.
    #[inline(always)]
    unsafe fn get_bits_unchecked(&self, table: TableId, row: TableRow) -> &BitsetChunk {
        let bitset = self.bitsets.get(table.as_usize());
        // SAFETY: The caller guarantees that table is in bounds.
        let bitset = unsafe { bitset.unwrap_unchecked() };
        let bits = bitset.get(BitsetChunk::index_for_chunk(row));
        // SAFETY: The caller guarantees that row is in bounds.
        unsafe { bits.unwrap_unchecked() }
    }

    /// Returns whether the bit for the row `row` in the table `table` is set.
    ///
    /// # Safety
    ///
    /// `table` and `row` must be in bounds for this bitset storage.
    pub unsafe fn get_unchecked(&self, table: TableId, row: TableRow) -> bool {
        // SAFETY: The caller guarantees table and row are in bounds.
        let bits = unsafe { self.get_bits_unchecked(table, row) };
        bits.get(row)
    }

    /// Sets the bit for the row `row` in the table `table` is set.
    ///
    /// # Safety
    ///
    /// `table` and `row` must be in bounds for this bitset storage.
    pub unsafe fn set_unchecked(&self, table: TableId, row: TableRow) {
        // SAFETY: The caller guarantees table and row are in bounds.
        let bits = unsafe { self.get_bits_unchecked(table, row) };
        bits.set(row);
    }
}
