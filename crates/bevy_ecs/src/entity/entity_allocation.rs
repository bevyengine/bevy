use bevy_platform_support::{
    prelude::Vec,
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc,
    },
};
use core::mem::MaybeUninit;

use bevy_utils::syncunsafecell::SyncUnsafeCell;

use super::Entity;

/// This is the item we store in the owned buffers.
/// It might not be init (if it's out of bounds).
/// It's in an unsafe cell (makes things simpler.)
type Slot = MaybeUninit<SyncUnsafeCell<Entity>>;

/// Each chunk stores a buffer of [`Slot`]s at a fixed capacity.
struct Chunk {
    /// Points to the first slot. If this is null, we need to allocate it.
    first: AtomicPtr<Slot>,
}

impl Chunk {
    const NUM_CHUNKS: u32 = 24;

    /// Computes the capacity of the chunk at this index within [`Self::NUM_CHUNKS`].
    /// The first 2 have length 512 (2^9) and the last has length (2^31)
    fn capacity_of_chunk(chunk_index: u32) -> u32 {
        // We do this because we're skipping the first 8 powers, so we need to make up for them by doubling the first index.
        // This is why the first 2 indices both have a capacity of 256.
        let corrected = chunk_index.max(1);
        // We add 8 because the total capacity should be as if [`Self::NUM_CHUNKS`] were 32.
        // This skips the first 8 powers.
        let corrected = corrected + 8;
        // This bit shift is just 2^corrected.
        1 << corrected
    }

    /// For this index in the whole buffer, returns the index of the [`Chunk`] and the index within that chunk.
    fn get_indices(full_idnex: u32) -> (u32, u32) {
        let leading = full_idnex.leading_zeros().min(Self::NUM_CHUNKS - 1);
        let chunk_index = Self::NUM_CHUNKS - 1 - leading;
        let slice_index = full_idnex & !Self::capacity_of_chunk(chunk_index);
        (chunk_index, slice_index)
    }
}

/// This is the shared data for the owned list.
/// It is the source of truth.
struct OwnedBuffer {
    /// Each chunk has a length the power of 2.
    chunks: [Chunk; Chunk::NUM_CHUNKS as usize],
    /// This is the total length of the buffer; the sum of each of the [`chunks`](Self::chunks)'s length.
    len: AtomicUsize,
    /// This points to the index in this buffer of the first [`Slot`] that is pending reuse.
    free_cursor: AtomicUsize,
}

/// This is the owned list.
/// It contains all entities owned by an allocator.
/// This includes empty archetype entities and entities pending reuse.
pub struct Owned {
    buffer: Arc<OwnedBuffer>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Ensure the total capacity of [`OwnedBuffer`] is `u32::MAX + 1`, since the max *index* of an [`Entity`] is `u32::MAX`.
    #[test]
    fn chunk_capacity_sums() {
        let total: usize = (0..Chunk::NUM_CHUNKS)
            .map(Chunk::capacity_of_chunk)
            .map(|x| x as usize)
            .sum();
        let expected = u32::MAX as usize + 1;
        assert_eq!(total, expected);
    }

    /// Ensure [`OwnedBuffer`] can be properly indexed
    #[test]
    fn chunk_indexing() {
        let to_test = vec![
            (0, (0, 0)), // index 0 cap = 512
            (1, (0, 1)),
            (256, (0, 256)),
            (511, (0, 511)),
            (512, (1, 0)), // index 1 cap = 512
            (1023, (1, 511)),
            (1024, (2, 0)), // index 2 cap = 1024
            (1025, (2, 1)),
            (2047, (2, 1023)),
            (2048, (3, 0)), // index 3 cap = 2048
            (4095, (3, 2047)),
            (4096, (4, 0)), // index 3 cap = 4096
        ];

        for (input, output) in to_test {
            assert_eq!(Chunk::get_indices(input), output);
        }
    }
}
