use crate::{
    bundle::{Bundle, BundleSpawner, NoBundleEffect, StaticBundle},
    change_detection::MaybeLocation,
    entity::{Entity, EntitySetIterator},
    world::World,
};
use core::iter::FusedIterator;

/// An iterator that spawns a series of entities and returns the [ID](Entity) of
/// each spawned entity.
///
/// If this iterator is not fully exhausted, any remaining entities will be spawned when this type is dropped.
pub struct SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle + StaticBundle,
{
    inner: I,
    spawner: BundleSpawner<'w>,
    caller: MaybeLocation,
}

impl<'w, I> SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle<Effect: NoBundleEffect> + StaticBundle,
{
    #[inline]
    #[track_caller]
    pub(crate) fn new(world: &'w mut World, iter: I, caller: MaybeLocation) -> Self {
        // Ensure all entity allocations are accounted for so `self.entities` can realloc if
        // necessary
        world.flush();

        let change_tick = world.change_tick();

        let (lower, upper) = iter.size_hint();
        let length = upper.unwrap_or(lower);
        world.entities.reserve(length as u32);

        let mut spawner = BundleSpawner::new_static::<I::Item>(world, change_tick);
        spawner.reserve_storage(length);

        Self {
            inner: iter,
            spawner,
            caller,
        }
    }
}

impl<I> Drop for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle + StaticBundle,
{
    fn drop(&mut self) {
        // Iterate through self in order to spawn remaining bundles.
        for _ in &mut *self {}
        // Apply any commands from those operations.
        // SAFETY: `self.spawner` will be dropped immediately after this call.
        unsafe { self.spawner.flush_commands() };
    }
}

impl<I> Iterator for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle + StaticBundle,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let bundle = self.inner.next()?;
        // SAFETY: bundle matches spawner type
        unsafe { Some(self.spawner.spawn(bundle, self.caller).0) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, T> ExactSizeIterator for SpawnBatchIter<'_, I>
where
    I: ExactSizeIterator<Item = T>,
    T: Bundle + StaticBundle,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<I, T> FusedIterator for SpawnBatchIter<'_, I>
where
    I: FusedIterator<Item = T>,
    T: Bundle + StaticBundle,
{
}

// SAFETY: Newly spawned entities are unique.
unsafe impl<I: Iterator, T> EntitySetIterator for SpawnBatchIter<'_, I>
where
    I: FusedIterator<Item = T>,
    T: Bundle + StaticBundle,
{
}
