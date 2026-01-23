//! This module contains the guts of Bevy's entity allocator.
//!
//! Entity allocation needs to work concurrently and remotely.
//! Remote allocations (where no reference to the world is held) is needed for long running tasks, such as loading assets on separate threads.
//! Non-remote, "normal" allocation needs to be as fast as possible while still supporting remote allocation.
//!
//! The allocator fundamentally is made of a cursor for the next fresh, never used [`EntityIndex`] and a free list.
//! The free list is a collection that holds [`Entity`] values that were used and can be reused; they are "free"/available.
//! If the free list is empty, it's really simple to just increment the fresh index cursor.
//! The tricky part is implementing a remotely accessible free list.
//!
//! A naive free list could just a concurrent queue.
//! That would probably be fine for remote allocation but for non-remote, we can go much faster.
//! In particular, a concurrent queue must do additional work to handle cases where something is added concurrently with being removed.
//! But for non-remote allocation, we can guarantee that no free will happen during an allocation since `free` needs mutably access to the world already.
//! That means, we can skip a lot of those safety checks.
//! Plus, we know the maximum size of the free list ahead of time, since we can assume there are no duplicates.
//! That means, we can have a much more efficient allocation scheme, far better than a linked list.
//!
//! For the free list, the list needs to be pinned in memory and yet grow-able.
//! That's quite the pickle, but by splitting the growth over multiple arrays, this isn't so bad.
//! When the list needs to grow, we just *add* on another array to the buffer (instead of *replacing* the old one with a bigger one).
//! These arrays are called [`Chunk`]s.
//! This keeps everything pinned, and since we know the maximum size ahead of time, we can make this mapping very fast.
//!
//! Similar to how `Vec` is implemented, the free list is implemented as a [`FreeBuffer`] (handling allocations and implicit capacity)
//! and the [`FreeCount`] manages the length of the free list.
//! The free list's item is a [`Slot`], which manages accessing each item concurrently.
//!
//! These types are summed up in [`SharedAllocator`], which is highly unsafe.
//! The interfaces [`Allocator`] and [`RemoteAllocator`] provide safe interfaces to them.

use arrayvec::ArrayVec;
use bevy_platform::{
    cell::SyncUnsafeCell,
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

use super::{Entity, EntityIndex, EntitySetIterator};

/// This is the item we store in the free list.
/// Effectively, this is a `MaybeUninit<Entity>` where uninit is represented by `Entity::PLACEHOLDER`.
struct Slot {
    inner: SyncUnsafeCell<Entity>,
}

impl Slot {
    /// Produces a meaningless empty value. This is a valid but incorrect `Entity`.
    /// It's valid because the bits do represent a valid bit pattern of an `Entity`.
    /// It's incorrect because this is in the free buffer even though the entity was never freed.
    /// Importantly, [`FreeCount`] determines which part of the free buffer is the free list.
    /// An empty slot may be in the free buffer, but should not be in the free list.
    /// This can be thought of as the `MaybeUninit` uninit in `Vec`'s excess capacity.
    const fn empty() -> Self {
        let source = Entity::PLACEHOLDER;
        Self {
            inner: SyncUnsafeCell::new(source),
        }
    }

    /// Sets the entity at this slot.
    ///
    /// # Safety
    ///
    /// There must be a clear, strict order between this call and the previous uses of this [`Slot`].
    /// Otherwise, the compiler will make unsound optimizations.
    #[inline]
    const unsafe fn set_entity(&self, entity: Entity) {
        // SAFETY: Ensured by caller.
        unsafe {
            self.inner.get().write(entity);
        }
    }

    /// Gets the stored entity. The result will be [`Entity::PLACEHOLDER`] unless [`set_entity`](Self::set_entity) has been called.
    ///
    /// # Safety
    ///
    /// There must be a clear, strict order between this call and the previous uses of this [`Slot`].
    /// Otherwise, the compiler will make unsound optimizations.
    #[inline]
    const unsafe fn get_entity(&self) -> Entity {
        // SAFETY: Ensured by caller.
        unsafe { self.inner.get().read() }
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
    /// There must be a clear, strict order between this call and the previous uses of this `index`.
    /// Otherwise, the compiler will make unsound optimizations.
    #[inline]
    unsafe fn get(&self, index: u32) -> Entity {
        // Relaxed is fine since caller has already assured memory ordering is satisfied.
        let head = self.first.load(Ordering::Relaxed);
        // SAFETY: caller ensures we are in bounds and init (because `set` must be in bounds)
        let target = unsafe { &*head.add(index as usize) };
        // SAFETY: Caller ensures ordering.
        unsafe { target.get_entity() }
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
    /// Index must be in bounds.
    /// There must be a clear, strict order between this call and the previous uses of this `index`.
    /// Otherwise, the compiler will make unsound optimizations.
    /// This must not be called on the same chunk concurrently.
    #[inline]
    unsafe fn set(&self, index: u32, entity: Entity, chunk_capacity: u32) {
        // Relaxed is fine here since the caller ensures memory ordering.
        let ptr = self.first.load(Ordering::Relaxed);
        let head = if ptr.is_null() {
            // SAFETY: Ensured by caller.
            unsafe { self.init(chunk_capacity) }
        } else {
            ptr
        };

        // SAFETY: caller ensures it is in bounds and we are not fighting with other `set` calls or `get` calls.
        // A race condition is therefore impossible.
        // The address can't wrap or pass isize max since this addition is within an allocation.
        // For that to happen, you would first run out of memory in practice.
        let target = unsafe { &*head.add(index as usize) };

        // SAFETY: Ensured by caller.
        unsafe {
            target.set_entity(entity);
        }
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
    /// `chunk_capacity` must be the same as it was initialized with.
    unsafe fn dealloc(&mut self, chunk_capacity: u32) {
        let to_drop = *self.first.get_mut();
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
    const fn capacity_of_chunk(chunk_index: u32) -> u32 {
        // We do this because we're skipping the first `NUM_SKIPPED` powers, so we need to make up for them by doubling the first index.
        // This is why the first 2 indices both have a capacity of 512.
        let corrected = if chunk_index == 0 { 1 } else { chunk_index };
        // We add NUM_SKIPPED because the total capacity should be as if [`Self::NUM_CHUNKS`] were 32.
        // This skips the first NUM_SKIPPED powers.
        let corrected = corrected + Self::NUM_SKIPPED;
        // This bit shift is just 2^corrected.
        1 << corrected
    }

    /// For this index in the whole buffer, returns the index of the [`Chunk`], the index within that chunk, and the capacity of that chunk.
    #[inline]
    const fn index_info(full_index: u32) -> (u32, u32, u32) {
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
        // SAFETY: The `index_info` is correct.
        let chunk = unsafe { self.0.get_unchecked(chunk_index as usize) };
        (chunk, index_in_chunk, chunk_capacity)
    }

    /// Gets the entity at an index.
    ///
    /// # Safety
    ///
    /// [`set`](Self::set) must have been called on this index to initialize its memory.
    /// There must be a clear, strict order between this call and the previous uses of this `full_index`.
    /// Otherwise, the compiler will make unsound optimizations.
    unsafe fn get(&self, full_index: u32) -> Entity {
        let (chunk, index, _) = self.index_in_chunk(full_index);
        // SAFETY: Ensured by caller.
        unsafe { chunk.get(index) }
    }

    /// Sets an entity at an index.
    ///
    /// # Safety
    ///
    /// There must be a clear, strict order between this call and the previous uses of this `full_index`.
    /// Otherwise, the compiler will make unsound optimizations.
    /// This must not be called on the same buffer concurrently.
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
    /// There must be a clear, strict order between this call and the previous uses of these `indices`.
    /// Note that until the returned value is dropped, these `indices` are still being accessed,
    /// making safety for other operations afterward need careful justification.
    /// Otherwise, the compiler will make unsound optimizations.
    #[inline]
    unsafe fn iter(&self, indices: core::ops::Range<u32>) -> FreeBufferIterator<'_> {
        FreeBufferIterator {
            buffer: self,
            future_buffer_indices: indices,
            current_chunk_slice: [].iter(),
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
    /// The part of the buffer we are iterating at the moment.
    current_chunk_slice: core::slice::Iter<'a, Slot>,
    /// The indices in the buffer that are not yet in `current_chunk_slice`.
    future_buffer_indices: core::ops::Range<u32>,
}

impl<'a> Iterator for FreeBufferIterator<'a> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(found) = self.current_chunk_slice.next() {
            // SAFETY: We have `&mut self`, so that memory order is certain.
            // The caller of `FreeBuffer::iter` ensures the memory order of this value's lifetime.
            return Some(unsafe { found.get_entity() });
        }

        let still_need = self.future_buffer_indices.len() as u32;
        let next_index = self.future_buffer_indices.next()?;
        let (chunk, index, chunk_capacity) = self.buffer.index_in_chunk(next_index);

        // SAFETY: Assured by `FreeBuffer::iter`
        let slice = unsafe { chunk.get_slice(index, still_need, chunk_capacity) };
        self.future_buffer_indices.start += slice.len() as u32;
        self.current_chunk_slice = slice.iter();

        // SAFETY: Constructor ensures these indices are valid in the buffer; the buffer is not sparse, and we just got the next slice.
        // So the only way for the slice to be empty is if the constructor did not uphold safety.
        let next = unsafe { self.current_chunk_slice.next().debug_checked_unwrap() };
        // SAFETY: We have `&mut self`, so that memory order is certain.
        // The caller of `FreeBuffer::iter` ensures the memory order of this value's lifetime.
        Some(unsafe { next.get_entity() })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.future_buffer_indices.len() + self.current_chunk_slice.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for FreeBufferIterator<'a> {}
impl<'a> core::iter::FusedIterator for FreeBufferIterator<'a> {}

/// This tracks the state of a [`FreeCount`], which has lots of information packed into it.
///
/// This has three jobs:
///
///  - First, obviously, this needs to track the length of the free list.
///    When the length is 0, we use the [`FreshAllocator`]; otherwise, we pop.
///    The length also tells us where on the list to push freed entities to.
///  - Second, we need to be able to "freeze" the length for remote allocations.
///    This happens when pushing to the list; we need to prevent a push and remote pop from happening at the same time.
///    We call this "disabling the length".
///    When it is disabled, only the thing that disabled it is allowed to re-enable it.
///    This is like a mutex, but it's faster because we pack the mutex into the same bits as the state.
///    See [`FreeCount::disable_len_for_state`] and [`FreeCount::set_state_risky`] for how this can be done.
///  - Third, we need to track the generation of the free list.
///    That is, any two distinct states of the free list, even if they are the same length, must have different [`FreeCount`] values.
///    This becomes important when a remote allocator needs to know if the information it is working with has been outdated.
///    See [`FreeList::remote_alloc`] for why this is so important.
///
/// As if that isn't hard enough, we need to do all three of these things in the same [`AtomicU64`] for performance.
/// Not only that, but for memory ordering guarantees, we need to be able to change the length and generation in a single atomic operation.
/// We do that with a very specific bit layout:
///
/// - The least significant 33 bits store a signed 33 bit integer for the length.
///   This behaves like a u33, but we define `1 << 32` as 0.
/// - The 34th bit stores a flag that indicates if the length has been disabled.
/// - The remaining 30 bits are the generation.
///   The generation helps differentiates different versions of the state that happen to encode the same length.
///
/// Why this layout?
/// A few observations:
/// First, since the disabling mechanic acts as a mutex, we only need one bit for that, and we can use bit operations to interact with it.
/// That leaves the length and the generation (which we need to distinguish between two states of the free list that happen to be the same length).
/// Every change to the length must be/cause a change to the [`FreeCountState`] such that the new state does not equal any previous state.
/// The second observation is that we only need to change the generation when we move the length in one direction.
/// Here, we tie popping/allocation to a generation change.
/// When the length increases, the length part of the state changes, so a generation change is a moot point. (Ex `L0-G0` -> `L1G0`)
/// When the length decreases, we also need to change the generation to distinguish the states. (Ex `L1-G0` -> `L0G1`)
///
/// We need the generation to freely wrap.
/// In this case, the generation is 30 bits, so after 2 ^ 30 allocations, the generation will wrap.
/// That is technically a soundness concern,
/// but it would only cause a problem if the same [`FreeList::remote_alloc`] procedure had been sleeping for all 2 ^ 30 allocations and then when it woke up, all 2 ^ 30 allocations had been freed.
/// This is impossibly unlikely and is safely ignored in other concurrent queue implementations.
/// Still, we need the generation to wrap; it must not overflow into the length bits.
/// As a result, the generation bits *must* be the most significant; this allows them to wrap freely.
///
/// It is convenient to put the disabling bit next since that leaves the length bits already aligned to the least significant bits.
/// That saves us a bit shift!
///
/// But now we need to stop the length information from messing with the generation or disabling bits.
/// Preventing overflow is easy since we can assume the list is unique and there are only `u32::MAX` [`Entity`] values.
/// We can't prevent underflow with just 32 bits, and performance prevents us from running checks before a subtraction.
/// But we do know that it can't overflow more than `u32::MAX` times because that would cause the [`FreshAllocator`] to overflow and panic for allocating too many entities.
/// That means we need to represent "length" values in `±u32::MAX` range, which gives us an `i33` that we then saturatingly cast to `u32`.
/// As mentioned above, we represent this `i33` as a `u33` where we define `1 << 32` as 0.
/// This representation works slightly easier for the `saturating_sub` in [`FreeCountState::length`] than a true `i33` representation.
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
    /// When the length is disabled, [`try_set_state`](Self::try_set_state) will fail.
    /// This is used to prevent remote allocation during a free.
    #[inline]
    fn disable_len_for_state(&self, order: Ordering) -> FreeCountState {
        // We don't care about the generation here since this changes the value anyway.
        FreeCountState(self.0.fetch_or(FreeCountState::DISABLING_BIT, order))
    }

    /// Sets the state explicitly.
    /// Caller must be careful that the state has not changed since getting the state and setting it.
    /// If that happens, the state may not properly reflect the length of the free list or its generation,
    /// causing entities to be skipped or given out twice.
    /// This is not a safety concern, but it is a major correctness concern.
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
        match self
            .0
            .compare_exchange(expected_current_state.0, target_state.0, success, failure)
        {
            Ok(_) => Ok(()),
            Err(val) => Err(FreeCountState(val)),
        }
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
    /// # Risk
    ///
    /// For this to be accurate, this must not be called during a [`Self::free`].
    #[inline]
    fn num_free(&self) -> u32 {
        // Relaxed ordering is fine since this doesn't act on the length value in memory.
        self.len.state(Ordering::Relaxed).length()
    }

    /// Frees the `entities` allowing them to be reused.
    ///
    /// # Safety
    ///
    /// There must be a clear, strict order between this call and calls to [`Self::free`], [`Self::alloc_many`], and [`Self::alloc`].
    /// Otherwise, the compiler will make unsound optimizations.
    #[inline]
    unsafe fn free(&self, entities: &[Entity]) {
        // Disable remote allocation.
        // We don't need to acquire the most recent memory from remote threads because we never read it.
        // We do not need to release to remote threads because we only changed the disabled bit,
        // which the remote allocator would with relaxed ordering.
        let state = self.len.disable_len_for_state(Ordering::Relaxed);

        // Append onto the buffer
        let mut len = state.length();
        entities.iter().copied().for_each(|entity| {
            // SAFETY: Caller ensures this does not conflict with `free` or `alloc` calls,
            // and we just disabled remote allocation with a strict memory ordering.
            // We only call `set` during a free, and the caller ensures that is not called concurrently.
            unsafe {
                self.buffer.set(len, entity);
            }
            len += 1;
        });

        // Update length
        let new_state = state.with_length(len);
        // This is safe because `alloc` is not being called and `remote_alloc` checks that it is not disabled.
        // We don't need to change the generation since this will change the length, which changes the value anyway.
        // If, from a `remote_alloc` perspective, this does not change the length (i.e. this changes it *back* to what it was),
        // then `alloc` must have been called, which changes the generation.
        self.len.set_state_risky(new_state, Ordering::Release);
    }

    /// Allocates an [`Entity`] from the free list if one is available.
    ///
    /// # Safety
    ///
    /// There must be a clear, strict order between this call and calls to [`Self::free`].
    /// Otherwise, the compiler will make unsound optimizations.
    #[inline]
    unsafe fn alloc(&self) -> Option<Entity> {
        // SAFETY: This will get a valid index because caller ensures there is no way for `free` to be done at the same time.
        // Relaxed is ok here since `free` is the only time memory is changed, and relaxed still gets the most recent state.
        // The memory ordering to ensure we read the most recent value at the index is ensured by the caller.
        let len = self.len.pop_for_state(1, Ordering::Relaxed).length();
        let index = len.checked_sub(1)?;

        // SAFETY: This was less then `len`, so it must have been `set` via `free` before.
        // There is a strict memory ordering of this use of the index because the length is only decreasing.
        // That means there is only one use of this index since the last call to `free`.
        // The only time the length increases is during `free`, which the caller ensures has a "happened before" relationship with this call.
        Some(unsafe { self.buffer.get(index) })
    }

    /// Allocates as many [`Entity`]s from the free list as are available, up to `count`.
    ///
    /// # Safety
    ///
    /// There must be a clear, strict order between this call and calls to [`Self::free`].
    /// Otherwise, the compiler will make unsound optimizations.
    ///
    /// Note that this allocation call doesn't end until the returned value is dropped.
    /// So, calling [`Self::free`] while the returned value is live is unsound.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> FreeBufferIterator<'_> {
        // SAFETY: This will get a valid index because there is no way for `free` to be done at the same time.
        // Relaxed is ok here since `free` is the only time memory is changed, and relaxed still gets the most recent state.
        // The memory ordering to ensure we read the most recent value at the index is ensured by the caller.
        let len = self.len.pop_for_state(count, Ordering::Relaxed).length();
        let index = len.saturating_sub(count);

        // SAFETY: The iterator's items are all less than the length, so they are in bounds and have been previously set.
        // There is a strict memory ordering of this use of the indices because the length is only decreasing.
        // That means there is only one use of these indices since the last call to `free`.
        // The only time it the length increases is during `free`, which the caller ensures has a "happened before" relationship with this call.
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
        // We also need to prevent this from conflicting with a `free` call, so we check to ensure the state is not disabled.

        // We keep track of the attempts so we can yield the thread on std after a few fails.
        #[cfg(feature = "std")]
        let mut attempts = 1u32;
        // We need an acquire ordering to acquire the most recent memory of `free` calls.
        let mut state = self.len.state(Ordering::Acquire);
        loop {
            // The state is only disabled when freeing.
            // If a free is happening, we need to wait for the new entity to be ready on the free buffer.
            // That means we will also need to re-fetch the state and acquire the new memory.
            // Then, we can allocate it.
            if state.is_disabled() {
                // Spin 64 times before yielding.
                #[cfg(feature = "std")]
                {
                    attempts += 1;
                    if attempts.is_multiple_of(64) {
                        // scheduler probably isn't running the thread doing the `free` call, so yield so it can finish.
                        std::thread::yield_now();
                    } else {
                        core::hint::spin_loop();
                    }
                }

                #[cfg(not(feature = "std"))]
                core::hint::spin_loop();

                // Retry with the fresh state and acquired memory order.
                state = self.len.state(Ordering::Acquire);
                continue;
            }

            // At this point, we know a `free` was not happening when we started.

            let len = state.length();
            let index = len.checked_sub(1)?;

            // SAFETY:
            //
            // If no `free` call has started, this safety follows the same logic as in non-remote `alloc`.
            // That is, the len always counts down, so this is the only use of this index since the last `free`,
            // and another `free` hasn't happened.
            //
            // But if a `free` did start at this point, it would be operating on indices greater than `index`.
            // We haven't updated the `FreeCount` yet, so the `free` call would be adding to it, while we've been subtracting from it.
            // That means this is still the only time this index is used since the last `free`!
            // So, even though we can't guarantee when the concurrent `free` is happening in memory order, it doesn't matter since that `free` doesn't use this index.
            // We can still establish a clear, strict ordering for this slot because 1) any concurrent `free` doesn't use this index and 2) we have an `Acquire` relationship with the `free` before it.
            //
            // So yeah, we could be reading from outdated memory (the free buffer), but the part that we are reading, hasn't changed, so that's ok.
            // That satisfies safety but not correctness.
            // We still need to double check that a free didn't happen, and retry if it did.
            // Otherwise, this entity might be given out twice.
            let entity = unsafe { self.buffer.get(index) };

            let ideal_state = state.pop(1);
            // If we fail, we need to acquire the new state.
            // If we succeed, we are finished, and we haven't changed any memory, so we can used relaxed ordering.
            match self
                .len
                .try_set_state(state, ideal_state, Ordering::Relaxed, Ordering::Acquire)
            {
                Ok(_) => return Some(entity),
                Err(new_state) => state = new_state,
            }
        }
    }
}

struct FreshAllocator {
    /// The next value of [`Entity::index`] to give out if needed.
    next_entity_index: AtomicU32,
}

impl FreshAllocator {
    /// This exists because it may possibly change depending on platform.
    /// Ex: We may want this to be smaller on 32 bit platforms at some point.
    const MAX_ENTITIES: u32 = u32::MAX;

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

    /// Allocates a fresh [`EntityIndex`].
    /// This row has never been given out before.
    #[inline]
    fn alloc(&self) -> Entity {
        let index = self.next_entity_index.fetch_add(1, Ordering::Relaxed);
        if index == Self::MAX_ENTITIES {
            Self::on_overflow();
        }
        // SAFETY: We just checked that this was not max and we only added 1, so we can't have missed it.
        Entity::from_index(unsafe { EntityIndex::new(NonMaxU32::new_unchecked(index)) })
    }

    /// Allocates `count` [`EntityIndex`]s.
    /// These rows will be fresh.
    /// They have never been given out before.
    fn alloc_many(&self, count: u32) -> AllocUniqueEntityIndexIterator {
        let start_new = self.next_entity_index.fetch_add(count, Ordering::Relaxed);
        let new = match start_new
            .checked_add(count)
            .filter(|new| *new < Self::MAX_ENTITIES)
        {
            Some(new_next_entity_index) => start_new..new_next_entity_index,
            None => Self::on_overflow(),
        };
        AllocUniqueEntityIndexIterator(new)
    }
}

/// An [`Iterator`] returning a sequence of [`EntityIndex`] values from an [`Allocator`] that are never aliased.
/// These rows have never been given out before.
///
/// **NOTE:** Dropping will leak the remaining entity rows!
pub(super) struct AllocUniqueEntityIndexIterator(core::ops::Range<u32>);

impl Iterator for AllocUniqueEntityIndexIterator {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            // SAFETY: This came from an *exclusive* range. It can never be max.
            .map(|idx| unsafe { EntityIndex::new(NonMaxU32::new_unchecked(idx)) })
            .map(Entity::from_index)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for AllocUniqueEntityIndexIterator {}
impl core::iter::FusedIterator for AllocUniqueEntityIndexIterator {}

/// This stores allocation data shared by all entity allocators.
struct SharedAllocator {
    /// The entities pending reuse
    free: FreeList,
    fresh: FreshAllocator,
    /// Tracks whether or not the primary [`Allocator`] has been closed or not.
    is_closed: AtomicBool,
}

impl SharedAllocator {
    /// Constructs a [`SharedAllocator`]
    fn new() -> Self {
        Self {
            free: FreeList::new(),
            fresh: FreshAllocator {
                next_entity_index: AtomicU32::new(0),
            },
            is_closed: AtomicBool::new(false),
        }
    }

    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeList::free`] calls.
    #[inline]
    unsafe fn alloc(&self) -> Entity {
        // SAFETY: assured by caller
        unsafe { self.free.alloc() }.unwrap_or_else(|| self.fresh.alloc())
    }

    /// Allocates a `count` [`Entity`]s, reusing freed indices if they exist.
    ///
    /// # Safety
    ///
    /// This must not conflict with [`FreeList::free`] calls for the duration of the iterator.
    #[inline]
    unsafe fn alloc_many(&self, count: u32) -> AllocEntitiesIterator<'_> {
        // SAFETY: Ensured by caller.
        let reused = unsafe { self.free.alloc_many(count) };
        let still_need = count - reused.len() as u32;
        let new = self.fresh.alloc_many(still_need);
        AllocEntitiesIterator { new, reused }
    }

    /// Allocates a new [`Entity`].
    /// This will only try to reuse a freed index if it is safe to do so.
    #[inline]
    fn remote_alloc(&self) -> Entity {
        self.free
            .remote_alloc()
            .unwrap_or_else(|| self.fresh.alloc())
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
///
/// Note that this must not implement [`Clone`].
/// The allocator assumes that it is the only one with [`FreeList::free`] permissions.
/// If this were cloned, that assumption would be broken, leading to undefined behavior.
/// This is in contrast to the [`RemoteAllocator`], which may be cloned freely.
pub(super) struct Allocator {
    shared: Arc<SharedAllocator>,
    quick_free: ArrayVec<Entity, 64>,
}

impl Default for Allocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Allocator {
    /// Constructs a new [`Allocator`]
    pub(super) fn new() -> Self {
        Self {
            shared: Arc::new(SharedAllocator::new()),
            quick_free: ArrayVec::new(),
        }
    }

    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    #[inline]
    pub(super) fn alloc(&self) -> Entity {
        // SAFETY: violating safety requires a `&mut self` to exist, but rust does not allow that.
        unsafe { self.shared.alloc() }
    }

    /// The total number of indices given out.
    #[inline]
    fn total_entity_indices(&self) -> u32 {
        self.shared.fresh.total_entity_indices()
    }

    /// The number of free entities.
    #[inline]
    fn num_free(&self) -> u32 {
        // RISK: `free` requires mutable access.
        self.shared.free.num_free()
    }

    /// Frees the entity allowing it to be reused.
    #[inline]
    pub(super) fn free(&mut self, entity: Entity) {
        if self.quick_free.is_full() {
            // SAFETY: We have `&mut self`.
            unsafe {
                self.shared.free.free(self.quick_free.as_slice());
            }
            self.quick_free.clear();
        }
        // SAFETY: The `ArrayVec` is not full or has just been cleared.
        unsafe {
            self.quick_free.push_unchecked(entity);
        }
    }

    /// Allocates `count` entities in an iterator.
    #[inline]
    pub(super) fn alloc_many(&self, count: u32) -> AllocEntitiesIterator<'_> {
        // SAFETY: `free` takes `&mut self`, and this lifetime is captured by the iterator.
        unsafe { self.shared.alloc_many(count) }
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
pub(super) struct AllocEntitiesIterator<'a> {
    new: AllocUniqueEntityIndexIterator,
    reused: FreeBufferIterator<'a>,
}

impl<'a> Iterator for AllocEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.reused.next().or_else(|| self.new.next())
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

/// This is a stripped down entity allocator that operates on fewer assumptions than [`EntityAllocator`](super::EntityAllocator).
/// As a result, using this will be slower than than the main allocator but this offers additional freedoms.
/// In particular, this type is fully owned, allowing you to allocate entities for a world without locking or holding reference to the world.
/// This is especially useful in async contexts.
#[derive(Clone)]
pub struct RemoteAllocator {
    shared: Arc<SharedAllocator>,
}

impl RemoteAllocator {
    /// Creates a new [`RemoteAllocator`] with the provided [`Allocator`] source.
    /// If the source is ever destroyed, [`Self::alloc`] will yield garbage values.
    /// Be sure to use [`Self::is_closed`] to determine if it is safe to use these entities.
    pub(super) fn new(source: &Allocator) -> Self {
        Self {
            shared: source.shared.clone(),
        }
    }

    /// Returns whether or not this [`RemoteAllocator`] is connected to this source [`Allocator`].
    pub(super) fn is_connected_to(&self, source: &Allocator) -> bool {
        Arc::ptr_eq(&self.shared, &source.shared)
    }

    /// Allocates an entity remotely.
    ///
    /// This comes with a major downside:
    /// Because this does not hold reference to the world, the world may be cleared or destroyed before you get a chance to use the result.
    /// If that happens, these entities will be garbage!
    /// They will not be unique in the world anymore and you should not spawn them!
    /// Before using the returned values in the world, first check that it is ok with [`EntityAllocator::has_remote_allocator`](super::EntityAllocator::has_remote_allocator).
    #[inline]
    pub fn alloc(&self) -> Entity {
        self.shared.remote_alloc()
    }

    /// Returns whether or not this [`RemoteAllocator`] is still connected to its source [`EntityAllocator`](super::EntityAllocator).
    ///
    /// Note that this could close immediately after the function returns false, so be careful.
    /// The best way to ensure that does not happen is to only trust the returned value while holding a reference to the world
    /// and to ensure it is the right world through [`EntityAllocator::has_remote_allocator`](super::EntityAllocator::has_remote_allocator).
    ///
    /// This is generally best used as a diagnostic.
    /// [`EntityAllocator::has_remote_allocator`](super::EntityAllocator::has_remote_allocator) is a better check for correctness.
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
