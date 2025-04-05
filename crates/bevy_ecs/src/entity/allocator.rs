use bevy_platform_support::{
    prelude::Vec,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
};
use core::mem::ManuallyDrop;
use log::warn;

use super::{Entity, EntitySetIterator};

/// This is the item we store in the free list.
struct Slot {
    #[cfg(not(target_has_atomic = "64"))]
    entity_index: AtomicU32,
    #[cfg(not(target_has_atomic = "64"))]
    entity_generation: AtomicU32,
    #[cfg(target_has_atomic = "64")]
    inner_entity: AtomicU64,
}

impl Slot {
    /// Produces a meaningless an empty value. This produces a valid but incorrect `Entity`.
    fn empty() -> Self {
        let source = Entity::PLACEHOLDER;
        #[cfg(not(target_has_atomic = "64"))]
        return Self {
            entity_index: AtomicU32::new(source.index()),
            entity_generation: AtomicU32::new(source.generation()),
        };
        #[cfg(target_has_atomic = "64")]
        return Self {
            inner_entity: AtomicU64::new(source.to_bits()),
        };
    }

    // TODO: could maybe make this `&mut`??
    #[inline]
    fn set_entity(&self, entity: Entity) {
        #[cfg(not(target_has_atomic = "64"))]
        self.entity_generation
            .store(entity.generation(), Ordering::Relaxed);
        #[cfg(not(target_has_atomic = "64"))]
        self.entity_index.store(entity.index(), Ordering::Relaxed);
        #[cfg(target_has_atomic = "64")]
        self.inner_entity.store(entity.to_bits(), Ordering::Relaxed);
    }

    /// Gets the stored entity. The result be [`Entity::PLACEHOLDER`] unless [`set_entity`](Self::set_entity) has been called.
    #[inline]
    fn get_entity(&self) -> Entity {
        #[cfg(not(target_has_atomic = "64"))]
        return Entity {
            index: self.entity_index.load(Ordering::Relaxed),
            // SAFETY: This is not 0 since it was from an entity's generation.
            generation: unsafe {
                core::num::NonZero::new_unchecked(self.entity_generation.load(Ordering::Relaxed))
            },
        };
        #[cfg(target_has_atomic = "64")]
        // SAFETY: This is always sourced from a proper entity.
        return unsafe { Entity::from_bits_unchecked(self.inner_entity.load(Ordering::Relaxed)) };
    }
}

/// Each chunk stores a buffer of [`Slot`]s at a fixed capacity.
struct Chunk {
    /// Points to the first slot. If this is null, we need to allocate it.
    first: AtomicPtr<Slot>,
}

impl Chunk {
    /// Constructs a null [`Chunk`].
    const fn new() -> Self {
        Self {
            first: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Gets the entity at the index within this chunk.
    ///
    /// # Safety
    ///
    /// [`Self::set`] must have been called on this index before, ensuring it is in bounds and the chunk is initialized.
    #[inline]
    unsafe fn get(&self, index: u32) -> Entity {
        // Relaxed is fine since caller ensures we are iitialized already.
        // In order for the caller to guarantee that, they must have an ordering that orders this get after the required `set`.
        let head = self.first.load(Ordering::Relaxed);
        // SAFETY: caller ensures we are in bounds and init (because `set` must be in bounds)
        let target = unsafe { &*head.add(index as usize) };

        target.get_entity()
    }

    /// Gets a slice of indices.
    ///
    /// # Safety
    ///
    /// [`Self::set`] must have been called on these indices before, ensuring it is in bounds and the chunk is initialized.
    #[inline]
    unsafe fn get_slice(&self, index: u32, ideal_len: u32, chunk_capacity: u32) -> &[Slot] {
        let after_index_slice_len = chunk_capacity - index;
        let len = after_index_slice_len.min(ideal_len) as usize;

        // Relaxed is fine since caller ensures we are iitialized already.
        // In order for the caller to guarantee that, they must have an ordering that orders this get after the required `set`.
        let head = self.first.load(Ordering::Relaxed);

        // SAFETY: Caller ensures we are init, so the chunk was allocated via a `Vec` and the index is within the capacity.
        unsafe { core::slice::from_raw_parts(head, len) }
    }

    /// Sets this entity at this index.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    /// Index must be in bounds.
    /// Access does not conflict with another [`Self::get`].
    #[inline]
    unsafe fn set(&self, index: u32, entity: Entity, chunk_capacity: u32) {
        // Relaxed is fine here since this is not called concurrently and does not conflict with a `get`.
        let ptr = self.first.load(Ordering::Relaxed);
        let head = if ptr.is_null() {
            self.init(chunk_capacity)
        } else {
            ptr
        };

        // SAFETY: caller ensures it is in bounds and we are not fighting with other `set` calls or `get` calls.
        // A race condition is therefore impossible.
        let target = unsafe { &*head.add(index as usize) };

        target.set_entity(entity);
    }

    /// Initializes the chunk to be valid, returning the pointer.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    #[cold]
    unsafe fn init(&self, chunk_capacity: u32) -> *mut Slot {
        let mut buff = ManuallyDrop::new(Vec::new());
        buff.reserve_exact(chunk_capacity as usize);
        buff.resize_with(chunk_capacity as usize, Slot::empty);
        let ptr = buff.as_mut_ptr();
        // Relaxed is fine here since this is not called concurrently.
        self.first.store(ptr, Ordering::Relaxed);
        ptr
    }

    /// Frees memory
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    /// `chunk_capacity` must be the same as it was initialized with.
    unsafe fn dealloc(&self, chunk_capacity: u32) {
        // Relaxed is fine here since this is not called concurrently.
        let to_drop = self.first.load(Ordering::Relaxed);
        if !to_drop.is_null() {
            // SAFETY: This was created in [`Self::init`] from a standard Vec.
            unsafe {
                Vec::from_raw_parts(to_drop, chunk_capacity as usize, chunk_capacity as usize);
            }
        }
    }
}

/// This is a buffer that has been split into chunks, so that each chunk is pinned in memory.
/// Conceptually, each chunk is put end-to-end to form the buffer.
/// This will expand in capacity as needed, but a separate system must track the length of the list in the buffer.
struct FreeBuffer([Chunk; Self::NUM_CHUNKS as usize]);

impl FreeBuffer {
    const NUM_CHUNKS: u32 = 24;
    const NUM_SKIPPED: u32 = u32::BITS - Self::NUM_CHUNKS;

    /// Constructs a empty [`FreeBuffer`].
    const fn new() -> Self {
        Self([const { Chunk::new() }; Self::NUM_CHUNKS as usize])
    }

    /// Computes the capacity of the chunk at this index within [`Self::NUM_CHUNKS`].
    /// The first 2 have length 512 (2^9) and the last has length (2^31)
    #[inline]
    fn capacity_of_chunk(chunk_index: u32) -> u32 {
        // We do this because we're skipping the first `NUM_SKIPPED` powers, so we need to make up for them by doubling the first index.
        // This is why the first 2 indices both have a capacity of 256.
        let corrected = chunk_index.max(1);
        // We add NUM_SKIPPED because the total capacity should be as if [`Self::NUM_CHUNKS`] were 32.
        // This skips the first NUM_SKIPPED powers.
        let corrected = corrected + Self::NUM_SKIPPED;
        // This bit shift is just 2^corrected.
        1 << corrected
    }

    /// For this index in the whole buffer, returns the index of the [`Chunk`], the index within that chunk, and the capacity of that chunk.
    #[inline]
    fn index_info(full_index: u32) -> (u32, u32, u32) {
        // We do a `saturating_sub` because we skip the first `NUM_SKIPPED` powers to make space for the first chunk's entity count.
        // The -1 is because this is the number of chunks, but we want the index in the end.
        // We store chunks in smallest to biggest order, so we need to reverse it.
        let chunk_index = (Self::NUM_CHUNKS - 1).saturating_sub(full_index.leading_zeros());
        let chunk_capacity = Self::capacity_of_chunk(chunk_index);
        // We only need to cut off this particular bit.
        // The capacity is only one bit, and if other bits needed to be dropped, `leading` would have been greater
        let index_in_chunk = full_index & !chunk_capacity;

        (chunk_index, index_in_chunk, chunk_capacity)
    }

    /// For this index in the whole buffer, returns the [`Chunk`], the index within that chunk, and the capacity of that chunk.
    #[inline]
    fn index_in_chunk(&self, full_index: u32) -> (&Chunk, u32, u32) {
        let (chunk_index, index_in_chunk, chunk_capacity) = Self::index_info(full_index);
        // SAFETY: The chunk index is correct
        let chunk = unsafe { self.0.get_unchecked(chunk_index as usize) };
        (chunk, index_in_chunk, chunk_capacity)
    }

    /// Gets the entity at an index.
    ///
    /// # Safety
    ///
    /// [`set`](Self::set) must have been called on this index to initialize the its memory.
    unsafe fn get(&self, full_index: u32) -> Entity {
        let (chunk, index, _) = self.index_in_chunk(full_index);
        // SAFETY: Caller ensures this index was set
        unsafe { chunk.get(index) }
    }

    /// Sets an entity at an index.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    /// Access does not conflict with another [`Self::get`].
    #[inline]
    unsafe fn set(&self, full_index: u32, entity: Entity) {
        let (chunk, index, chunk_capacity) = self.index_in_chunk(full_index);
        // SAFETY: Ensured by caller and that the index is correct.
        unsafe { chunk.set(index, entity, chunk_capacity) }
    }

    /// Iterates the entities in these indices.
    ///
    /// # Safety
    ///
    /// [`Self::set`] must have been called on these indices before to initialize memory.
    #[inline]
    unsafe fn iter(&self, indices: core::ops::RangeInclusive<u32>) -> FreeBufferIterator {
        FreeBufferIterator {
            buffer: self,
            indices,
            current: [].iter(),
        }
    }
}

impl Drop for FreeBuffer {
    fn drop(&mut self) {
        for index in 0..Self::NUM_CHUNKS {
            let capacity = Self::capacity_of_chunk(index);
            // SAFETY: we have `&mut` and the capacity is correct.
            unsafe { self.0[index as usize].dealloc(capacity) };
        }
    }
}

/// An iterator over a [`FreeBuffer`].
///
/// # Safety
///
/// [`FreeBuffer::set`] must have been called on these indices before to initialize memory.
struct FreeBufferIterator<'a> {
    buffer: &'a FreeBuffer,
    indices: core::ops::RangeInclusive<u32>,
    current: core::slice::Iter<'a, Slot>,
}

impl<'a> Iterator for FreeBufferIterator<'a> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(found) = self.current.next() {
            return Some(found.get_entity());
        }

        let next_index = self.indices.next()?;
        let (chunk, index, chunk_capacity) = self.buffer.index_in_chunk(next_index);

        // SAFETY: Assured by constructor
        let slice = unsafe { chunk.get_slice(index, self.len() as u32 + 1, chunk_capacity) };
        self.indices = (*self.indices.start() + slice.len() as u32 - 1)..=(*self.indices.end());

        self.current = slice.iter();
        Some(self.current.next()?.get_entity())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.indices.end().saturating_sub(*self.indices.start()) as usize;
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for FreeBufferIterator<'a> {}
impl<'a> core::iter::FusedIterator for FreeBufferIterator<'a> {}

/// This stores two things: the length of the buffer (which can be negative) and a generation value to track *any* change to the length.
///
/// The upper 48 bits store an unsigned integer of the length, and the lower 16 bits store the generation value.
/// By keeping the length in the upper bits, we can add anything to them without it affecting the generation bits.
/// See [`Self::encode_pop`] for how this is done.
/// To prevent the generation from ever overflowing into the length,
/// we follow up each operation with a bit-wise `&` to turn of the most significant generation bit, preventing overflow.
///
/// Finally, to get the signed length from the unsigned 48 bit value, we simply set `u48::MAX - u32::MAX` equal to 0.
/// This is fine since for the length to go over `u32::MAX`, the entity index would first need to be exhausted, ausing a "too many entities" panic.
/// In theory, the length should not drop below `-u32::MAX` since doing so would cause a "too many entities" panic.
/// However, using 48 bits provides a buffer here and allows extra flags like [`Self::DISABLING_BIT`].
struct FreeCount(AtomicU64);

impl FreeCount {
    /// The bit of the u64 with the highest bit of the u16 generation.
    const HIGHEST_GENERATION_BIT: u64 = 1 << 15;
    /// The u48 encoded length considers this value to be 0. Lower values are considered negative.
    const FALSE_ZERO: u64 = ((1 << 48) - 1) - ((1 << 32) - 1);
    /// This bit is off only when the length has been entirely disabled.
    const DISABLING_BIT: u64 = 1 << 63;

    /// Constructs a length of 0.
    const fn new_zero_len() -> Self {
        Self(AtomicU64::new(Self::FALSE_ZERO << 16))
    }

    /// Gets the current state of the buffer.
    #[inline]
    fn state(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    /// Gets the length from a given state. Returns 0 if the length is negative or zero.
    #[inline]
    fn len_from_state(state: u64) -> u32 {
        let encoded_length = state >> 16;
        // Since `FALSE_ZERO` only leaves 32 bits of a u48 above it, the len must fit within 32 bits.
        encoded_length.saturating_sub(Self::FALSE_ZERO) as u32
    }

    /// Returns true if the length is currently disabled.
    #[inline]
    fn is_state_disabled(state: u64) -> bool {
        (state & Self::DISABLING_BIT) == 0
    }

    /// Gets the length. Returns 0 if the length is negative or zero.
    #[inline]
    fn len(&self) -> u32 {
        Self::len_from_state(self.state())
    }

    /// Returns the number to add for subtracting this `num`.
    #[inline]
    fn encode_pop(num: u32) -> u64 {
        let encoded_diff = (num as u64) << 16;
        // In modular arithmetic, this is equivalent to the requested subtraction.
        let to_add = u64::MAX - encoded_diff;

        // add one to the generation.
        // Note that if `num` is 0, this will wrap `to_add` to 0,
        // which is correct since we aren't adding anything.
        // Since we aren't really popping anything either,
        // it is perfectly fine to not add to the generation too.
        to_add.wrapping_add(1)
    }

    /// Subtracts `num` from the length, returning the new state.
    #[inline]
    fn pop_from_state(mut state: u64, num: u32) -> u64 {
        state += Self::encode_pop(num);
        // prevent generation overflow
        state &= !Self::HIGHEST_GENERATION_BIT;
        state
    }

    /// Subtracts `num` from the length, returning the previous state.
    #[inline]
    fn pop_for_state(&self, num: u32) -> u64 {
        let state = self.0.fetch_add(Self::encode_pop(num), Ordering::AcqRel);
        // This can be relaxed since it only affects the one bit,
        // and 2^15 operations would need to happen with this never being called for an overflow to occor.
        self.0
            .fetch_and(!Self::HIGHEST_GENERATION_BIT, Ordering::Relaxed);
        state
    }

    /// Subtracts `num` from the length, returning the previous length.
    #[inline]
    fn pop_for_len(&self, num: u32) -> u32 {
        Self::len_from_state(self.pop_for_state(num))
    }

    /// Disables the length completely, returning the previous state.
    #[inline]
    fn disable_len_for_state(&self) -> u64 {
        // We don't care about the generation here since the length is invalid anyway.
        // In order to reset length, `set_len` must be called, which handles the generation.
        self.0.fetch_add(!Self::DISABLING_BIT, Ordering::AcqRel)
    }

    /// Sets the length explicitly.
    #[inline]
    fn set_len(&self, len: u32, recent_state: u64) {
        let encoded_length = (len as u64 + Self::FALSE_ZERO) << 16;
        let recent_generation = recent_state & (u16::MAX as u64 & !Self::HIGHEST_GENERATION_BIT);

        // This effectively adds a 2^14 to the generation, so for recent `recent_state` values, this is very safe.
        // It is worth mentioning that doing this back to back will negate it, but in theory, we don't even need this at all.
        // If an uneven number of free and alloc calls are made, the length will be different, so the generation is a moot point.
        // If they are even, then at least one alloc call has been made, which would have incremented the generation in `recent_state`.
        // So in all cases, the state is sufficiently changed such that `try_set_state` will fail when needed.
        let far_generation = recent_generation ^ (1 << 14);

        let fully_encoded = encoded_length | far_generation;
        self.0.store(fully_encoded, Ordering::Release);
    }

    /// Attempts to update the state, returning the new state if it fails.
    #[inline]
    fn try_set_state(&self, expected_current_state: u64, target_state: u64) -> Result<(), u64> {
        self.0
            .compare_exchange(
                expected_current_state,
                target_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
    }
}

/// This is conceptually like a `Vec<Entity>` that stores entities pending reuse.
struct FreeList {
    /// The actual buffer of [`Slot`]s.
    /// Conceptually, this is like the `RawVec` for this `Vec`.
    buffer: FreeBuffer,
    /// The length of the free buffer
    len: FreeCount,
}

impl FreeList {
    /// Constructs a empty [`FreeList`].
    fn new() -> Self {
        Self {
            buffer: FreeBuffer::new(),
            len: FreeCount::new_zero_len(),
        }
    }

    /// Gets the number of free entities.
    ///
    /// # Safety
    ///
    /// For this to be accurate, this must not be called during a [`Self::free`].
    #[inline]
    unsafe fn num_free(&self) -> u32 {
        self.len.len()
    }

    /// Frees the `entity` allowing it to be reused.
    ///
    /// # Safety
    ///
    /// This must not conflict with any other [`Self::free`] or [`Self::alloc`] calls.
    #[inline]
    unsafe fn free(&self, entity: Entity) {
        // Disable remote allocation.
        let state = self.len.disable_len_for_state();

        // Push onto the buffer
        let len = FreeCount::len_from_state(state);
        // SAFETY: Caller ensures this does not conflict with `free` or `alloc` calls,
        // and we just disabled remote allocation.
        unsafe {
            self.buffer.set(len, entity);
        }

        // Update length
        let new_len = len + 1;
        self.len.set_len(new_len, state);
    }

    /// Allocates an [`Entity`] from the free list if one is available.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`Self::free`] calls.
    #[inline]
    unsafe fn alloc(&self) -> Option<Entity> {
        // SAFETY: This will get a valid index because there is no way for `free` to be done at the same time.
        let len = self.len.pop_for_len(1);
        let index = len.checked_sub(1)?;

        // SAFETY: This was less then `len`, so it must have been `set` via `free` before.
        Some(unsafe { self.buffer.get(index) })
    }

    /// Allocates an as many [`Entity`]s from the free list as are available, up to `count`.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`Self::free`] calls for the duration of the returned iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> FreeBufferIterator {
        // SAFETY: This will get a valid index because there is no way for `free` to be done at the same time.
        let len = self.len.pop_for_len(count);
        let index = len.saturating_sub(count);

        let indices = if index < len {
            let end = len - 1;
            index..=end
        } else {
            #[expect(
                clippy::reversed_empty_ranges,
                reason = "We intentionally need an empty range"
            )]
            {
                1..=0
            }
        };

        // SAFETY: The indices are all less then the length.
        unsafe { self.buffer.iter(indices) }
    }

    /// Allocates an [`Entity`] from the free list if one is available and it is safe to do so.
    #[inline]
    fn remote_alloc(&self) -> Option<Entity> {
        // The goal is the same as `alloc`, so what's the difference?
        // `alloc` knows `free` is not being called, but this does not.
        // What if we `len.fetch_sub(1)` but then `free` overwrites the entity before we could read it?
        // That would mean we would leak an entity and give another entity out twice.
        // We get around this by only updating `len` after the read is complete.
        // But that means something else could be trying to allocate the same index!
        // So we need a `len.compare_exchange` loop to ensure the index is unique.
        // Because we keep a generation value in the `FreeCount`, if any of these things happen, we simply try again.

        let mut state = self.len.state();
        loop {
            // The state is only disabled when freeing.
            // If a free is happening, we need to wait for the new entity to be ready on the free buffer.
            // Then, we can allocate it.
            if FreeCount::is_state_disabled(state) {
                core::hint::spin_loop();
                state = self.len.state();
                continue;
            }

            let len = FreeCount::len_from_state(state);
            let index = len.checked_sub(1)?;

            // SAFETY: This was less then `len`, so it must have been `set` via `free` before.
            let entity = unsafe { self.buffer.get(index) };

            let ideal_state = FreeCount::pop_from_state(state, 1);
            match self.len.try_set_state(state, ideal_state) {
                Ok(_) => return Some(entity),
                Err(new_state) => state = new_state,
            }
        }
    }
}

/// This stores allocation data shared by all entity allocators.
struct SharedAllocator {
    /// The entities pending reuse
    free: FreeList,
    /// The next value of [`Entity::index`] to give out if needed.
    next_entity_index: AtomicU32,
    /// If true, the [`Self::next_entity_index`] has been incremented before,
    /// so if it hits or passes zero again, an overflow has occored.
    entity_index_given: AtomicBool,
    /// Tracks whether or not the primary [`Allocator`] has been closed or not.
    is_closed: AtomicBool,
}

impl SharedAllocator {
    /// Constructs a [`SharedAllocator`]
    fn new() -> Self {
        Self {
            free: FreeList::new(),
            next_entity_index: AtomicU32::new(0),
            entity_index_given: AtomicBool::new(false),
            is_closed: AtomicBool::new(false),
        }
    }

    /// The total number of indices given out.
    #[inline]
    fn total_entity_indices(&self) -> u64 {
        let next = self.next_entity_index.load(Ordering::Relaxed);
        if next == 0 {
            if self.entity_index_given.load(Ordering::Relaxed) {
                // every index has been given
                u32::MAX as u64 + 1
            } else {
                // no index has been given
                0
            }
        } else {
            next as u64
        }
    }

    /// Call this when the entity index is suspected to have overflown.
    /// Panic if the overflow did happen.
    #[cold]
    fn check_overflow(&self) {
        if self.entity_index_given.swap(true, Ordering::AcqRel) {
            panic!("too many entities")
        }
    }

    /// Allocates an [`Entity`] with a brand new index.
    #[inline]
    fn alloc_new_index(&self) -> Entity {
        let index = self.next_entity_index.fetch_add(1, Ordering::Relaxed);
        if index == 0 {
            self.check_overflow();
        }
        Entity::from_raw(index)
    }

    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeList::free`] calls.
    #[inline]
    unsafe fn alloc(&self) -> Entity {
        // SAFETY: assured by caller
        unsafe { self.free.alloc() }.unwrap_or_else(|| self.alloc_new_index())
    }

    /// Allocates a `count` [`Entity`]s, reusing freed indices if they exist.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeList::free`] calls for the duration of the iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> AllocEntitiesIterator {
        let reused = self.free.alloc_many(count);
        let missing = count - reused.len() as u32;
        let start_new = self.next_entity_index.fetch_add(missing, Ordering::Relaxed);

        let new_next_entity_index = start_new + missing;
        if new_next_entity_index < missing || start_new == 0 {
            self.check_overflow();
        }

        let new = start_new..=(start_new + missing - 1);
        AllocEntitiesIterator { new, reused }
    }

    /// Allocates a new [`Entity`].
    /// This will only try to reuse a freed index if it is safe to do so.
    #[inline]
    fn remote_alloc(&self) -> Entity {
        self.free
            .remote_alloc()
            .unwrap_or_else(|| self.alloc_new_index())
    }

    /// Marks the allocator as closed, but it will still function normally.
    fn close(&self) {
        self.is_closed.store(true, Ordering::Release);
    }

    /// Returns true if [`Self::close`] has been called.
    fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }
}

/// This keeps track of freed entities and allows the allocation of new ones.
pub struct Allocator {
    shared: Arc<SharedAllocator>,
}

impl Allocator {
    /// Constructs a new [`Allocator`]
    pub fn new() -> Self {
        Self {
            shared: Arc::new(SharedAllocator::new()),
        }
    }

    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    #[inline]
    pub fn alloc(&self) -> Entity {
        // SAFETY: violating safety requires a `&mut self` to exist, but rust does not allow that.
        unsafe { self.shared.alloc() }
    }

    /// The total number of indices given out.
    #[inline]
    pub fn total_entity_indices(&self) -> u64 {
        self.shared.total_entity_indices()
    }

    /// The number of free entities.
    #[inline]
    pub fn num_free(&self) -> u32 {
        // SAFETY: `free` is not being called since it takes `&mut self`.
        unsafe { self.shared.free.num_free() }
    }

    /// Returns whether or not the index is valid in this allocator.
    #[inline]
    pub fn is_valid_index(&self, index: u32) -> bool {
        (index as u64) < self.total_entity_indices()
    }

    /// Frees the entity allowing it to be reused.
    #[inline]
    pub fn free(&mut self, entity: Entity) {
        // SAFETY: We have `&mut self`.
        unsafe {
            self.shared.free.free(entity);
        }
    }

    /// Allocates `count` entities in an iterator.
    #[inline]
    pub fn alloc_many(&self, count: u32) -> AllocEntitiesIterator {
        // SAFETY: `free` takes `&mut self`, but this lifetime is captured by the iterator.
        unsafe { self.shared.alloc_many(count) }
    }

    /// Allocates `count` entities in an iterator.
    ///
    /// # Safety
    ///
    /// Caller ensures [`Self::free`] is not called for the duration of the iterator.
    /// Caller ensures this allocator is not dropped for the lifetime of the iterator.
    #[inline]
    pub unsafe fn alloc_many_unsafe(&self, count: u32) -> AllocEntitiesIterator<'static> {
        // SAFETY: Caller ensures this instance is valid until the returned value is dropped.
        let this: &'static Self = unsafe { &*core::ptr::from_ref(self) };
        // SAFETY:  Caller ensures free is not called.
        unsafe { this.shared.alloc_many(count) }
    }
}

impl Drop for Allocator {
    fn drop(&mut self) {
        self.shared.close();
    }
}

impl core::fmt::Debug for Allocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(core::any::type_name::<Self>())
            .field("total_indices", &self.total_entity_indices())
            .field("total_free", &self.num_free())
            .finish()
    }
}

/// An [`Iterator`] returning a sequence of [`Entity`] values from an [`Allocator`].
///
/// **NOTE:** Dropping will leak the remaining entities!
pub struct AllocEntitiesIterator<'a> {
    new: core::ops::RangeInclusive<u32>,
    reused: FreeBufferIterator<'a>,
}

impl<'a> Iterator for AllocEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.reused
            .next()
            .or_else(|| self.new.next().map(Entity::from_raw))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.reused.len() + self.new.end().saturating_sub(*self.new.end()) as usize;
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for AllocEntitiesIterator<'a> {}
impl<'a> core::iter::FusedIterator for AllocEntitiesIterator<'a> {}

// SAFETY: Newly reserved entity values are unique.
unsafe impl EntitySetIterator for AllocEntitiesIterator<'_> {}

impl Drop for AllocEntitiesIterator<'_> {
    fn drop(&mut self) {
        let leaking = self.len();
        if leaking > 0 {
            warn!(
                "{} entities being leaked via unfinished `AllocEntitiesIterator`",
                leaking
            );
        }
    }
}

/// This is a stripped down version of [`Allocator`] that operates on fewer assumptions.
/// As a result, using this will be slower than [`Allocator`] but this offers additional freedoms.
#[derive(Clone)]
pub struct RemoteAllocator {
    shared: Arc<SharedAllocator>,
}

impl RemoteAllocator {
    /// Creates a new [`RemoteAllocator`] with the provided [`Allocator`] source.
    /// If the source is ever destroyed, [`Self::alloc`] will yield garbage values.
    /// Be sure to use [`Self::is_closed`] to determine if it is safe to use these entities.
    pub fn new(source: &Allocator) -> Self {
        Self {
            shared: source.shared.clone(),
        }
    }

    /// Allocates an entity remotely.
    /// This is not guaranteed to reuse a freed entity, even if one exists.
    ///
    /// This will return [`None`] if the source [`Allocator`] is destroyed.
    #[inline]
    pub fn alloc(&self) -> Entity {
        self.shared.remote_alloc()
    }

    /// Returns whether or not this [`RemoteAllocator`] is still connected to its source [`Allocator`].
    /// Note that this could close immediately after the function returns false, so be careful.
    pub fn is_closed(&self) -> bool {
        self.shared.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Ensure the total capacity of [`OwnedBuffer`] is `u32::MAX + 1`, since the max *index* of an [`Entity`] is `u32::MAX`.
    #[test]
    fn chunk_capacity_sums() {
        let total: u64 = (0..FreeBuffer::NUM_CHUNKS)
            .map(FreeBuffer::capacity_of_chunk)
            .map(|x| x as u64)
            .sum();
        let expected = u32::MAX as u64 + 1;
        assert_eq!(total, expected);
    }

    /// Ensure [`OwnedBuffer`] can be properly indexed
    #[test]
    fn chunk_indexing() {
        let to_test = vec![
            (0, (0, 0, 512)), // index 0 cap = 512
            (1, (0, 1, 512)),
            (256, (0, 256, 512)),
            (511, (0, 511, 512)),
            (512, (1, 0, 512)), // index 1 cap = 512
            (1023, (1, 511, 512)),
            (1024, (2, 0, 1024)), // index 2 cap = 1024
            (1025, (2, 1, 1024)),
            (2047, (2, 1023, 1024)),
            (2048, (3, 0, 2048)), // index 3 cap = 2048
            (4095, (3, 2047, 2048)),
            (4096, (4, 0, 4096)), // index 3 cap = 4096
        ];

        for (input, output) in to_test {
            assert_eq!(FreeBuffer::index_info(input), output);
        }
    }

    #[test]
    fn buffer_len_encoding() {
        let len = FreeCount::new_zero_len();
        assert_eq!(len.len(), 0);
        assert_eq!(len.pop_for_len(200), 0);
        len.set_len(5, 0);
        assert_eq!(len.pop_for_len(2), 5);
        assert_eq!(len.pop_for_len(2), 3);
        assert_eq!(len.pop_for_len(2), 1);
        assert_eq!(len.pop_for_len(2), 0);
    }
}
