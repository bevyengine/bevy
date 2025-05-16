use crate::{
    bundle::{Bundle, BundleSpawner, NoBundleEffect},
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
    I::Item: Bundle,
{
    inner: I,
    spawner: BundleSpawner<'w>,
    caller: MaybeLocation,
    allocator: crate::entity::allocator::AllocEntitiesIterator<'static>,
}

impl<'w, I> SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    #[inline]
    #[track_caller]
    pub(crate) fn new(world: &'w mut World, iter: I, caller: MaybeLocation) -> Self {
        let change_tick = world.change_tick();

        let (lower, upper) = iter.size_hint();
        let length = upper.unwrap_or(lower);

        world.entities.prepare(length as u32);
        // SAFETY: We take the lifetime of the world, so the instance is valid.
        // `BundleSpawner::spawn_non_existent` never frees entities, and that is the only thing we call on it while the iterator is not empty.
        let allocator = unsafe { world.entities.alloc_entities_unsafe(lower as u32) };

        let mut spawner = BundleSpawner::new::<I::Item>(world, change_tick);
        spawner.reserve_storage(length);

        Self {
            inner: iter,
            spawner,
            caller,
            allocator,
        }
    }
}

impl<I> Drop for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
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
    I::Item: Bundle,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let bundle = self.inner.next()?;
        let entity = self.allocator.next();

        let spawned = match entity {
            // SAFETY: bundle matches spawner type. `entity` is fresh
            Some(entity) => unsafe {
                self.spawner.spawn_non_existent(entity, bundle, self.caller);
                entity
            },
            // SAFETY: bundle matches spawner type
            None => unsafe { self.spawner.spawn(bundle, self.caller).0 },
        };

        Some(spawned)
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

// SAFETY: Newly spawned entities are unique.
unsafe impl<I: Iterator, T> EntitySetIterator for SpawnBatchIter<'_, I>
where
    I: FusedIterator<Item = T>,
    T: Bundle,
{
}
