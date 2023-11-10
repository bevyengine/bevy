use crate::{
    bundle::{Bundle, BundleSpawner},
    entity::Entity,
    world::World,
};
use std::iter::FusedIterator;

/// An iterator that spawns a series of entities and returns the [ID](Entity) of
/// each spawned entity.
///
/// If this iterator is not fully exhausted, any remaining entities will be spawned when this type is dropped.
pub struct SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    inner: I,
    spawner: BundleSpawner<'w, 'w>,
}

impl<'w, I> SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    #[inline]
    pub(crate) fn new(world: &'w mut World, iter: I) -> Self {
        // Ensure all entity allocations are accounted for so `self.entities` can realloc if
        // necessary
        world.flush();

        let change_tick = world.change_tick();

        let (lower, upper) = iter.size_hint();
        let length = upper.unwrap_or(lower);

        let bundle_info = world
            .bundles
            .init_info::<I::Item>(&mut world.components, &mut world.storages);
        world.entities.reserve(length as u32);
        let mut spawner = bundle_info.get_bundle_spawner(
            &mut world.entities,
            &mut world.archetypes,
            &world.components,
            &mut world.storages,
            change_tick,
        );
        spawner.reserve_storage(length);

        Self {
            inner: iter,
            spawner,
        }
    }
}

impl<I> Drop for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    fn drop(&mut self) {
        for _ in self {}
    }
}

impl<I> Iterator for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let bundle = self.inner.next()?;
        // SAFETY: bundle matches spawner type
        unsafe { Some(self.spawner.spawn(bundle)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, T> ExactSizeIterator for SpawnBatchIter<'_, I>
where
    I: ExactSizeIterator<Item = T>,
    T: Bundle,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<I, T> FusedIterator for SpawnBatchIter<'_, I>
where
    I: FusedIterator<Item = T>,
    T: Bundle,
{
}
