use bevy_platform_support::{
    prelude::Vec,
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc,
    },
};
use core::mem::{ManuallyDrop, MaybeUninit};

use crate::query::DebugCheckedUnwrap;

use super::Entity;

/// This is the item we store in the owned buffers.
/// It might not be init (if it's out of bounds).
type Slot = MaybeUninit<Entity>;

/// Each chunk stores a buffer of [`Slot`]s at a fixed capacity.
struct Chunk {
    /// Points to the first slot. If this is null, we need to allocate it.
    first: AtomicPtr<Slot>,
    /// The idnex of this chunk.
    index: u32,
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
        // We're countint leading zeros since each chunk has power of 2 capacity.
        // So the leading zeros will be proportional to the chunk index.
        let leading = full_idnex
            .leading_zeros()
            // We do a min because we skip the first 8 powers.
            // The -1 is because this is the number of chunks, but we want the index in the end.
            .min(Self::NUM_CHUNKS - 1);
        // We store chunks in smallest to biggest order, so we need to reverse it.
        let chunk_index = Self::NUM_CHUNKS - 1 - leading;
        // We only need to cut of this particular bit.
        // The capacity is only one bit, and if other bits needed to be dropped, `leading` would have been greater
        let slice_index = full_idnex & !Self::capacity_of_chunk(chunk_index);

        (chunk_index, slice_index)
    }

    /// Gets the entity at the index within this chunk.
    ///
    /// # Safety
    ///
    /// The chunk must be valid, the index must be in bounds, and the [`Slot`] must be init.
    unsafe fn get(&self, index: u32) -> Entity {
        // SAFETY: caller ensure we are init.
        let head = unsafe { self.ptr().debug_checked_unwrap() };
        let target = head.add(index as usize);

        // SAFETY: Ensured by caller.
        unsafe { (*target).assume_init() }
    }

    /// Sets this entity at this index.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    /// Index must be in bounds.
    unsafe fn set(&self, index: u32, entity: Entity) {
        let head = self.ptr().unwrap_or_else(|| self.init());
        let target = head.add(index as usize);

        // SAFETY: Ensured by caller.
        unsafe { (*target).write(entity) };
    }

    /// Initializes the chunk to be valid, returning the pointer.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    #[cold]
    unsafe fn init(&self) -> *mut Slot {
        let cap = Self::capacity_of_chunk(self.index);
        let mut buff = ManuallyDrop::new(Vec::new());
        buff.reserve_exact(cap as usize);
        let ptr = buff.as_mut_ptr();
        self.first.store(ptr, Ordering::Relaxed);
        ptr
    }

    /// Returns [`Self::first`] if it is valid.
    #[inline]
    fn ptr(&self) -> Option<*mut Slot> {
        let ptr = self.first.load(Ordering::Relaxed);
        (!ptr.is_null()).then_some(ptr)
    }

    fn new(index: u32) -> Self {
        Self {
            first: AtomicPtr::new(core::ptr::null_mut()),
            index,
        }
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        if let Some(to_drop) = self.ptr() {
            let cap = Self::capacity_of_chunk(self.index) as usize;
            // SAFETY: This was created in [`Self::init`] from a standard Vec.
            unsafe {
                Vec::from_raw_parts(to_drop, cap, cap);
            }
        }
    }
}

/// This is the shared data for the owned list.
/// It is the source of truth.
struct OwnedBuffer {
    /// Each chunk has a length the power of 2.
    /// We store chunks in smallest to biggest order.
    chunks: [Chunk; Chunk::NUM_CHUNKS as usize],
    /// This is the total length of the buffer; the sum of each of the [`chunks`](Self::chunks)'s length.
    len: AtomicUsize,
    /// This points to the index in this buffer of the first [`Slot`] that is pending reuse.
    free_cursor: AtomicUsize,
}

impl OwnedBuffer {
    fn new() -> Self {
        let base = [(); Chunk::NUM_CHUNKS as usize];
        let mut index = 0u32;
        let chunks = base.map(|()| {
            let chunk = Chunk::new(index);
            index += 1;
            chunk
        });
        Self {
            chunks,
            len: AtomicUsize::new(0),
            free_cursor: AtomicUsize::new(0),
        }
    }
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
