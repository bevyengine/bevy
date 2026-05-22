use alloc::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque},
    vec::Vec,
};
use core::{
    num::Saturating,
    ops::{Deref, DerefMut},
};

use bevy_platform::cell::SyncCell;

use core::hash::{BuildHasher, Hash};

#[cfg(feature = "std")]
use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};

use bevy_platform::collections::{HashMap, HashSet};

use crate::{
    entity::{EntityHashMap, EntityHashSet},
    system::{ExclusiveSystemParam, ReadOnlySystemParam, SystemParam},
    world::FromWorld,
};

// Same assumption as is used to calculate CHECK_TICK_THRESHOLD
const ASSUMED_TICKS_PER_FRAME: u32 = 1000;

// 1000 change ticks in a frame * 10 frames
const DEFAULT_SHRINK_DELAY: Saturating<u32> = Saturating(ASSUMED_TICKS_PER_FRAME * 10);

/// State used to construct a fetch the [`Scratch`] for a system.
pub struct ScratchState<T: ClearableCollection> {
    /// The collection returned from the [`Scratch`]
    data: SyncCell<T>,
    /// The capacity `data` had last time we resized it.
    target_capacity: usize,
    /// How many ticks until we resize. Gets reset if the struct expands
    shrink_delay: Saturating<u32>,
}

impl<T: ClearableCollection> ScratchState<T> {
    fn clear(&mut self) {
        self.data.get().clear();
    }

    /// Updates state based on how many ticks have occurred
    fn tick(&mut self, ticks: u32) {
        self.shrink_delay -= ticks;

        let data = self.data.get();
        let data_capacity = data.capacity();
        if data_capacity > self.target_capacity {
            self.target_capacity = data_capacity;
            self.shrink_delay = DEFAULT_SHRINK_DELAY;
            return;
        }

        if self.shrink_delay.0 == 0 {
            // TODO: Get the collections element size and fully free small allocations sooner.
            //       Right now it will take 3 updates to fully free a Vec<u8> that's empty but has
            //        a capacity of 8.
            data.shrink_to(self.target_capacity / 2);
            self.target_capacity = data.capacity();
            self.shrink_delay = DEFAULT_SHRINK_DELAY;
        }
    }
}

/// A [`Local`](crate::system::Local) like [`SystemParam`] for collections holding scratch data.
///
/// Collections are automatically cleared each time a system runs and
/// their capacity automatically shrunk if it hasn't increased in a while.
pub struct Scratch<'a, T: ClearableCollection>(&'a mut T);

impl<'a, T: ClearableCollection> Deref for Scratch<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: ClearableCollection> DerefMut for Scratch<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'s, 'a, T: ClearableCollection> IntoIterator for &'a Scratch<'s, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'s, 'a, T: ClearableCollection> IntoIterator for &'a mut Scratch<'s, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A clearable collection that can be used as a [`Scratch`] type.
pub trait ClearableCollection {
    /// The current capacity.
    fn capacity(&self) -> usize;
    /// Attempts to shrink the capacity of the collection.
    fn shrink_to(&mut self, capacity: usize);
    /// Removes all elements from a collection.
    fn clear(&mut self);
}

impl<'a, T: ClearableCollection + FromWorld + Send + 'static> ExclusiveSystemParam
    for Scratch<'a, T>
{
    type State = ScratchState<T>;
    type Item<'s> = Scratch<'s, T>;

    fn init(world: &mut crate::prelude::World, _: &mut super::SystemMeta) -> Self::State {
        let data = T::from_world(world);
        let target_capacity = data.capacity();
        let data = SyncCell::new(data);
        ScratchState {
            data,
            target_capacity,
            shrink_delay: DEFAULT_SHRINK_DELAY,
        }
    }

    fn get_param<'s>(
        state: &'s mut Self::State,
        _: &super::SystemMeta,
    ) -> Result<Self::Item<'s>, super::SystemParamValidationError> {
        state.clear();

        // We don't know how much time has passed. We could specialize the state
        // and store the ticks in there, but I don't think it's worth the effort.
        state.tick(ASSUMED_TICKS_PER_FRAME);

        // We can't use Self here since we're a Scratch<'a, _> not Scratch<'state, _>
        Ok(Scratch(state.data.get()))
    }
}

/// SAFETY: We only access world change ticks immutably. We can't conflict with anything.
unsafe impl<'a, T: ClearableCollection + FromWorld + Send + 'static> SystemParam
    for Scratch<'a, T>
{
    type State = ScratchState<T>;

    type Item<'world, 'state> = Scratch<'state, T>;

    fn init_state(world: &mut crate::prelude::World) -> Self::State {
        let data = T::from_world(world);
        let target_capacity = data.capacity();
        let data = SyncCell::new(data);
        ScratchState {
            data,
            target_capacity,
            shrink_delay: DEFAULT_SHRINK_DELAY,
        }
    }

    fn init_access(
        _: &Self::State,
        _: &mut super::SystemMeta,
        _: &mut crate::query::FilteredAccessSet,
        _: &mut crate::prelude::World,
    ) {
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &super::SystemMeta,
        _: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: crate::change_detection::Tick,
    ) -> Result<Self::Item<'world, 'state>, super::SystemParamValidationError> {
        state.clear();

        // There's no real ordering between ticks. That said this only breaks
        // at the first system run and when the change ticks overflow. Both events
        // are rare enough and the cost of getting it wrong is so low (we shrink the allocations)
        // that we just ignore it.
        let offset = change_tick.relative_to(system_meta.last_run);
        state.tick(offset.get());

        // We can't use Self here since we're a Scratch<'a, _> not Scratch<'state, _>
        Ok(Scratch(state.data.get()))
    }
}

/// SAFETY: We only access world change ticks immutably. We can't conflict with anything.
unsafe impl<'a, T: ClearableCollection + FromWorld + Send + 'static> ReadOnlySystemParam
    for Scratch<'a, T>
{
}

impl<T> ClearableCollection for Vec<T> {
    fn capacity(&self) -> usize {
        Vec::capacity(self)
    }

    fn clear(&mut self) {
        Vec::clear(self);
    }

    fn shrink_to(&mut self, capacity: usize) {
        Vec::shrink_to(self, capacity);
    }
}

impl<T> ClearableCollection for VecDeque<T> {
    fn capacity(&self) -> usize {
        VecDeque::capacity(self)
    }

    fn clear(&mut self) {
        VecDeque::clear(self);
    }

    fn shrink_to(&mut self, capacity: usize) {
        VecDeque::shrink_to(self, capacity);
    }
}

impl<I> ClearableCollection for BinaryHeap<I> {
    fn capacity(&self) -> usize {
        BinaryHeap::capacity(self)
    }

    fn shrink_to(&mut self, capacity: usize) {
        BinaryHeap::shrink_to(self, capacity);
    }

    fn clear(&mut self) {
        BinaryHeap::clear(self);
    }
}

#[cfg(feature = "std")]
impl<K: Hash + Eq, V, S: BuildHasher> ClearableCollection for StdHashMap<K, V, S> {
    fn capacity(&self) -> usize {
        StdHashMap::capacity(self)
    }

    fn clear(&mut self) {
        StdHashMap::clear(self);
    }

    fn shrink_to(&mut self, capacity: usize) {
        StdHashMap::shrink_to(self, capacity);
    }
}

#[cfg(feature = "std")]
impl<K: Hash + Eq, S: BuildHasher> ClearableCollection for StdHashSet<K, S> {
    fn capacity(&self) -> usize {
        StdHashSet::capacity(self)
    }

    fn clear(&mut self) {
        StdHashSet::clear(self);
    }

    fn shrink_to(&mut self, capacity: usize) {
        StdHashSet::shrink_to(self, capacity);
    }
}

impl<V> ClearableCollection for EntityHashMap<V> {
    fn capacity(&self) -> usize {
        self.deref().capacity()
    }

    fn clear(&mut self) {
        self.deref_mut().clear();
    }

    fn shrink_to(&mut self, capacity: usize) {
        self.deref_mut().shrink_to(capacity);
    }
}

impl ClearableCollection for EntityHashSet {
    fn capacity(&self) -> usize {
        self.deref().capacity()
    }

    fn clear(&mut self) {
        self.deref_mut().clear();
    }

    fn shrink_to(&mut self, capacity: usize) {
        self.deref_mut().shrink_to(capacity);
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> ClearableCollection for HashMap<K, V, S> {
    fn capacity(&self) -> usize {
        HashMap::capacity(self)
    }

    fn clear(&mut self) {
        HashMap::clear(self);
    }

    fn shrink_to(&mut self, capacity: usize) {
        HashMap::shrink_to(self, capacity);
    }
}

impl<K: Hash + Eq, S: BuildHasher> ClearableCollection for HashSet<K, S> {
    fn capacity(&self) -> usize {
        HashSet::capacity(self)
    }

    fn clear(&mut self) {
        HashSet::clear(self);
    }

    fn shrink_to(&mut self, capacity: usize) {
        HashSet::shrink_to(self, capacity);
    }
}

impl<I> ClearableCollection for LinkedList<I> {
    fn capacity(&self) -> usize {
        0
    }

    fn shrink_to(&mut self, _: usize) {}

    fn clear(&mut self) {
        LinkedList::clear(self);
    }
}

impl<K, V> ClearableCollection for BTreeMap<K, V> {
    fn capacity(&self) -> usize {
        0
    }

    fn shrink_to(&mut self, _: usize) {}

    fn clear(&mut self) {
        BTreeMap::clear(self);
    }
}

impl<I> ClearableCollection for BTreeSet<I> {
    fn capacity(&self) -> usize {
        0
    }

    fn shrink_to(&mut self, _: usize) {}

    fn clear(&mut self) {
        BTreeSet::clear(self);
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicU32, Ordering};

    use crate::{
        system::{
            scratch::{Scratch, DEFAULT_SHRINK_DELAY},
            Local,
        },
        world::World,
    };

    #[test]
    fn scratch_shrinks() {
        static UPDATE_COUNTER: AtomicU32 = AtomicU32::new(0);

        let mut world = World::new();
        world.increment_change_tick();

        fn check_system(
            mut item_under_test: Scratch<Vec<u32>>,
            mut expected_capacity: Local<usize>,
        ) {
            if UPDATE_COUNTER.load(Ordering::Relaxed) == 0 {
                item_under_test.reserve(512);
                // The vec may over-reserve space so we need to store the actual capacity.
                *expected_capacity = item_under_test.capacity();
            }

            if UPDATE_COUNTER.load(Ordering::Relaxed) == (DEFAULT_SHRINK_DELAY.0 + 1) {
                assert!(
                    item_under_test.capacity() < *expected_capacity,
                    "Scratch didn't shrink allocation, capacity was {}, expected {}, update was {}",
                    item_under_test.capacity(),
                    *expected_capacity,
                    UPDATE_COUNTER.load(Ordering::Relaxed),
                );
            }
        }

        let test_system = world.register_system(check_system);

        // The first tick breaks the time calculation logic. We wait it out. We also
        // have to wait one for the deallocation to actually happen.
        for _ in 0..=(DEFAULT_SHRINK_DELAY.0 + 2) {
            let result = world.run_system(test_system);

            assert!(result.is_ok(), "Test system failed to update {result:?}");
            UPDATE_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn scratch_clears() {
        let mut world = World::new();
        world.increment_change_tick();

        fn check_system(mut items: Scratch<Vec<u32>>) {
            assert!(items.is_empty());

            items.push(1);
        }

        let test_system = world.register_system(check_system);

        for _ in 0..=10 {
            let result = world.run_system(test_system);

            assert!(result.is_ok(), "Test system failed to update {result:?}");
        }
    }
}
