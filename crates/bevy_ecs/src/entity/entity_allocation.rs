use bevy_platform_support::{
    prelude::Vec,
    sync::{
        atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering},
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
    unsafe fn set(&self, index: u32, entity: Entity, index_of_self: u32) -> Slot {
        let head = self.ptr().unwrap_or_else(|| self.init(index_of_self));
        let target = head.add(index as usize);

        // SAFETY: Ensured by caller.
        unsafe { core::ptr::replace(target, Slot::new(entity)) }
    }

    /// Initializes the chunk to be valid, returning the pointer.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    #[cold]
    unsafe fn init(&self, index: u32) -> *mut Slot {
        let cap = Self::capacity_of_chunk(index);
        let mut buff = ManuallyDrop::new(Vec::new());
        buff.reserve_exact(cap as usize);
        let ptr = buff.as_mut_ptr();
        self.first.store(ptr, Ordering::Relaxed);
        ptr
    }

    fn try_dealloc(&self, index: u32) {
        if let Some(to_drop) = self.ptr() {
            let cap = Self::capacity_of_chunk(index) as usize;
            // SAFETY: This was created in [`Self::init`] from a standard Vec.
            unsafe {
                Vec::from_raw_parts(to_drop, cap, cap);
            }
        }
    }

    /// Returns [`Self::first`] if it is valid.
    #[inline]
    fn ptr(&self) -> Option<*mut Slot> {
        let ptr = self.first.load(Ordering::Relaxed);
        (!ptr.is_null()).then_some(ptr)
    }

    fn new() -> Self {
        Self {
            first: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

/// This is the shared data for the owned list.
/// It is the source of truth.
///
/// Conceptually, this is like a `Vec<Entity>` divided into two slices.
/// The first slice stores the [`Entity`]s that are spawned but have no components.
/// The second slice are those that are free (pending reuse).
///
/// The empty slice starts at index zero.
/// The free slice starts at index [`Self::free_cursor`].
/// The combined length of the two slices is [`Self::len`].
struct OwnedBuffer {
    /// Each chunk has a length the power of 2.
    /// We store chunks in smallest to biggest order.
    chunks: [Chunk; Chunk::NUM_CHUNKS as usize],
    /// This is the total length of the whole buffer.
    ///
    /// # Safety
    ///
    /// This must only be changed exclusively. (By [`Owned`])
    len: AtomicU32,
    /// This is the index in this buffer of the first [`Slot`] that is free (pending reuse).
    /// If this index is out of bounds (at all) nothing is free. Values out of bounds have no real meaning.
    free_cursor: AtomicUsize,
}

impl OwnedBuffer {
    fn new() -> Self {
        let base = [(); Chunk::NUM_CHUNKS as usize];
        let chunks = base.map(|()| Chunk::new());
        Self {
            chunks,
            len: AtomicU32::new(0),
            free_cursor: AtomicUsize::new(0),
        }
    }

    /// Gets the [`Entity`] at this idnex.
    ///
    /// # Safety
    ///
    /// The index must have have been [`Owned::set`] before.
    unsafe fn get(&self, index: u32) -> Entity {
        let (chunk_idnex, index_in_chunk) = Chunk::get_indices(index);
        // SAFETY: `chunk_idnex` is correct. The chunk is valid and the slot is init because the index is inbounds.
        unsafe {
            self.chunks
                .get_unchecked(chunk_idnex as usize)
                .get(index_in_chunk)
        }
    }
}

impl Drop for OwnedBuffer {
    fn drop(&mut self) {
        for index in 0..Chunk::NUM_CHUNKS {
            self.chunks[index as usize].try_dealloc(index);
        }
    }
}

/// This is the owned list.
/// It contains all entities owned by an entity allocator.
/// This includes empty archetype entities and entities pending reuse.
pub struct Owned {
    /// The buffer itself
    buffer: Arc<OwnedBuffer>,
    /// This mirrors the [`OwnedBuffer::len`] in [`Self::buffer`].
    /// Since this is the only object that can change this value,
    /// we keep a copy of it here and only write through to the [`OwnedBuffer::len`] as it is changed.
    len: u32,
}

impl Owned {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(OwnedBuffer::new()),
            len: 0,
        }
    }

    /// Sets the [`Entity`] at this idnex.
    #[inline]
    fn set(&mut self, entity: Entity, index: u32) -> Slot {
        let (chunk_idnex, index_in_chunk) = Chunk::get_indices(index);
        // SAFETY: `chunk_idnex` is correct. The chunk is valid and the slot is init because the index is inbounds.
        // And this can't be called concurrently since we have `&mut`
        unsafe {
            self.buffer.chunks.get_unchecked(chunk_idnex as usize).set(
                index_in_chunk,
                entity,
                chunk_idnex,
            )
        }
    }

    /// If possible, spawns an empty by reusing a freed one.
    fn spawn_empty_in_buffer(&self) -> Option<Entity> {
        let index = self.buffer.free_cursor.fetch_add(1, Ordering::Relaxed);

        // SAFETY: We check that it is in bounds.
        (index < self.len as usize).then(|| unsafe {
            // We can safely cast to a `u32` since for it to overflow, there must already be too many entities.
            self.buffer.get(index as u32)
        })
    }

    /// Reserves an [`Entity`] without moving it into the empty archetype.
    fn alloc_non_empty_from_freed(&mut self) -> Option<Entity> {
        if self.len == 0 {
            return None;
        }

        // Make sure nobody tries to take the slot we're trying to pop.
        self.len -= 1; // len is now the index we're trying to pop.
        self.buffer.len.store(self.len, Ordering::Relaxed);

        let free_cursor = self.buffer.free_cursor.load(Ordering::Relaxed);
        if free_cursor <= self.len as usize {
            // The thing we want to pop is in fact pending reuse

            // SAFETY: This is init since it was just within `len`.
            Some(unsafe { self.buffer.get(self.len) })
        } else {
            // The thing we wanted to pop is not pending reusse but is a normal empty entity.

            // Bring this back up to source of truth
            self.len += 1;
            // We don't need to write back this source of truth to [`OwnedBuffer::len`] since the `free_cursor` is already out of bounds, so it doesn't matter.
            None
        }
    }

    /// Frees the [`Entity`] so it can be reused.
    /// Returns its index in the list.
    /// This will be it's archetype row if [`spawn_empty`](Self::spawn_empty) reuses it.
    ///
    /// # Safety
    ///
    /// The entity must not be in the empty archetype in this buffer.
    pub unsafe fn free_non_empty_buffer(&mut self, entity: Entity) -> u32 {
        let index = self.len;
        self.set(entity, index);
        self.len += 1;

        // Prevent any remote reservations from accessing the buffer for a bit
        let free_cursor = self
            .buffer
            .free_cursor
            .swap(u32::MAX as usize, Ordering::Relaxed);

        // Set the len now that everything is valid.
        self.buffer.len.store(self.len, Ordering::Relaxed);

        // If this changes the free cursor, this must be the only free entity, so it should be set to this index.
        self.buffer
            .free_cursor
            .store(free_cursor.min(index as usize), Ordering::Relaxed);

        index
    }

    /// The entity at this index in the [`OwnedBuffer`] will no longer be in the empty archetype.
    /// This removes it from the buffer.
    ///
    /// This returns the [`Entity`] that is now at `index`.
    ///
    /// # Safety
    ///
    /// The `index` must be valid and point to a empty archetype valid entity.
    pub unsafe fn make_non_empty(&mut self, index: u32) -> Option<Entity> {
        let free_cursor = self.buffer.free_cursor.load(Ordering::Relaxed);

        if free_cursor >= self.len as usize {
            // Nothing is free so we can just swap remove normally.

            self.len -= 1;

            if index == self.len {
                // This is at the end of the list, so we pop instead of swap remove.
                return None;
            }

            let to_swap = self.buffer.get(self.len);
            self.set(to_swap, index);

            // We don't need to write back to [`OwnedBuffer::len`] since the `free_cursor` is already out of bounds, so it doesn't matter.

            Some(to_swap)
        } else {
            // There is something free,

            // so first we need to fill in the `index` with the last empty archetype.
            let last_empty = free_cursor as u32 - 1;
            let fill_in = self.buffer.get(last_empty);
            self.set(fill_in, index);

            // then, we need to fill in the last empty archetype with the last free entity
            self.len -= 1;
            self.buffer.len.store(self.len, Ordering::Relaxed);
            let last_free = self.buffer.get(self.len);
            self.set(last_free, last_empty);

            // and finally, we need to decrement the free_cursor.
            let cmp = self.buffer.free_cursor.compare_exchange(
                free_cursor,
                // The last slot in the old empty list is now free
                last_empty as usize,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            match cmp {
                Ok(_) => Some(fill_in),

                // TODO: hint that this is unlikely
                Err(mut new_free_cursor) => {
                    // The free cursor was pulled out from under us.
                    // Now the buffer looks like | empty archetype | free entity (`last_free`) | just reserved empty archetype | rest of the free entities
                    let mut last_empty = last_empty;
                    loop {
                        let new_last_empty = (new_free_cursor - 1).min(self.len as usize) as u32;
                        let new_last_empty_entity = self.buffer.get(new_last_empty);
                        self.set(new_last_empty_entity, last_empty);
                        self.set(last_free, new_last_empty);
                        let cmp = self.buffer.free_cursor.compare_exchange(
                            new_free_cursor,
                            // The last slot in the old empty list is now free
                            new_last_empty as usize,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        );
                        match cmp {
                            Ok(_) => break Some(fill_in),
                            Err(new) => {
                                new_free_cursor = new;
                                last_empty = new_last_empty;
                            }
                        }
                    }
                }
            }
        }
    }

    /// This makes this [`Entity`] in the buffer's empty archetype, returning it's index in the buffer.
    ///
    /// # Safety
    ///
    /// `entity` must not be already in the buffer.
    pub unsafe fn make_empty(&mut self, entity: Entity) -> u32 {
        let index = self.buffer.free_cursor.fetch_add(1, Ordering::Relaxed);

        if index < self.len as usize {
            // We can push to the end of the empty archetype list, displacing a free entity.

            // SAFETY: The idnex is in bounds
            let free = unsafe { self.set(entity, index as u32).assume_init_read() };
            // SAFETY: `free` is no longer in the buffer
            unsafe {
                self.free_non_empty_buffer(free);
            }
            index as u32
        } else {
            // There are no free entities, so we can just push

            // do the push
            let index = self.len;
            self.set(entity, index);

            // This can not over take the free cursor since we're adding 1 to both.
            self.len += 1;
            self.buffer.len.store(self.len, Ordering::Relaxed);

            index
        }
    }
}

/// This is a more limmited version of [`Owned`].
/// There can be many of these but only one [`Owned`].
/// This is what allows for remote spawning.
pub struct RemoteOwned {
    buffer: Arc<OwnedBuffer>,
}

impl RemoteOwned {
    /// Constructs a new [`RemoteOwned`] from a source [`Owned`].
    pub fn new(source: &Owned) -> Self {
        Self {
            buffer: source.buffer.clone(),
        }
    }

    /// If possible, reuses a freed entity and spawns it as an empty entity.
    fn try_spawn_empty_from_free(&self) -> Option<Entity> {
        let index = self.buffer.free_cursor.fetch_add(1, Ordering::Relaxed);
        // We get length after the index since we need this to be more recent than the free cursor.
        // [`Owned`] handles unexpected changes to the free cursor.
        // We just need to make *certain* this is in bounds.
        let len = self.buffer.len.load(Ordering::Relaxed);

        // SAFETY: We check that it is in bounds.
        (index < len as usize).then(|| unsafe {
            // We can safely cast to a `u32` since for it to overflow, there must already be too many entities.
            self.buffer.get(index as u32)
        })
    }
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
