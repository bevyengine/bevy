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
    ops::Range,
    sync::atomic::{AtomicI32, AtomicU64},
};
use crossbeam_utils::CachePadded;
use log::warn;
use nonmax::NonMaxU32;

/// A [`SharedDrain`] is a wrapper around a [`Vec`]. It ensures the capacity doesn't overflow `i32::MAX`.
/// It also facilitates unsafe swapping and reading of elements.
///
/// # Note
/// [`ManuallyDrop`] is not needed here as the capacity is always correct (unlike the len) and
/// [`Entity`] is [`Copy`] but it is included anyways for correctness.
#[derive(Default)]
struct SharedDrain(ManuallyDrop<Vec<Entity>>);

impl SharedDrain {
    /// Swaps the previous [`Vec`] as if it had been drained to `0` with the `other` [`Vec`].
    /// Swaps it with an empty [`Vec`] if this is the first time it's been called.
    ///
    /// Returns the initial length of the [`SharedDrain`] that should be used as a cursor.
    ///
    /// This function makes sure the length of the vector is bound to `i32::MAX`.
    ///
    /// # Safety
    /// - The previous [`Vec`] must have had all items in it [`SharedDrain::read`].
    #[inline]
    unsafe fn swap(&mut self, other: &mut Vec<Entity>) -> i32 {
        const MAX: usize = i32::MAX as usize;
        if other.len() > MAX {
            #[cold]
            fn drain(to: &mut Vec<Entity>, from: &mut Vec<Entity>) {
                to.extend(from.drain(MAX..));
            }
            drain(&mut self.0, other);
        }
        let initial_len = other.len() as i32;
        mem::swap(&mut *self.0, other);
        // SAFETY: The next time this function is called this length will be correct
        unsafe {
            self.0.set_len(0);
        }
        initial_len
    }

    /// Returns the value at `index` the `Vec`.
    ///
    /// # Safety
    /// - `index` must be in bounds.
    /// - each `index` must be called with this function exactly once.
    #[inline]
    unsafe fn read(&self, index: usize) -> Entity {
        // SAFETY: Ensured by caller
        unsafe { self.0.as_ptr().add(index).read() }
    }
}

/// Reserves items to be drained from a [`SharedDrain`] using [`AtomicHead::claim_as_producer`] and [`AtomicHead::claim_as_consumer`].
///
/// # Producer Optimization
/// Consumers normally decrement the [`Head::head`] cursor to claim items and decrement the
/// tail counter to release items. However when we are the producer ([`Allocator`]) we
/// don't need to release items because we would be releasing them to ourself see
/// [`SharedSwapDrain::pop_as_producer`] vs [`SharedSwapDrain::pop_as_consumer`]. Still,
/// we need to know how many items we're popped by the producer. In order to achieve this
/// we have two [`Head`]s: [`Head::head`] and [`Head::consumer_head`]. [`Head::head`] is
/// decremented by the producer AND the consumer, and [`Head::consumer_head`] is decremented only by the consumer.
/// We can then recreate the [`Head::producer_pop_count`] by subtracting [`Head::consumer_head`] from [`Head::head`].
///
/// To achieve this without significantly slowing down [`SharedSwapDrain::pop_as_consumer`], we pack both counters
/// into a single `u64` value so that a single RMW operation can be used to update both. (I specifically chose this
/// over having producer doubly increment so that the performance of [`SharedSwapDrain::pop_as_producer`] is not affected
/// on unsupported platforms. vvv)
///
/// # Compatibility
/// Although this is a [`AtomicU64`]. On unsupporting platforms we can easily split this into two [`AtomicI32`] values and
/// have [`SharedSwapDrain::pop_as_consumer`] update both atomically.
///
/// Keep in mind that the two heads [`Head::head`] and [`Head::consumer_head`] would not be decremented (together) atomically in this case
/// resulting in [`Head::producer_pop_count`] being incorrect when publishing. However, assuming we decrement the [`Head::consumer_head`]
/// first, we can at least ensure that [`Head::producer_pop_count`] is only ever incorrect in a direction where it won't result in UB.
/// Additionally, because we use [`Metadata`] to (loosely) avoid polling empty queues, this possible design results in a fallible but
/// eventually correct [`Head::producer_pop_count`] value.
///
/// # Layout
/// - 0..31 - `i32` head used to reserve indices
/// - 32..63 - `i32` consumer only head
///
/// The lower 32 bits freely borrow and carry into the upper 32 bits. We manually correct
/// this by adding `1` to the upper 32 bits when the lower 32 bits are negative.
/// Keep in mind that this _only_ works because [`Head::head`] is not allowed to wrap (`i32::MAX` <-> `i32::MIN`)
#[derive(Clone, Copy)]
struct Head(u64);

impl Head {
    #[inline]
    fn new(v: i32) -> Self {
        let v = v as u64;
        Self(v | (v << 32))
    }

    /// The current cursor position as if only consumers popped values.
    ///
    /// Only should be used in [`Head::producer_pop_count`].
    #[inline]
    fn consumer_head(self) -> i32 {
        let mut consumer_head = (self.0 >> 32) as i32;
        // Note: This only works because `head` is not allowed to wrap.
        if self.head() < 0 {
            consumer_head += 1;
        }
        consumer_head
    }

    /// The current cursor position
    #[inline]
    fn head(self) -> i32 {
        (self.0 & u32::MAX as u64) as i32
    }

    /// Returns the number of entities popped by the producer since the last [`Head::consumer_head`] update.
    #[inline]
    fn producer_pop_count(self) -> u32 {
        // This could technically wrap which is why we cast to `u32` (besides it being the
        // semantically correct option).
        self.consumer_head().wrapping_sub(self.head()) as u32
    }
}

/// [`Head`]
#[derive(Default)]
struct AtomicHead(AtomicU64);

impl AtomicHead {
    /// Write a new head value and release any prior written data to consumers.
    #[inline]
    fn publish(&self, head: Head) {
        // release matches with [`AtomicHead::claim_as_consumer`]
        self.0.store(head.0, Ordering::Release);
    }

    /// Claim `n` slots as a consumer and return the old head value.
    #[inline]
    fn claim_as_consumer(&self, n: u32) -> Head {
        let n = n as u64;
        let rhs = n | (n << 32);
        // acquire matches with [`AtomicHead::publish`]
        Head(self.0.fetch_sub(rhs, Ordering::Acquire))
    }

    /// Claim `n` slots as a producer and return the old head value.
    ///
    /// # Safety
    /// - must not be called synchronously with [`AtomicHead::publish`]
    #[inline]
    unsafe fn claim_as_producer(&self, n: u32) -> Head {
        // relaxed ordering due to safety requirements
        Head(self.0.fetch_sub(n as u64, Ordering::Relaxed))
    }
}

/// The [`AtomicTail`] counts how many remaining slots are unread by consumers
///
/// Although this value is an `i32` it is always non-negative because consumers
/// don't decrement it past zero.
///
/// When the producer is performing their own reads they elide the decrement to
/// tail. The actual tail must be reconstructed by subtracting the [`Head::producer_pop_count`].
#[derive(Default)]
struct AtomicTail(AtomicI32);

impl AtomicTail {
    #[inline]
    fn release_as_consumer(&self, n: u32) {
        // release matches with [`Tail::acquire`]
        self.0.fetch_sub(n as i32, Ordering::Release);
    }

    /// load the current tail and Acquire any reads the consumers released.
    ///
    /// This is used to reconstruct the actual tail value when publishing.
    #[inline]
    fn acquire(&self) -> i32 {
        // acquire matches with [`Tail::release_as_consumer`]
        self.0.load(Ordering::Acquire)
    }
}

/// A "View" of a [`SharedSwapDrain`] because it's internally stored as a SOA (see [`SharedFreeList`]).
///
/// This structure facilitates concurrent access to a [`SharedDrain`].
///
/// When the `actual_tail` (as summed by [`Head::producer_pop_count`] and [`AtomicTail::acquire`]) is `<= 0`, the drain is considered empty.
/// This is the only time that the producer is allowed to write data because we know that no consumer are
/// reading the `drain`.
/// Otherwise, the `drain` is under shared immutable access to all consumers and producers.
#[derive(Clone, Copy)]
struct SharedSwapDrain<'a> {
    drain: &'a SyncUnsafeCell<SharedDrain>,
    head: &'a AtomicHead,
    tail: &'a AtomicTail,
}

impl<'a> SharedSwapDrain<'a> {
    /// Pop an entity from the drain.
    ///
    /// # Safety
    /// - must not be called synchronously with [`SharedSwapDrain::try_publish`]
    #[inline]
    unsafe fn pop_as_producer(self, on_empty: impl Fn()) -> Option<Entity> {
        // SAFETY: [`SharedSwapDrain::publish`] which calls [`Head::publish`] is not being called
        let head = unsafe { self.head.claim_as_producer(1) };
        if head.head() < 1 {
            let _ = head.head().strict_sub(1);
            return None;
        }

        let index = (head.head() - 1) as usize;
        if index == 0 {
            on_empty();
        }

        // SAFETY: if `head` returns any positive value then drain is under immutable access
        let drain = unsafe { self.drain.get().as_ref_unchecked() };
        // SAFETY: all indices in ranges returned by head are unique and in-bounds
        let value = unsafe { drain.read(index) };

        Some(value)
    }

    /// Pop an entity from the drain as a consumer.
    #[inline]
    fn pop_as_consumer(self, on_empty: impl Fn()) -> Option<Entity> {
        let head = self.head.claim_as_consumer(1);
        if head.head() < 1 {
            let _ = head.head().strict_sub(1);
            return None;
        }

        let index = (head.head() - 1) as usize;
        if index == 0 {
            on_empty();
        }

        // SAFETY: if `head` returns any positive value then drain is under immutable access
        let drain = unsafe { self.drain.get().as_ref_unchecked() };
        // SAFETY: all indices in ranges returned by head are unique and in-bounds
        let value = unsafe { drain.read(index) };

        self.tail.release_as_consumer(1);

        Some(value)
    }

    /// # Safety
    /// - must not be called synchronously with [`SharedSwapDrain::try_publish`]
    #[inline]
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

    #[inline]
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
    /// - must not be called synchronously with any `*_as_producer`
    /// - must not be called concurrently with itself
    #[inline]
    unsafe fn try_publish(self, data: &mut Vec<Entity>) -> bool {
        let tail = self.tail.acquire();
        let head = Head(self.head.0.load(Ordering::Relaxed));
        if head.head() > 0 {
            return false;
        }
        // producer_pop_count cannot change while we have exclusive access to the producer (see docs
        // for [`Head`] for more info about 32 bit targets)
        let producer_pop_count = head.producer_pop_count();
        let (actual_tail, overflow) = tail.overflowing_sub_unsigned(producer_pop_count);

        if actual_tail > 0 || overflow {
            return false;
        }

        // SAFETY: Nobody is allowed to read from `drain` when `actual_tail <= 0`.
        let drain = unsafe { self.drain.get().as_mut_unchecked() };
        // SAFETY: We checked that `actual_tail <= 0` meaning all items have been claimed and read.
        let initial_len = unsafe { drain.swap(data) };

        // `tail` can be relaxed because `head` fences the publication
        // `tail` must be stored before `head` because `head` also fences `tail`.
        self.tail.0.store(initial_len, Ordering::Relaxed);
        self.head.publish(Head::new(initial_len));

        true
    }
}

#[inline]
fn clamp_to_positive(range: Range<i32>, on_empty: impl Fn()) -> Range<u32> {
    if range.start > 0 {
        range.start as u32..range.end as u32
    } else {
        if range.end > 0 {
            on_empty();
            0..range.end as u32
        } else {
            0..0
        }
    }
}

struct PopMany<'a> {
    swap: SharedSwapDrain<'a>,
    range: Range<u32>,
    popped_as_consumer: Option<NonZeroU32>,
}

impl<'a> PopMany<'a> {
    /// # Safety
    /// - if range yields elements `swap.drain` must be under shared access
    /// - all items yielded by range must be valid indices into `swap.drain`
    #[inline]
    unsafe fn new(
        swap: SharedSwapDrain<'a>,
        range: Range<u32>,
        popped_as_consumer: Option<NonZeroU32>,
    ) -> Self {
        Self {
            swap,
            range,
            popped_as_consumer,
        }
    }

    #[inline]
    fn empty(swap: SharedSwapDrain<'a>) -> Self {
        Self {
            swap,
            range: 0..0,
            popped_as_consumer: None,
        }
    }
}

impl<'a> Iterator for PopMany<'a> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.range.next()?;
        // SAFETY: ensured by construction with either [`PopMany::new`] or [`PopMany::empty`]
        let drain = unsafe { self.swap.drain.get().as_ref_unchecked() };
        let index = index as usize;
        // SAFETY: ensured by construction with either [`PopMany::new`] or [`PopMany::empty`]
        Some(unsafe { drain.read(index) })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<'a> ExactSizeIterator for PopMany<'a> {}
impl<'a> FusedIterator for PopMany<'a> {}

impl<'a> Drop for PopMany<'a> {
    #[inline]
    fn drop(&mut self) {
        // Note: Since [`Entity`] is [`Copy`] we don't actually have to do this.
        self.by_ref().for_each(drop);

        if let Some(popped) = self.popped_as_consumer {
            self.swap.tail.release_as_consumer(popped.get());
        }
    }
}

/// Determines which [`SharedSwapDrain`] to prioritize when popping elements. Additionally,
/// stores the empty state of both drains to short-circuit popping elements.
///
/// # Layout
/// - 0: `a_non_empty`
/// - 1: `b_non_empty`
/// - 2: priority (true -> b, false -> b) (see [`SharedFreeList::swaps`])
#[derive(Clone, Copy)]
struct Metadata(u32);

impl Metadata {
    /// Returns `which` [`SharedSwapDrain`] to prioritize when popping elements.
    #[inline]
    fn which_priority(self) -> bool {
        self.0 & 0b100 != 0
    }

    /// Returns `true` if both [`SharedSwapDrain`]s are empty.
    #[inline]
    fn are_empty(self) -> bool {
        self.0 & 0b11 == 0b00
    }

    /// Returns the number of non-empty elements in both [`SharedSwapDrain`]s.
    #[inline]
    fn non_empty_count(self) -> u32 {
        (self.0 & 0b11).count_ones()
    }
}

/// [`Metadata`]
///
/// All operations are [`Ordering::Relaxed`] because metadata only changes
/// behavior and does not guard data.
#[derive(Default)]
struct AtomicMetadata(AtomicU32);

impl AtomicMetadata {
    /// Loads the metadata value from the atomic variable.
    #[inline]
    fn load(&self) -> Metadata {
        Metadata(self.0.load(Ordering::Relaxed))
    }

    /// Call this when a drain becomes empty.
    ///
    /// Swaps the priority bit (see how [`SharedFreeList::try_publish`] handles this)
    /// and swaps the empty bit for the drain.
    ///
    /// It doesn't matter if these operations are reordered, as long as
    /// they are performed atomically. That's because we simply toggle state
    /// and n toggles in any order will eventually result in the correct state
    /// once they are all performed.
    ///
    /// What does matter is that these "Event" operations are only called
    /// once when the event happens.
    #[inline]
    fn on_drain_empty(&self, which: bool) {
        let bit_index = if which { 1 } else { 0 };
        let empty_mask = 1 << bit_index;
        let priority_mask = 0b100;
        self.0
            .fetch_xor(empty_mask | priority_mask, Ordering::Relaxed);
    }

    /// Called when a drain is swapped.
    ///
    /// Swaps the empty bit for the drain being swapped to.
    ///
    /// It doesn't matter if these operations are reordered, as long as
    /// they are performed atomically. That's because we simply toggle state
    /// and n toggles in any order will eventually result in the correct state
    /// once they are all performed.
    ///
    /// What does matter is that these "Event" operations are only called
    /// once when the event happens.
    #[inline]
    fn on_drain_swapped(&self, which: bool) {
        let bit_index = if which { 1 } else { 0 };
        let empty_mask = 1 << bit_index;
        self.0.fetch_xor(empty_mask, Ordering::Relaxed);
    }
}

/// Stored as an SOA so that I can put the [`SharedDrain`]s and [`AtomicMetadata`]
/// in the same cache line.
///
/// The other fields are hot RMW fields and so are padded to stop false sharing.
#[derive(Default)]
pub struct SharedFreeList {
    heads: [CachePadded<AtomicHead>; 2],
    tails: [CachePadded<AtomicTail>; 2],
    drains: [SyncUnsafeCell<SharedDrain>; 2],
    meta: AtomicMetadata,
}

impl SharedFreeList {
    /// Returns a [`SharedSwapDrain`] for the given `which`.
    #[inline]
    fn swap(&self, which: bool) -> SharedSwapDrain<'_> {
        let index = if which { 1 } else { 0 };
        SharedSwapDrain {
            drain: &self.drains[index],
            head: &self.heads[index],
            tail: &self.tails[index],
        }
    }

    /// Returns the [priority, other] array of [`SharedSwapDrain`]s for the given `priority`.
    #[inline]
    fn swaps(&self, which_priority: bool) -> [SharedSwapDrain<'_>; 2] {
        [self.swap(which_priority), self.swap(!which_priority)]
    }

    /// Pops an entity from the free list as a producer, returning it.
    ///
    /// # Safety
    /// - must not be called concurrently with [`SharedFreeList::try_publish`]
    #[inline]
    unsafe fn pop_as_producer(&self) -> Option<Entity> {
        let meta = self.meta.load();
        if meta.are_empty() {
            return None;
        }

        let which_priority = meta.which_priority();
        let [priority, other] = self.swaps(which_priority);

        // SAFETY: [`SharedSwapDrain::try_publish`] which calls [`SharedSwapDrain::try_publish`]
        // is not being called right now.
        unsafe {
            priority
                .pop_as_producer(|| self.meta.on_drain_empty(which_priority))
                .or_else(|| other.pop_as_producer(|| self.meta.on_drain_empty(!which_priority)))
        }
    }

    /// Pops an entity from the free list as a consumer, returning it.
    #[inline]
    fn pop_as_consumer(&self) -> Option<Entity> {
        let meta = self.meta.load();
        if meta.are_empty() {
            return None;
        }

        let which_priority = meta.which_priority();
        let [priority, other] = self.swaps(which_priority);

        priority
            .pop_as_consumer(|| self.meta.on_drain_empty(which_priority))
            .or_else(|| other.pop_as_consumer(|| self.meta.on_drain_empty(!which_priority)))
    }

    /// Pops `n` entities from the free list as a producer, returning them in a [`FlattenPopMany`].
    ///
    /// # Safety
    /// - must not be called concurrently with [`SharedFreeList::try_publish`]
    #[inline]
    unsafe fn pop_many_as_producer(&self, n: u32) -> FlattenPopMany<'_> {
        if n == 0 {
            return FlattenPopMany::empty(self.swap(false));
        }
        let meta = self.meta.load();
        if meta.are_empty() {
            return FlattenPopMany::empty(self.swap(false));
        }

        let which_priority = meta.which_priority();
        let [priority, other] = self.swaps(which_priority);

        // SAFETY: [`SharedSwapDrain::try_publish`] which calls [`SharedSwapDrain::try_publish`]
        // is not being called right now.
        unsafe {
            let a = priority.pop_many_as_producer(n, || self.meta.on_drain_empty(which_priority));
            let remaining = n - a.len() as u32;
            let b =
                other.pop_many_as_producer(remaining, || self.meta.on_drain_empty(!which_priority));
            FlattenPopMany::new([a, b])
        }
    }

    /// Pops `n` entities from the free list as a consumer, returning them in a [`FlattenPopMany`].
    #[inline]
    fn pop_many_as_consumer(&self, n: u32) -> FlattenPopMany<'_> {
        if n == 0 {
            return FlattenPopMany::empty(self.swap(false));
        }
        let meta = self.meta.load();
        if meta.are_empty() {
            return FlattenPopMany::empty(self.swap(false));
        }

        let which_priority = meta.which_priority();
        let [priority, other] = self.swaps(which_priority);

        let a = priority.pop_many_as_consumer(n, || self.meta.on_drain_empty(which_priority));
        let remaining = n - a.len() as u32;
        let b = other.pop_many_as_consumer(remaining, || self.meta.on_drain_empty(!which_priority));
        FlattenPopMany::new([a, b])
    }

    /// Tries to pull all of `data` into the free list by trying to swap it with a [`SharedDrain`].
    ///
    /// # Safety
    /// - must not be called concurrently with any `*_as_producer` function
    /// - must not be called concurrently with itself
    #[inline]
    unsafe fn try_publish(&self, data: &mut Vec<Entity>) {
        if data.is_empty() {
            return;
        }
        let meta = self.meta.load();

        let which_priority = meta.which_priority();
        let which = match meta.non_empty_count() {
            2 => return,
            // publish to the non-priority buffer which will be (or already has been) swapped to when
            // the current priority buffer empties
            1 => !which_priority,
            // publish to the priority buffer which has already been swapped to when the last buffer
            // was emptied
            0 => which_priority,
            _ => unreachable!(),
        };

        let swap = self.swap(which);
        // SAFETY: ensured by caller
        if unsafe { swap.try_publish(data) } {
            self.meta.on_drain_swapped(which);
        }
    }
}

impl Drop for SharedFreeList {
    #[inline]
    fn drop(&mut self) {
        for i in 0..2 {
            let head = Head(*self.heads[i].0.get_mut());
            let undrained_len = head.head().max(0) as usize;
            let drain = self.drains[i].get_mut();
            // SAFETY: `num_undrained_items` is the number of elements in `drain` that are still valid.
            // Note: Since [`Entity`] is [`Copy`] we don't actually have to do this.
            unsafe {
                drain.0.set_len(undrained_len);
                ManuallyDrop::drop(&mut drain.0);
            }
        }
    }
}

pub struct FlattenPopMany<'a> {
    pop_manys: [PopMany<'a>; 2],
    index: usize,
}

impl<'a> FlattenPopMany<'a> {
    #[inline]
    fn new(pop_manys: [PopMany<'a>; 2]) -> Self {
        Self {
            pop_manys,
            index: 0,
        }
    }

    #[inline]
    fn empty(swap: SharedSwapDrain<'a>) -> Self {
        Self {
            pop_manys: [PopMany::empty(swap), PopMany::empty(swap)],
            index: 2,
        }
    }
}

impl<'a> Iterator for FlattenPopMany<'a> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < 2 {
            if let Some(item) = self.pop_manys[self.index].next() {
                return Some(item);
            }
            self.index += 1;
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.pop_manys[0].len() + self.pop_manys[1].len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for FlattenPopMany<'a> {}
impl<'a> FusedIterator for FlattenPopMany<'a> {}

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
    #[inline]
    fn alloc_many(&self, count: u32) -> AllocUniqueEntityIndexIterator {
        if count == 0 {
            return AllocUniqueEntityIndexIterator(0..0);
        }
        let start_new = self.next_entity_index.fetch_add(count, Ordering::Relaxed);
        #[expect(
            clippy::absurd_extreme_comparisons,
            reason = "Self::MAX_ENTITIES may later be changed to not be equal to u32::MAX n some platforms"
        )]
        let new = match start_new
            .checked_add(count)
            .filter(|new| *new <= Self::MAX_ENTITIES)
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
pub(super) struct AllocUniqueEntityIndexIterator(Range<u32>);

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
    /// - must not be called at the same time as [`SharedAllocator::try_publish`]
    #[inline]
    unsafe fn alloc_as_producer(&self) -> Entity {
        // SAFETY: ensured by caller
        unsafe {
            self.free
                .pop_as_producer()
                .unwrap_or_else(|| self.fresh.alloc())
        }
    }

    /// Allocates `count` [`Entity`]s, reusing freed indices if they exist.
    ///
    /// # Safety
    /// - must not be called at the same time as [`SharedAllocator::try_publish`]
    #[inline]
    unsafe fn alloc_many_as_producer(&self, count: u32) -> AllocEntitiesIterator<'_> {
        // SAFETY: ensured by caller
        let reused = unsafe { self.free.pop_many_as_producer(count) };
        let still_need = count - reused.len() as u32;
        let new = self.fresh.alloc_many(still_need);
        AllocEntitiesIterator { new, reused }
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
    fn alloc_many_as_consumer(&self, count: u32) -> AllocEntitiesIterator<'_> {
        let reused = self.free.pop_many_as_consumer(count);
        let still_need = count - reused.len() as u32;
        let new = self.fresh.alloc_many(still_need);
        AllocEntitiesIterator { new, reused }
    }

    /// Attempts the publish the contents of `data` to the free list.
    ///
    /// # Safety
    /// - must not be called at the same time as [`Self::alloc_as_producer`] or [`Self::alloc_many_as_producer`]
    #[inline]
    unsafe fn try_publish(&self, data: &mut Vec<Entity>) {
        // SAFETY: ensured by caller
        unsafe {
            self.free.try_publish(data);
        }
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
/// The allocator assumes that it is the only one with `*_as_producer` and `try_publish` permissions.
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
        // SAFETY: we have &self therefore [`Allocator::flush`] cannot be called right now
        unsafe { self.shared.alloc_as_producer() }
    }

    /// Allocates `count` entities in an iterator.
    #[inline]
    pub(super) fn alloc_many(&self, count: u32) -> AllocEntitiesIterator<'_> {
        // SAFETY: we have &self therefore [`Allocator::flush`] cannot be called right now
        unsafe { self.shared.alloc_many_as_producer(count) }
    }

    /// Synchronizes the local free list with the shared free list.
    #[inline]
    pub(crate) fn flush(&mut self) {
        // SAFETY: we have &mut self therefore [`Allocator::alloc_many`] and [`Allocator::alloc`] and [`Allocator::try_publish`]cannot be called right now
        unsafe { self.shared.try_publish(&mut self.local) };
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
    }

    /// Frees the entity allowing it to be reused.
    #[inline]
    pub(super) fn free(&mut self, entity: Entity) {
        self.local.push(entity);
    }

    /// Frees the entities allowing them to be reused.
    #[inline]
    pub(super) fn free_many(&mut self, entities: &[Entity]) {
        self.local.extend_from_slice(entities);
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
    reused: FlattenPopMany<'a>,
    new: AllocUniqueEntityIndexIterator,
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
impl<'a> FusedIterator for AllocEntitiesIterator<'a> {}

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
    pub fn alloc_many(&self, count: u32) -> AllocEntitiesIterator<'_> {
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
