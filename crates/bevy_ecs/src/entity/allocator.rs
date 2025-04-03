use bevy_platform_support::{
    prelude::Vec,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
        Arc, Weak,
    },
};
use core::mem::ManuallyDrop;
use log::warn;

use crate::query::DebugCheckedUnwrap;

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

    /// Gets the stored entity.
    ///
    /// # Safety
    ///
    /// This slot *must* have been [`set_entity`](Self::set_entity) before this.
    /// Otherwise, the entity may be invalid or meaningless.
    #[inline]
    unsafe fn get_entity(&self) -> Entity {
        #[cfg(not(target_has_atomic = "64"))]
        return Entity {
            index: self.entity_index.load(Ordering::Relaxed),
            // SAFETY: This is not 0 since it was from an entity's generation.
            generation: unsafe {
                core::num::NonZero::new_unchecked(self.entity_generation.load(Ordering::Relaxed))
            },
        };
        #[cfg(target_has_atomic = "64")]
        return Entity::from_bits(self.inner_entity.load(Ordering::Relaxed));
    }
}

/// Each chunk stores a buffer of [`Slot`]s at a fixed capacity.
struct Chunk {
    /// Points to the first slot. If this is null, we need to allocate it.
    first: AtomicPtr<Slot>,
}

impl Chunk {
    const NUM_CHUNKS: u32 = 24;
    const NUM_SKIPPED: u32 = u32::BITS - Self::NUM_CHUNKS;

    fn new() -> Self {
        Self {
            first: AtomicPtr::new(core::ptr::null_mut()),
        }
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

    /// For this index in the whole buffer, returns the index of the [`Chunk`] and the index within that chunk.
    #[inline]
    fn map_to_indices(full_index: u32) -> (u32, u32) {
        // We're countint leading zeros since each chunk has power of 2 capacity.
        // So the leading zeros will be proportional to the chunk index.
        let leading = full_index
            .leading_zeros()
            // We do a min because we skip the first `NUM_SKIPPED` powers to make space for the first chunk's entity count.
            // The -1 is because this is the number of chunks, but we want the index in the end.
            .min(Self::NUM_CHUNKS - 1);
        // We store chunks in smallest to biggest order, so we need to reverse it.
        let chunk_index = Self::NUM_CHUNKS - 1 - leading;
        // We only need to cut off this particular bit.
        // The capacity is only one bit, and if other bits needed to be dropped, `leading` would have been greater
        let slice_index = full_index & !Self::capacity_of_chunk(chunk_index);

        (chunk_index, slice_index)
    }

    /// Gets the entity at the index within this chunk.
    ///
    /// # Safety
    ///
    /// [`Self::set`] must have been called on this index before.
    #[inline]
    unsafe fn get(&self, index: u32) -> Entity {
        // SAFETY: caller ensure we are init.
        let head = unsafe { self.ptr().debug_checked_unwrap() };
        // SAFETY: caller ensures we are in bounds (because `set` must be in bounds)
        let target = unsafe { &*head.add(index as usize) };

        // SAFETY: caller ensures `set` was called.
        unsafe { target.get_entity() }
    }

    /// Gets a slice of indices.
    ///
    /// # Safety
    ///
    /// [`Self::set`] must have been called on these indices before.
    #[inline]
    unsafe fn get_slice(&self, index: u32, ideal_len: u32, index_of_self: u32) -> &[Slot] {
        let cap = Self::capacity_of_chunk(index_of_self);
        let after_index_slice_len = cap - index;
        let len = after_index_slice_len.min(ideal_len) as usize;

        // SAFETY: caller ensure we are init.
        let head = unsafe { self.ptr().debug_checked_unwrap() };

        // SAFETY: The chunk was allocated via a `Vec` and the index is within the capacity.
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
    unsafe fn set(&self, index: u32, entity: Entity, index_of_self: u32) {
        let head = self.ptr().unwrap_or_else(|| self.init(index_of_self));
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
    unsafe fn init(&self, index: u32) -> *mut Slot {
        let cap = Self::capacity_of_chunk(index);
        let mut buff = ManuallyDrop::new(Vec::new());
        buff.reserve_exact(cap as usize);
        buff.resize_with(cap as usize, Slot::empty);
        let ptr = buff.as_mut_ptr();
        self.first.store(ptr, Ordering::Relaxed);
        ptr
    }

    /// Frees memory
    ///
    /// # Safety
    ///
    /// This must not be called concurrently.
    unsafe fn dealloc(&self, index: u32) {
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
}

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
struct FreeBufferLen(AtomicU64);

impl FreeBufferLen {
    /// The bit of the u64 with the highest bit of the u16 generation.
    const HIGHEST_GENERATION_BIT: u64 = 1 << 15;
    /// The u48 encoded length considers this value to be 0. Lower values are considered negative.
    const FALSE_ZERO: u64 = ((1 << 48) - 1) - ((1 << 32) - 1);
    /// This bit is off only when the length has been entirely disabled.
    const DISABLING_BIT: u64 = 1 << 63;

    /// Gets the current state of the buffer.
    #[inline]
    fn state(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    /// Constructs a length of 0.
    fn new_zero_len() -> Self {
        Self(AtomicU64::new(Self::FALSE_ZERO << 16))
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
struct FreeBuffer {
    /// The chunks of the free list.
    /// Put end-to-end, these chunks form a list of free entities.
    chunks: [Chunk; Chunk::NUM_CHUNKS as usize],
    /// The length of the free buffer
    len: FreeBufferLen,
}

impl FreeBuffer {
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
        let len = FreeBufferLen::len_from_state(state);
        // We can cast to u32 safely because if it were to overflow, there would already be too many entities.
        let (chunk_index, index) = Chunk::map_to_indices(len);

        // SAFETY: index is correct.
        let chunk = unsafe { self.chunks.get_unchecked(chunk_index as usize) };

        // SAFETY: Caller ensures this is not concurrent. The index is correct.
        // This can not confluct with a `get` because we already disabled remote allocation.
        unsafe {
            chunk.set(index, entity, chunk_index);
        }

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
        // We can cast to u32 safely because if it were to overflow, there would already be too many entities.
        let (chunk_index, index) = Chunk::map_to_indices(index);

        // SAFETY: index is correct.
        let chunk = unsafe { self.chunks.get_unchecked(chunk_index as usize) };

        // SAFETY: This was less then `len`, so it must have been `set` via `free` before.
        Some(unsafe { chunk.get(index) })
    }

    /// Allocates an as many [`Entity`]s from the free list as are available, up to `count`.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`Self::free`] calls for the duration of the returned iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> FreeListSliceIterator {
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
        FreeListSliceIterator {
            buffer: self,
            indices,
            current: [].iter(),
        }
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
        // Because we keep a generation value in the `FreeBufferLen`, if any of these things happen, we simply try again.

        let mut state = self.len.state();
        loop {
            // The state is only disabled when freeing.
            // If a free is happening, we need to wait for the new entity to be ready on the free buffer.
            // Then, we can allocate it.
            if FreeBufferLen::is_state_disabled(state) {
                core::hint::spin_loop();
                state = self.len.state();
                continue;
            }

            let len = FreeBufferLen::len_from_state(state);
            let index = len.checked_sub(1)?;

            // We can cast to u32 safely because if it were to overflow, there would already be too many entities.
            let (chunk_index, index) = Chunk::map_to_indices(index);

            // SAFETY: index is correct.
            let chunk = unsafe { self.chunks.get_unchecked(chunk_index as usize) };

            // SAFETY: This was less then `len`, so it must have been `set` via `free` before.
            let entity = unsafe { chunk.get(index) };

            let ideal_state = FreeBufferLen::pop_from_state(state, 1);
            match self.len.try_set_state(state, ideal_state) {
                Ok(_) => return Some(entity),
                Err(new_state) => state = new_state,
            }
        }
    }

    fn new() -> Self {
        Self {
            chunks: core::array::from_fn(|_index| Chunk::new()),
            len: FreeBufferLen::new_zero_len(),
        }
    }
}

impl Drop for FreeBuffer {
    fn drop(&mut self) {
        for index in 0..Chunk::NUM_CHUNKS {
            // SAFETY: we have `&mut`
            unsafe { self.chunks[index as usize].dealloc(index) };
        }
    }
}

/// A list that iterates the [`FreeBuffer`].
///
/// # Safety
///
/// Must be constructed to only iterate slots that have been initialized.
struct FreeListSliceIterator<'a> {
    buffer: &'a FreeBuffer,
    indices: core::ops::RangeInclusive<u32>,
    current: core::slice::Iter<'a, Slot>,
}

impl<'a> Iterator for FreeListSliceIterator<'a> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sliced) = self.current.next() {
            // SAFETY: Ensured by constructor
            unsafe {
                return Some(sliced.get_entity());
            }
        }

        let next_index = self.indices.next()?;
        let (chunk_index, inner_index) = Chunk::map_to_indices(next_index);
        // SAFETY: index is correct
        let chunk = unsafe { self.buffer.chunks.get_unchecked(chunk_index as usize) };

        // SAFETY: Assured by constructor
        let slice = unsafe { chunk.get_slice(inner_index, self.len() as u32 + 1, chunk_index) };
        self.indices = (*self.indices.start() + slice.len() as u32 - 1)..=(*self.indices.end());

        self.current = slice.iter();
        // SAFETY: Ensured by constructor
        unsafe { Some(self.current.next()?.get_entity()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.indices.end().saturating_sub(*self.indices.start()) as usize;
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for FreeListSliceIterator<'a> {}
impl<'a> core::iter::FusedIterator for FreeListSliceIterator<'a> {}

/// This stores allocation data shared by all entity allocators.
struct SharedAllocator {
    /// The entities pending reuse
    free: FreeBuffer,
    /// The next value of [`Entity::index`] to give out if needed.
    next_entity_index: AtomicU32,
    /// If true, the [`Self::next_entity_index`] has been incremented before,
    /// so if it hits or passes zero again, an overflow has occored.
    entity_index_given: AtomicBool,
}

impl SharedAllocator {
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
    /// This must not conflict with [`FreeBuffer::free`] calls.
    #[inline]
    unsafe fn alloc(&self) -> Entity {
        // SAFETY: assured by caller
        unsafe { self.free.alloc() }.unwrap_or_else(|| self.alloc_new_index())
    }

    /// Allocates a `count` [`Entity`]s, reusing freed indices if they exist.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeBuffer::free`] calls for the duration of the iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> AllocEntitiesIterator {
        let reused = self.free.alloc_many(count);
        let missing = count - reused.len() as u32;
        let start_new = self.next_entity_index.fetch_add(missing, Ordering::Relaxed);
        if start_new < missing {
            self.check_overflow();
        }
        let new = start_new..=(start_new + missing);
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

    fn new() -> Self {
        Self {
            free: FreeBuffer::new(),
            next_entity_index: AtomicU32::new(0),
            entity_index_given: AtomicBool::new(false),
        }
    }
}

/// This keeps track of freed entities and allows the allocation of new ones.
pub struct Allocator {
    shared: Arc<SharedAllocator>,
}

impl Allocator {
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
    reused: FreeListSliceIterator<'a>,
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
    // PERF: We could avoid the extra 2 atomic ops from upgrading and then dropping the `Weak`,
    // But this provides more safety and allows memory to be freed earlier.
    shared: Weak<SharedAllocator>,
}

impl RemoteAllocator {
    /// Allocates an entity remotely.
    /// This is not guaranteed to reuse a freed entity, even if one exists.
    ///
    /// This will return [`None`] if the source [`Allocator`] is destroyed.
    #[inline]
    pub fn alloc(&self) -> Option<Entity> {
        self.shared
            .upgrade()
            .map(|allocator| allocator.remote_alloc())
    }

    /// Returns whether or not this [`RemoteAllocator`] is still connected to its source [`Allocator`].
    pub fn is_closed(&self) -> bool {
        self.shared.strong_count() > 0
    }

    /// Creates a new [`RemoteAllocator`] with the provided [`Allocator`] source.
    /// If the source is ever destroyed, [`Self::alloc`] will yield [`None`].
    pub fn new(source: &Allocator) -> Self {
        Self {
            shared: Arc::downgrade(&source.shared),
        }
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
            assert_eq!(Chunk::map_to_indices(input), output);
        }
    }

    #[test]
    fn buffer_len_encoding() {
        let len = FreeBufferLen::new_zero_len();
        assert_eq!(len.len(), 0);
        assert_eq!(len.pop_for_len(200), 0);
        len.set_len(5, 0);
        assert_eq!(len.pop_for_len(2), 5);
        assert_eq!(len.pop_for_len(2), 3);
        assert_eq!(len.pop_for_len(2), 1);
        assert_eq!(len.pop_for_len(2), 0);
    }
}
