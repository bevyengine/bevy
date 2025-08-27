use crate::{
    bundle::{Bundle, BundleSpawner, NoBundleEffect},
    change_detection::MaybeLocation,
    entity::{Entity, EntitySetIterator},
    world::World,
};
use core::iter::FusedIterator;
use core::mem::ManuallyDrop;

/// An iterator that spawns a series of entities and returns the [ID](Entity) of
/// each spawned entity.
///
/// If this iterator is not fully exhausted, any remaining entities will be spawned when this type is dropped.
pub struct SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    inner: I,
    spawner: BundleSpawner<'w>,
    caller: MaybeLocation,
}

impl<'w, I> SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle<Effect: NoBundleEffect>,
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

        let mut spawner = BundleSpawner::new::<I::Item>(world, change_tick);
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
    I::Item: Bundle<Effect: NoBundleEffect>,
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
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let mut bundle = ManuallyDrop::new(self.inner.next()?);
        let bundle_ptr = &raw mut bundle;
        // SAFETY:
        // - bundle matches spawner type
        // - `I::Item`'s effect type implements `NoBundleEffect`, there is no need to call
        //   `apply_effect`.
        unsafe {
            Some(
                self.spawner
                    .spawn(bundle_ptr.cast::<I::Item>(), self.caller),
            )
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, T> ExactSizeIterator for SpawnBatchIter<'_, I>
where
    I: ExactSizeIterator<Item = T>,
    T: Bundle<Effect: NoBundleEffect>,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<I, T> FusedIterator for SpawnBatchIter<'_, I>
where
    I: FusedIterator<Item = T>,
    T: Bundle<Effect: NoBundleEffect>,
{
}

// SAFETY: Newly spawned entities are unique.
unsafe impl<I: Iterator, T> EntitySetIterator for SpawnBatchIter<'_, I>
where
    I: FusedIterator<Item = T>,
    T: Bundle<Effect: NoBundleEffect>,
{
}
