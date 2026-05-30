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

use super::{Entity, EntityIndex, EntitySetIterator};
use bevy_platform::{
    cell::SyncUnsafeCell,
    prelude::Vec,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};
use core::{
    iter::FusedIterator,
    mem::{self, ManuallyDrop},
    num::NonZeroU32,
    range::{Range, RangeIter},
    sync::atomic::{AtomicI32, AtomicU64},
};
use crossbeam_utils::CachePadded;
use log::warn;
use nonmax::NonMaxU32;

#[derive(Default)]
struct SharedDrain(ManuallyDrop<Vec<Entity>>);

impl SharedDrain {
    /// Swaps the previous `Vec` as if it had been drained to `0` with the `other` `Vec`.
    /// Swaps it with an empty `Vec` if this is the first time it's been called.
    ///
    /// This function makes sure the length of the vector is bound to `i32::MAX`.
    ///
    /// # Safety
    /// - The previous `Vec` must have a length of `0`
    unsafe fn swap(&mut self, other: &mut Vec<Entity>) {
        const MAX: usize = i32::MAX as usize;
        if other.len() > MAX {
            #[cold]
            fn drain(to: &mut Vec<Entity>, from: &mut Vec<Entity>) {
                to.extend(from.drain(MAX..));
            }
            drain(&mut *self.0, other);
        }
        mem::swap(&mut *self.0, other);
        // SAFETY: The next time this function is called this length will be correct
        unsafe {
            self.0.set_len(0);
        }
    }

    /// Returns the value at `index` the `Vec`.
    ///
    /// # Safety
    /// - `index` must be in bounds.
    /// - each `index` must be called with this function exactly once.
    unsafe fn read(&self, index: usize) -> Entity {
        unsafe { self.0.as_ptr().add(index).read() }
    }

    fn initial_len(&self) -> i32 {
        // see [`SharedDrain::swap`]
        self.0.len() as i32
    }
}

/// Reserves items to be drained from a [`SharedDrain`] using [`AtomicHead::claim_as_*`].
///
/// # Layout
/// - 0..31 - `i32` head used to reserve indices
/// - 32..63 - `i32` consumer only head
///
/// The lower 32 bits freely borrow and carry into the upper 32 bits. We only ever
/// need to read the upper 32 bits when the lower 32 bits are negative (and also the
/// lower 32 bits aren't allowed to wrap from negative <-> positive) therefore we can
/// extract the actual value from the upper 32 bits by adding 1 when the lower 32 bits
/// are negative.
#[derive(Clone, Copy)]
struct Head(u64);

impl Default for Head {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Head {
    fn new(v: i32) -> Self {
        let v = v as u64;
        Self(v | (v << 32))
    }

    fn consumer_head(self) -> i32 {
        let mut consumer_head = ((self.0 as i64) >> 32) as i32;
        // Note: This only works because `head` is not allowed to wrap
        if self.head() < 0 {
            consumer_head += 1;
        }
        consumer_head
    }

    fn head(self) -> i32 {
        (self.0 & u32::MAX as u64) as i32
    }

    /// We only actually need the value if the head is < 0
    fn producer_pop_count(self) -> i32 {
        self.head() - self.consumer_head()
    }
}

/// [`Head`]
#[derive(Default)]
struct AtomicHead(AtomicU64);

impl AtomicHead {
    fn publish(&self, head: Head) {
        // release matches with [`AtomicHead::claim_as_consumer`]
        self.0.store(head.0, Ordering::Release);
    }

    fn claim_as_consumer(&self, n: u32) -> Head {
        let n = n as u64;
        let rhs = n | (n << 32);
        // acquire matches with [`AtomicHead::publish`]
        Head(self.0.fetch_sub(rhs, Ordering::Acquire))
    }

    /// # Safety
    /// - must not be called syncronously with [`AtomicHead::publish`]
    unsafe fn claim_as_producer(&self, n: u32) -> Head {
        // relaxed ordering due to safety requirements
        Head(self.0.fetch_sub(n as u64, Ordering::Relaxed))
    }
}

/// The `Tail` tracks how many remaining slots still need to be drained
/// in order for the producer to have exclusive access to the buffer.
///
/// Although this value is an `i32` it is always non-negative.
///
/// When the producer is performing their own reads they elide the decriment to
/// tail. The actual tail must be reconstructed when publishing.
#[derive(Default)]
struct AtomicTail(AtomicI32);

impl AtomicTail {
    fn release_as_consumer(&self, n: u32) {
        // release matches with [`Tail::acquire`]
        self.0.fetch_sub(n as i32, Ordering::Release);
    }

    fn acquire(&self) -> i32 {
        // acquire matches with [`Tail::release_as_consumer`]
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy)]
struct SharedSwapDrain<'a> {
    drain: &'a SyncUnsafeCell<SharedDrain>,
    head: &'a AtomicHead,
    tail: &'a AtomicTail,
}

impl<'a> SharedSwapDrain<'a> {
    /// # Safety
    /// - must not be called syncronously with [`SharedSwapDrain::publish`]
    unsafe fn pop_as_producer(self, on_empty: impl Fn()) -> Option<Entity> {
        // SAFETY: [`SharedSwapDrain::publish`] which calls [`Head::publish`] is not being called
        let head = unsafe { self.head.claim_as_producer(1) };
        if head.head() < 1 {
            let _ = head.head().strict_sub(1);
            return None;
        }

        let index = (head.head() - 1) as usize;
        if index == 0 {
            on_empty()
        }

        // SAFETY: [`SharedSwapDrain::publish`], which mutates this value, is not being called
        let drain = unsafe { self.drain.get().as_ref_unchecked() };
        // SAFETY: all indices in ranges returned by head are unique and in-bounds
        let value = unsafe { drain.read(index) };

        Some(value)
    }

    fn pop_as_consumer(self, on_empty: impl Fn()) -> Option<Entity> {
        let head = self.head.claim_as_consumer(1);
        if head.head() < 1 {
            let _ = head.head().strict_sub(1);
            return None;
        }

        let index = (head.head() - 1) as usize;
        if index == 0 {
            on_empty()
        }

        // SAFETY: if `head` returns any positive value then consumers have immutable access to drain
        let drain = unsafe { self.drain.get().as_ref_unchecked() };
        // SAFETY: all indices in ranges returned by head are unique and in-bounds
        let value = unsafe { drain.read(index) };

        self.tail.release_as_consumer(1);

        Some(value)
    }

    /// # Safety
    /// - must not be called syncronously with [`SharedSwapDrain::publish`]
    unsafe fn pop_many_as_producer(self, n: u32, on_empty: impl Fn()) -> PopMany<'a> {
        if n == 0 {
            return PopMany::empty(self);
        }

        // SAFETY: [`SharedSwapDrain::publish`] which calls [`Head::publish`] is not being called
        let head = unsafe { self.head.claim_as_producer(n) };

        let end = head.head();
        let start = end.strict_sub_unsigned(n);
        let range = clamp_to_positive(Range { start, end }, on_empty);

        // SAFETY: negative ranges will yield zero elements and positive ranges mean that `drain` is under shared access
        unsafe { PopMany::new(self, range.into_iter(), None) }
    }

    fn pop_many_as_consumer(self, n: u32, on_empty: impl Fn()) -> PopMany<'a> {
        if n == 0 {
            return PopMany::empty(self);
        }

        let head = self.head.claim_as_consumer(n);

        let end = head.head();
        let start = end.strict_sub_unsigned(n);
        let range = clamp_to_positive(Range { start, end }, on_empty);

        let popped_as_consumer = NonZeroU32::new(range.end - range.start);

        // SAFETY: negative ranges will yield zero elements and positive ranges mean that `drain` is under shared access
        unsafe { PopMany::new(self, range.into_iter(), popped_as_consumer) }
    }

    /// # Safety
    /// - must not be called syncronously with any [`SharedSwapDrain::*_as_producer`]
    unsafe fn try_publish(self, data: &mut Vec<Entity>) -> bool {
        let tail = self.tail.acquire();
        // producer_pop_count cannot change while we have exclusive access to the producer
        let producer_pop_count = Head(self.head.0.load(Ordering::Relaxed)).producer_pop_count();
        let actual_tail = tail - producer_pop_count;

        if actual_tail <= 0 {
            return false;
        }

        // SAFETY: Nobody is allowed to read from `drain` when `actual_tail <= 0`.
        let drain = unsafe { self.drain.get().as_mut_unchecked() };
        // SAFETY: We checked that `actual_tail < 0` meaning all items have been drained
        unsafe {
            drain.swap(data);
        }

        let initial_len = drain.initial_len() as i32;

        // `tail` can be relaxed because `head` fences the publication
        // `tail` must be stored before `head`
        self.tail.0.store(initial_len, Ordering::Relaxed);
        self.head.publish(Head::new(initial_len));

        true
    }
}

fn clamp_to_positive(range: Range<i32>, on_empty: impl Fn()) -> Range<u32> {
    // `<=` makes sure we also call `on_empty` on ranges to `0` not just past `0`
    if range.start <= 0 {
        if range.end > 0 {
            on_empty();
            Range {
                start: 0,
                end: range.end as u32,
            }
        } else {
            Range { start: 0, end: 0 }
        }
    } else {
        Range {
            start: range.start as u32,
            end: range.end as u32,
        }
    }
}

struct PopMany<'a> {
    swap: SharedSwapDrain<'a>,
    range: RangeIter<u32>,
    popped_as_consumer: Option<NonZeroU32>,
}

impl<'a> PopMany<'a> {
    /// # Safety
    /// - if range yields elements `swap.drain` must be under shared access
    unsafe fn new(
        swap: SharedSwapDrain<'a>,
        range: RangeIter<u32>,
        popped_as_consumer: Option<NonZeroU32>,
    ) -> Self {
        Self {
            swap,
            range,
            popped_as_consumer,
        }
    }

    fn empty(swap: SharedSwapDrain<'a>) -> Self {
        Self {
            swap,
            range: Range::default().into_iter(),
            popped_as_consumer: None,
        }
    }
}

impl<'a> Iterator for PopMany<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(index) = self.range.next() else {
            return None;
        };
        // SAFETY: ensured by construction with either [`PopMany::new`] or [`PopMany::empty`]
        let drain = unsafe { self.swap.drain.get().as_ref_unchecked() };
        let index = index as usize;
        // SAFETY: all indices in ranges returned by head are unique and in-bounds
        Some(unsafe { drain.read(index) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<'a> ExactSizeIterator for PopMany<'a> {}
impl<'a> FusedIterator for PopMany<'a> {}

impl<'a> Drop for PopMany<'a> {
    fn drop(&mut self) {
        // Note: Since [`Entity`] is [`Copy`] we don't actually have to do this.
        self.by_ref().for_each(drop);

        if let Some(popped) = self.popped_as_consumer {
            self.swap.tail.release_as_consumer(popped.get());
        }
    }
}

/// # Layout
/// - 0: a_empty
/// - 1: b_empty
/// - 2: priority (true -> b, false -> b) (see [`SharedFreeList::swaps`])
#[derive(Clone, Copy)]
struct Metadata(u32);

impl Metadata {
    fn priority(self) -> bool {
        self.0 & 0b100 != 0
    }

    fn are_empty(self) -> bool {
        self.0 & 0b11 == 0b11
    }

    fn non_empty_count(self) -> u32 {
        (self.0 & 0b11).count_zeros()
    }
}

/// [`Metadata`]
///
/// All operations are [`Ordering::Relaxed`] because metadata only changes
/// behavior and does not guard data.
#[derive(Default)]
struct AtomicMetadata(AtomicU32);

impl AtomicMetadata {
    fn load(&self) -> Metadata {
        Metadata(self.0.load(Ordering::Relaxed))
    }

    fn set_empty(&self, which: bool) {
        let bit_index = if which { 1 } else { 0 };
        let empty_mask = 1 << bit_index;
        let priority_mask = 0b100;
        // We can do a checkless toggle because we that we only
        // toggle these flags once on each event therefore even if
        // we don't check the state, the state will be correct OR
        // pending correct (correctness influences heuristics but not
        // soundness so it's only loosely guarded)
        self.0
            .fetch_xor(empty_mask | priority_mask, Ordering::Relaxed);
    }

    // We don't swap the priority in this case. The priority is only swapped
    // on emptying.
    fn set_non_empty(&self, which: bool) {
        let bit_index = if which { 1 } else { 0 };
        let empty_mask = 1 << bit_index;
        self.0.fetch_xor(empty_mask, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub struct SharedFreeList {
    heads: [CachePadded<AtomicHead>; 2],
    tails: [CachePadded<AtomicTail>; 2],
    drains: [SyncUnsafeCell<SharedDrain>; 2],
    meta: AtomicMetadata,
}

impl SharedFreeList {
    #[inline]
    fn swaps(&self, priority: bool) -> [SharedSwapDrain<'_>; 2] {
        let a = SharedSwapDrain {
            drain: &self.drains[0],
            head: &self.heads[0],
            tail: &self.tails[0],
        };
        let b = SharedSwapDrain {
            drain: &self.drains[1],
            head: &self.heads[1],
            tail: &self.tails[1],
        };
        if priority {
            [b, a]
        } else {
            [a, b]
        }
    }

    /// # Safety
    /// - Must
    unsafe fn pop_as_producer(&self) -> Option<Entity> {
        let meta = self.meta.load();
        if meta.are_empty() {
            return None;
        }

        let which = meta.priority();
        let [priority, other] = self.swaps(which);

        // SAFETY: todo
        unsafe {
            priority
                .pop_as_producer(|| self.meta.set_empty(which))
                .or_else(|| other.pop_as_producer(|| self.meta.set_empty(!which)))
        }
    }

    fn pop_as_consumer(&self) -> Option<Entity> {
        let meta = self.meta.load();
        if meta.are_empty() {
            return None;
        }

        let which = meta.priority();
        let [priority, other] = self.swaps(which);

        priority
            .pop_as_consumer(|| self.meta.set_empty(which))
            .or_else(|| other.pop_as_consumer(|| self.meta.set_empty(!which)))
    }

    /// # Safety
    ///
    unsafe fn pop_many_as_producer(&self, n: u32) -> ChainPopMany<'_> {
        let empty = ChainPopMany::empty(self.swaps(false)[0]);
        if n == 0 {
            return empty;
        }
        let meta = self.meta.load();
        if meta.are_empty() {
            return empty;
        }

        let which = meta.priority();
        let [priority, other] = self.swaps(which);

        // SAFETY: todo
        unsafe {
            let a = priority.pop_many_as_producer(n, || self.meta.set_empty(which));
            let remaining = n - a.len() as u32;
            let b = other.pop_many_as_producer(remaining, || self.meta.set_empty(!which));
            ChainPopMany { a, b }
        }
    }

    fn pop_many_as_consumer(&self, n: u32) -> ChainPopMany<'_> {
        let empty = ChainPopMany::empty(self.swaps(false)[0]);
        if n == 0 {
            return empty;
        }
        let meta = self.meta.load();
        if meta.are_empty() {
            return empty;
        }

        let which = meta.priority();
        let [priority, other] = self.swaps(which);

        let a = priority.pop_many_as_consumer(n, || self.meta.set_empty(which));
        let remaining = n - a.len() as u32;
        let b = other.pop_many_as_consumer(remaining, || self.meta.set_empty(!which));
        ChainPopMany { a, b }
    }

    /// # Safety
    pub unsafe fn try_publish(&self, data: &mut Vec<Entity>) {
        if data.len() == 0 {
            return;
        }
        let meta = self.meta.load();

        let which = meta.priority();
        let publish_to = match meta.non_empty_count() {
            // nowhere to publish
            2 => return,
            // publish to the non-priority buffer which will be (or already has been) swapped to when the current
            // buffer empties
            1 => !which,
            // publish to the priority buffer which has already been swapped to when the last buffer
            // was emptied
            0 => which,
            _ => unreachable!(),
        };

        let [swap, _] = self.swaps(publish_to);
        if unsafe { swap.try_publish(data) } {
            self.meta.set_non_empty(publish_to);
        }
    }
}

impl Drop for SharedFreeList {
    fn drop(&mut self) {
        for i in 0..2 {
            // Since we have &mut self we know that `head.max(0) == actual_tail` (actual_tail as computed)
            let head = Head(*self.heads[i].0.get_mut());
            let undrained_len = head.head().max(0) as usize;

            let drain = self.drains[i].get_mut();
            // SAFETY: `num_undrained_items` is the number of elements in `drain` that are still valid.
            //
            // Note: Since [`Entity`] is [`Copy`] we don't actually have to do this.
            unsafe {
                drain.0.set_len(undrained_len);
                ManuallyDrop::drop(&mut drain.0);
            }
        }
    }
}

pub struct ChainPopMany<'a> {
    a: PopMany<'a>,
    b: PopMany<'a>,
}

impl<'a> ChainPopMany<'a> {
    fn empty(swap: SharedSwapDrain<'a>) -> Self {
        Self {
            a: PopMany::empty(swap),
            b: PopMany::empty(swap),
        }
    }
}

impl<'a> Iterator for ChainPopMany<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.a.next().or_else(|| self.b.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.a.len() + self.b.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for ChainPopMany<'a> {}
impl<'a> FusedIterator for ChainPopMany<'a> {}

#[derive(Default)]
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
        if count == 0 {
            return AllocUniqueEntityIndexIterator(0..0);
        }
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
impl FusedIterator for AllocUniqueEntityIndexIterator {}

/// This stores allocation data shared by all entity allocators.
#[derive(Default)]
struct SharedAllocator {
    /// The entities pending reuse
    free: SharedFreeList,
    fresh: FreshAllocator,
    /// Tracks whether or not the primary [`Allocator`] has been closed or not.
    is_closed: AtomicBool,
}

impl SharedAllocator {
    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    ///
    /// # Safety
    /// - You must be the single producer to call this function.
    #[inline]
    unsafe fn alloc_as_producer(&self) -> Entity {
        unsafe {
            self.free
                .pop_as_producer()
                .unwrap_or_else(|| self.fresh.alloc())
        }
    }

    /// Allocates a `count` [`Entity`]s, reusing freed indices if they exist.
    ///
    /// # Safety
    /// - You must be the single producer to call this function.
    #[inline]
    unsafe fn alloc_many_as_producer(&self, count: u32) -> AllocEntitiesiter<'_> {
        let reused = unsafe { self.free.pop_many_as_producer(count) };
        let still_need = count - reused.len() as u32;
        let new = self.fresh.alloc_many(still_need);
        AllocEntitiesiter { new, reused }
    }

    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    #[inline]
    fn alloc_as_consumer(&self) -> Entity {
        self.free
            .pop_as_consumer()
            .unwrap_or_else(|| self.fresh.alloc())
    }

    /// Allocates a `count` [`Entity`]s, reusing freed indices if they exist.
    #[inline]
    fn alloc_many_as_consumer(&self, count: u32) -> AllocEntitiesiter<'_> {
        let reused = unsafe { self.free.pop_many_as_producer(count) };
        let still_need = count - reused.len() as u32;
        let new = self.fresh.alloc_many(still_need);
        AllocEntitiesiter { new, reused }
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
#[derive(Default)]
pub(crate) struct Allocator {
    /// The shared allocator state, which we share with any [`RemoteAllocator`]s.
    shared: Arc<SharedAllocator>,
    /// The local free list. All local operations go through this.
    /// In order to share items with [`RemoteAllocator`]s, we swap this with
    /// other `Vec`s.
    local: Vec<Entity>,
}

impl Allocator {
    /// Allocates a new [`Entity`], reusing a freed index if one exists.
    #[inline]
    pub(super) fn alloc(&self) -> Entity {
        // SAFETY: we have &self, so we are the single producer. also no Clone
        unsafe { self.shared.alloc_as_producer() }
    }

    /// Allocates `count` entities in an iterator.
    #[inline]
    pub(super) fn alloc_many(&self, count: u32) -> AllocEntitiesiter<'_> {
        // SAFETY: we have &self, so we are the single producer. also no Clone
        unsafe { self.shared.alloc_many_as_producer(count) }
    }

    /// The total number of indices given out.
    #[inline]
    pub(crate) fn total_entity_indices(&self) -> u32 {
        self.shared.fresh.total_entity_indices()
    }

    /// The number of free entities.
    #[inline]
    fn num_free(&self) -> u32 {
        todo!()
        // Safety: The `Allocator` is the single producer for `SharedFreeList`.
        // unsafe { self.shared.free.len() }
    }

    /// Synchronizes the local free list with the shared free list.
    pub(crate) fn flush(&mut self) {
        // Safety: The `Allocator` is the single producer for `SharedFreeList`.
        unsafe { self.shared.free.try_publish(&mut self.local) };
    }

    /// Frees the entity allowing it to be reused.
    #[inline]
    pub(super) fn free(&mut self, entity: Entity) {
        self.local.push(entity);
    }

    /// Frees the entities allowing them to be reused.
    #[inline]
    pub(super) fn free_many(&mut self, entities: &[Entity]) {
        self.local.extend_from_slice(entities)
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
pub(crate) struct AllocEntitiesiter<'a> {
    reused: ChainPopMany<'a>,
    new: AllocUniqueEntityIndexIterator,
}

impl<'a> Iterator for AllocEntitiesiter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.reused.next().or_else(|| self.new.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.reused.len() + self.new.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for AllocEntitiesiter<'a> {}
impl<'a> FusedIterator for AllocEntitiesiter<'a> {}

// SAFETY: Newly reserved entity values are unique.
unsafe impl EntitySetIterator for AllocEntitiesiter<'_> {}

impl Drop for AllocEntitiesiter<'_> {
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
/// As a result, using this will be slower than the main allocator but this offers additional freedoms.
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
        self.shared.alloc_as_consumer()
    }

    /// Allocates a batch of entities remotely.
    ///
    /// This comes with a major downside:
    /// Because this does not hold reference to the world, the world may be cleared or destroyed before you get a chance to use the result.
    /// If that happens, these entities will be garbage!
    /// They will not be unique in the world anymore and you should not spawn them!
    /// Before using the returned values in the world, first check that it is ok with [`EntityAllocator::has_remote_allocator`](super::EntityAllocator::has_remote_allocator).
    #[inline]
    pub(crate) fn alloc_many(&self, count: u32) -> AllocEntitiesiter<'_> {
        self.shared.alloc_many_as_consumer(count)
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

    #[test]
    fn uniqueness() {
        let mut entities = Vec::with_capacity(2000);
        let mut allocator = Allocator::default();
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

    /// Bevy's allocator doesn't make guarantees about what order entities will be allocated in.
    /// This test just exists to make sure allocations don't step on each other's toes.
    #[test]
    fn allocation_order_correctness() {
        let mut allocator = Allocator::default();
        let e0 = allocator.alloc();
        let e1 = allocator.alloc();
        let e2 = allocator.alloc();
        let e3 = allocator.alloc();
        allocator.free(e0);
        allocator.free(e1);
        allocator.free(e2);
        allocator.free(e3);
        allocator.flush();

        let r0 = allocator.alloc();
        let mut many = allocator.alloc_many(2);
        let r1 = many.next().unwrap();
        let r2 = many.next().unwrap();
        assert!(many.next().is_none());
        drop(many);
        let r3 = allocator.alloc();

        assert_eq!(r0, e3);
        assert_eq!(r1, e1);
        assert_eq!(r2, e2);
        assert_eq!(r3, e0);
    }
}
