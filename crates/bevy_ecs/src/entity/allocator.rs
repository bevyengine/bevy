use bevy_platform::{
    prelude::Vec,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
};
use core::mem::ManuallyDrop;
use log::warn;
use nonmax::NonMaxU32;

use crate::query::DebugCheckedUnwrap;

use super::{Entity, EntityRow, EntitySetIterator};

/// This is the item we store in the free list.
/// Effectively, this is a `MaybeUninit<Entity>` where uninit is represented by `Entity::PLACEHOLDER`.
///
/// We use atomics internally not for special ordering but for *a* ordering.
/// Conceptually, this could just be `SyncCell<Entity>`,
/// but accessing that requires additional unsafe justification, and could cause unsound optimizations by the compiler.
///
/// No [`Slot`] access is ever contested between two threads due to the ordering constraints in the [`FreeCount`].
/// That also guarantees a proper ordering between slot access.
/// Hence these atomics don't need to account for any synchronization, and relaxed ordering is used everywhere.
// TODO: consider fully justifying `SyncCell` here with no atomics.
struct Slot {
    #[cfg(not(target_has_atomic = "64"))]
    entity_index: AtomicU32,
    #[cfg(not(target_has_atomic = "64"))]
    entity_generation: AtomicU32,
    #[cfg(target_has_atomic = "64")]
    inner_entity: AtomicU64,
}

impl Slot {
    /// Produces a meaningless empty value. This is a valid but incorrect `Entity`.
    /// It's valid because the bits do represent a valid bit pattern of an `Entity`.
    /// It's incorrect because this is in the free buffer even though the entity was never freed.
    /// Importantly, [`FreeCount`] determines which part of the free buffer is the free list.
    /// An empty slot may be in the free buffer, but should not be in the free list.
    /// This can be thought of as the `MaybeUninit` uninit in `Vec`'s excess capacity.
    fn empty() -> Self {
        let source = Entity::PLACEHOLDER;
        #[cfg(not(target_has_atomic = "64"))]
        return Self {
            entity_index: AtomicU32::new(source.index()),
            entity_generation: AtomicU32::new(source.generation().to_bits()),
        };
        #[cfg(target_has_atomic = "64")]
        return Self {
            inner_entity: AtomicU64::new(source.to_bits()),
        };
    }

    #[inline]
    fn set_entity(&self, entity: Entity) {
        #[cfg(not(target_has_atomic = "64"))]
        self.entity_generation
            .store(entity.generation().to_bits(), Ordering::Relaxed);
        #[cfg(not(target_has_atomic = "64"))]
        self.entity_index.store(entity.index(), Ordering::Relaxed);
        #[cfg(target_has_atomic = "64")]
        self.inner_entity.store(entity.to_bits(), Ordering::Relaxed);
    }

    /// Gets the stored entity. The result will be [`Entity::PLACEHOLDER`] unless [`set_entity`](Self::set_entity) has been called.
    #[inline]
    fn get_entity(&self) -> Entity {
        #[cfg(not(target_has_atomic = "64"))]
        return Entity {
            // SAFETY: This is valid since it was from an entity's index to begin with.
            row: unsafe {
                EntityRow::new(NonMaxU32::new_unchecked(
                    self.entity_index.load(Ordering::Relaxed),
                ))
            },
            generation: super::EntityGeneration::from_bits(
                self.entity_generation.load(Ordering::Relaxed),
            ),
        };
        #[cfg(target_has_atomic = "64")]
        // SAFETY: This is always sourced from a proper entity.
        return unsafe {
            Entity::try_from_bits(self.inner_entity.load(Ordering::Relaxed)).unwrap_unchecked()
        };
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
        // Relaxed is fine since caller ensures we are initialized already.
        // In order for the caller to guarantee that, they must have an ordering that orders this `get` after the required `set`.
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

        // Relaxed is fine since caller ensures we are initialized already.
        // In order for the caller to guarantee that, they must have an ordering that orders this `get` after the required `set`.
        let head = self.first.load(Ordering::Relaxed);

        // SAFETY: Caller ensures we are init, so the chunk was allocated via a `Vec` and the index is within the capacity.
        unsafe { core::slice::from_raw_parts(head, len) }
    }

    /// Sets this entity at this index.
    ///
    /// # Safety
    ///
    /// This must not be called concurrently with itself.
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
    /// This must not be called concurrently with itself.
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
    /// This must not be called concurrently with itself.
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

/// This is a buffer that has been split into power-of-two sized chunks, so that each chunk is pinned in memory.
/// Conceptually, each chunk is put end-to-end to form the buffer. This ultimately avoids copying elements on resize,
/// while allowing it to expand in capacity as needed. A separate system must track the length of the list in the buffer.
/// Each chunk is twice as large as the last, except for the first two which have a capacity of 512.
struct FreeBuffer([Chunk; Self::NUM_CHUNKS as usize]);

impl FreeBuffer {
    const NUM_CHUNKS: u32 = 24;
    const NUM_SKIPPED: u32 = u32::BITS - Self::NUM_CHUNKS;

    /// Constructs an empty [`FreeBuffer`].
    const fn new() -> Self {
        Self([const { Chunk::new() }; Self::NUM_CHUNKS as usize])
    }

    /// Computes the capacity of the chunk at this index within [`Self::NUM_CHUNKS`].
    /// The first 2 have length 512 (2^9) and the last has length (2^31)
    #[inline]
    fn capacity_of_chunk(chunk_index: u32) -> u32 {
        // We do this because we're skipping the first `NUM_SKIPPED` powers, so we need to make up for them by doubling the first index.
        // This is why the first 2 indices both have a capacity of 512.
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
        // SAFETY: Caller ensures the chunk index is correct
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
    /// This must not be called concurrently with itself.
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
    unsafe fn iter(&self, indices: core::ops::Range<u32>) -> FreeBufferIterator {
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
/// [`FreeBuffer::set`] must have been called on these indices beforehand to initialize memory.
struct FreeBufferIterator<'a> {
    buffer: &'a FreeBuffer,
    /// The indices in the buffer that are not in `current` yet.
    indices: core::ops::Range<u32>,
    current: core::slice::Iter<'a, Slot>,
}

impl<'a> Iterator for FreeBufferIterator<'a> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(found) = self.current.next() {
            return Some(found.get_entity());
        }

        let still_need = self.indices.len() as u32;
        let next_index = self.indices.next()?;
        let (chunk, index, chunk_capacity) = self.buffer.index_in_chunk(next_index);

        // SAFETY: Assured by constructor
        let slice = unsafe { chunk.get_slice(index, still_need, chunk_capacity) };
        self.indices.start += slice.len() as u32;
        self.current = slice.iter();

        // SAFETY: Constructor ensures these indices are valid in the buffer; the buffer is not sparse, and we just got the next slice.
        // So the only way for the slice to be empty is if the constructor did not uphold safety.
        let next = unsafe { self.current.next().debug_checked_unwrap() };
        Some(next.get_entity())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.indices.len() + self.current.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for FreeBufferIterator<'a> {}
impl<'a> core::iter::FusedIterator for FreeBufferIterator<'a> {}

/// This tracks the state of a [`FreeCount`], which has lots of information packed into it.
///
/// - The first 33 bits store a signed 33 bit integer. This behaves like a u33, but we define `1 << 32` as 0.
/// - The 34th bit stores a flag that indicates if the count has been disabled/suspended.
/// - The remaining 30 bits are the generation. The generation just differentiates different versions of the state that happen to encode the same length.
#[derive(Clone, Copy)]
struct FreeCountState(u64);

impl FreeCountState {
    /// When this bit is on, the count is disabled.
    /// This is used to prevent remote allocations from running at the same time as a free operation.
    const DISABLING_BIT: u64 = 1 << 33;
    /// This is the mask for the length bits.
    const LENGTH_MASK: u64 = (1 << 32) | u32::MAX as u64;
    /// This is the value of the length mask we consider to be 0.
    const LENGTH_0: u64 = 1 << 32;
    /// This is the lowest bit in the u30 generation.
    const GENERATION_LEAST_BIT: u64 = 1 << 34;

    /// Constructs a length of 0.
    const fn new_zero_len() -> Self {
        Self(Self::LENGTH_0)
    }

    /// Gets the encoded length.
    #[inline]
    const fn length(self) -> u32 {
        let unsigned_length = self.0 & Self::LENGTH_MASK;
        unsigned_length.saturating_sub(Self::LENGTH_0) as u32
    }

    /// Returns whether or not the count is disabled.
    #[inline]
    const fn is_disabled(self) -> bool {
        (self.0 & Self::DISABLING_BIT) > 0
    }

    /// Changes only the length of this count to `length`.
    #[inline]
    const fn with_length(self, length: u32) -> Self {
        // Just turns on the "considered zero" bit since this is non-negative.
        let length = length as u64 | Self::LENGTH_0;
        Self(self.0 & !Self::LENGTH_MASK | length)
    }

    /// For popping `num` off the count, subtract the resulting u64.
    #[inline]
    const fn encode_pop(num: u32) -> u64 {
        let subtract_length = num as u64;
        // Also subtract one from the generation bit.
        subtract_length | Self::GENERATION_LEAST_BIT
    }

    /// Returns the count after popping off `num` elements.
    #[inline]
    const fn pop(self, num: u32) -> Self {
        Self(self.0.wrapping_sub(Self::encode_pop(num)))
    }
}

/// This is an atomic interface to [`FreeCountState`].
struct FreeCount(AtomicU64);

impl FreeCount {
    /// Constructs a length of 0.
    const fn new_zero_len() -> Self {
        Self(AtomicU64::new(FreeCountState::new_zero_len().0))
    }

    /// Gets the current state of the buffer.
    #[inline]
    fn state(&self, order: Ordering) -> FreeCountState {
        FreeCountState(self.0.load(order))
    }

    /// Subtracts `num` from the length, returning the previous state.
    ///
    /// **NOTE:** Caller should be careful that changing the state is allowed and that the state is not disabled.
    #[inline]
    fn pop_for_state(&self, num: u32, order: Ordering) -> FreeCountState {
        let to_sub = FreeCountState::encode_pop(num);
        let raw = self.0.fetch_sub(to_sub, order);
        FreeCountState(raw)
    }

    /// Marks the state as disabled, returning the previous state
    #[inline]
    fn disable_len_for_state(&self, order: Ordering) -> FreeCountState {
        // We don't care about the generation here since this changes the value anyway.
        FreeCountState(self.0.fetch_or(FreeCountState::DISABLING_BIT, order))
    }

    /// Sets the state explicitly.
    /// Caller must be careful that the state has not changed since getting the state and setting it.
    #[inline]
    fn set_state_risky(&self, state: FreeCountState, order: Ordering) {
        self.0.store(state.0, order);
    }

    /// Attempts to update the state, returning the new [`FreeCountState`] if it fails.
    #[inline]
    fn try_set_state(
        &self,
        expected_current_state: FreeCountState,
        target_state: FreeCountState,
        success: Ordering,
        failure: Ordering,
    ) -> Result<(), FreeCountState> {
        self.0
            .compare_exchange(expected_current_state.0, target_state.0, success, failure)
            .map(|_| ())
            .map_err(FreeCountState)
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
        // Relaxed would probably be fine here, but this is more precise.
        self.len.state(Ordering::Acquire).length()
    }

    /// Frees the `entity` allowing it to be reused.
    ///
    /// # Safety
    ///
    /// This must not conflict with any other [`Self::free`] or [`Self::alloc`] calls.
    #[inline]
    unsafe fn free(&self, entity: Entity) {
        // Disable remote allocation.
        let state = self.len.disable_len_for_state(Ordering::Acquire);

        // Push onto the buffer
        let len = state.length();
        // SAFETY: Caller ensures this does not conflict with `free` or `alloc` calls,
        // and we just disabled remote allocation.
        unsafe {
            self.buffer.set(len, entity);
        }

        // Update length
        let new_state = state.with_length(len + 1);
        // This is safe because `alloc` is not being called and `remote_alloc` checks that it is not disabled.
        // We don't need to change the generation since this will change the length.
        // If, from a `remote_alloc` perspective, this does not change the length (i.e. this changes it *back* to what it was),
        // then `alloc` must have been called, which changes the generation.
        self.len.set_state_risky(new_state, Ordering::Release);
    }

    /// Allocates an [`Entity`] from the free list if one is available.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`Self::free`] calls.
    #[inline]
    unsafe fn alloc(&self) -> Option<Entity> {
        // SAFETY: This will get a valid index because there is no way for `free` to be done at the same time.
        let len = self.len.pop_for_state(1, Ordering::AcqRel).length();
        let index = len.checked_sub(1)?;

        // SAFETY: This was less then `len`, so it must have been `set` via `free` before.
        Some(unsafe { self.buffer.get(index) })
    }

    /// Allocates as many [`Entity`]s from the free list as are available, up to `count`.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`Self::free`] calls for the duration of the returned iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> FreeBufferIterator {
        // SAFETY: This will get a valid index because there is no way for `free` to be done at the same time.
        let len = self.len.pop_for_state(count, Ordering::AcqRel).length();
        let index = len.saturating_sub(count);

        // SAFETY: The iterator's items are all less than the length.
        unsafe { self.buffer.iter(index..len) }
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

        let mut state = self.len.state(Ordering::Acquire);
        #[cfg(feature = "std")]
        let mut attempts = 1u32;
        loop {
            // The state is only disabled when freeing.
            // If a free is happening, we need to wait for the new entity to be ready on the free buffer.
            // Then, we can allocate it.
            if state.is_disabled() {
                // Spin 64 times before yielding.
                #[cfg(feature = "std")]
                if attempts % 64 == 0 {
                    attempts += 1;
                    // scheduler probably isn't running the thread doing the `free` call, so yield so it can finish.
                    std::thread::yield_now();
                } else {
                    attempts += 1;
                    core::hint::spin_loop();
                }

                #[cfg(not(feature = "std"))]
                core::hint::spin_loop();

                state = self.len.state(Ordering::Acquire);
                continue;
            }

            let len = state.length();
            let index = len.checked_sub(1)?;

            // SAFETY: This was less than `len`, so it must have been `set` via `free` before.
            let entity = unsafe { self.buffer.get(index) };

            let ideal_state = state.pop(1);
            match self
                .len
                .try_set_state(state, ideal_state, Ordering::AcqRel, Ordering::Acquire)
            {
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
    /// Tracks whether or not the primary [`Allocator`] has been closed or not.
    is_closed: AtomicBool,
}

impl SharedAllocator {
    /// Constructs a [`SharedAllocator`]
    fn new() -> Self {
        Self {
            free: FreeList::new(),
            next_entity_index: AtomicU32::new(0),
            is_closed: AtomicBool::new(false),
        }
    }

    /// The total number of indices given out.
    #[inline]
    fn total_entity_indices(&self) -> u32 {
        self.next_entity_index.load(Ordering::Relaxed)
    }

    /// This just panics.
    /// It is included to help with branch prediction, and put the panic message in one spot.
    #[cold]
    #[inline]
    fn on_overflow() -> ! {
        panic!("too many entities")
    }

    /// Allocates a fresh [`EntityRow`]. This row has never been given out before.
    #[inline]
    pub(crate) fn alloc_unique_entity_row(&self) -> EntityRow {
        let index = self.next_entity_index.fetch_add(1, Ordering::Relaxed);
        if index == u32::MAX {
            Self::on_overflow();
        }
        // SAFETY: We just checked that this was not max.
        unsafe { EntityRow::new(NonMaxU32::new_unchecked(index)) }
    }

    /// Allocates `count` [`EntityRow`]s. These rows will be fresh. They have never been given out before.
    pub(crate) fn alloc_unique_entity_rows(&self, count: u32) -> AllocUniqueEntityRowIterator {
        let start_new = self.next_entity_index.fetch_add(count, Ordering::Relaxed);
        let new = match start_new.checked_add(count) {
            Some(new_next_entity_index) => start_new..new_next_entity_index,
            None => Self::on_overflow(),
        };
        AllocUniqueEntityRowIterator(new)
    }

    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeList::free`] calls.
    #[inline]
    unsafe fn alloc(&self) -> Entity {
        // SAFETY: assured by caller
        unsafe { self.free.alloc() }
            .unwrap_or_else(|| Entity::from_raw(self.alloc_unique_entity_row()))
    }

    /// Allocates a `count` [`Entity`]s, reusing freed indices if they exist.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeList::free`] calls for the duration of the iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> AllocEntitiesIterator {
        let reused = self.free.alloc_many(count);
        let still_need = count - reused.len() as u32;
        let new = self.alloc_unique_entity_rows(still_need);
        AllocEntitiesIterator { new, reused }
    }

    /// Allocates a new [`Entity`].
    /// This will only try to reuse a freed index if it is safe to do so.
    #[inline]
    fn remote_alloc(&self) -> Entity {
        self.free
            .remote_alloc()
            .unwrap_or_else(|| Entity::from_raw(self.alloc_unique_entity_row()))
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
    pub fn total_entity_indices(&self) -> u32 {
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
    pub fn is_valid_row(&self, row: EntityRow) -> bool {
        row.index() < self.total_entity_indices()
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
        // SAFETY: `free` takes `&mut self`, and this lifetime is captured by the iterator.
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
        // SAFETY: Caller ensures free is not called.
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

/// An [`Iterator`] returning a sequence of [`EntityRow`] values from an [`Allocator`] that are never aliased.
/// These rows have never been given out before.
///
/// **NOTE:** Dropping will leak the remaining entity rows!
pub(crate) struct AllocUniqueEntityRowIterator(core::ops::Range<u32>);

impl Iterator for AllocUniqueEntityRowIterator {
    type Item = EntityRow;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            // SAFETY: This came from an *exclusive* range. It can never be max.
            .map(|idx| unsafe { EntityRow::new(NonMaxU32::new_unchecked(idx)) })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for AllocUniqueEntityRowIterator {}
impl core::iter::FusedIterator for AllocUniqueEntityRowIterator {}

/// An [`Iterator`] returning a sequence of [`Entity`] values from an [`Allocator`].
///
/// **NOTE:** Dropping will leak the remaining entities!
pub struct AllocEntitiesIterator<'a> {
    new: AllocUniqueEntityRowIterator,
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
        let len = self.reused.len() + self.new.len();
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

    /// Ensure the total capacity of [`OwnedBuffer`] is `u32::MAX + 1`.
    #[test]
    fn chunk_capacity_sums() {
        let total: u64 = (0..FreeBuffer::NUM_CHUNKS)
            .map(FreeBuffer::capacity_of_chunk)
            .map(|x| x as u64)
            .sum();
        // The last 2 won't be used, but that's ok.
        // Keeping them powers of 2 makes things faster.
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
        assert_eq!(len.state(Ordering::Relaxed).length(), 0);
        assert_eq!(len.pop_for_state(200, Ordering::Relaxed).length(), 0);
        len.set_state_risky(
            FreeCountState::new_zero_len().with_length(5),
            Ordering::Relaxed,
        );
        assert_eq!(len.pop_for_state(2, Ordering::Relaxed).length(), 5);
        assert_eq!(len.pop_for_state(2, Ordering::Relaxed).length(), 3);
        assert_eq!(len.pop_for_state(2, Ordering::Relaxed).length(), 1);
        assert_eq!(len.pop_for_state(2, Ordering::Relaxed).length(), 0);
    }

    #[test]
    fn uniqueness() {
        let mut entities = Vec::with_capacity(2000);
        let mut allocator = Allocator::new();
        entities.extend(allocator.alloc_many(1000));

        let pre_len = entities.len();
        entities.dedup();
        assert_eq!(pre_len, entities.len());

        for e in entities.drain(..) {
            allocator.free(e);
        }

        entities.extend(allocator.alloc_many(500));
        for _ in 0..1000 {
            entities.push(allocator.alloc());
        }
        entities.extend(allocator.alloc_many(500));

        let pre_len = entities.len();
        entities.dedup();
        assert_eq!(pre_len, entities.len());
    }
}
