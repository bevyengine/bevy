use core::{
    num::Saturating,
    ops::{Deref, DerefMut},
};

use bevy_platform::cell::SyncCell;

use crate::{
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

/// A collection that can be `cleared` and there fore makes sense in a [`Scratch`].
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
