use bevy_ptr::move_as_ptr;

use crate::{
    bundle::{Bundle, BundleSpawner, NoBundleEffect},
    change_detection::MaybeLocation,
    entity::{AllocEntitiesIterator, Entity, EntitySetIterator},
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
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    inner: I,
    spawner: Option<BundleSpawner<'w>>,
    allocator: Option<AllocEntitiesIterator<'w>>,
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
        let change_tick = world.change_tick();

        let (lower, upper) = iter.size_hint();
        let length = upper.unwrap_or(lower);

        match BundleSpawner::new::<I::Item>(world, change_tick) {
            Ok(mut spawner) => {
                spawner.reserve_storage(length);
                let allocator = spawner.allocator().alloc_many(length as u32);
                Self {
                    inner: iter,
                    spawner: Some(spawner),
                    allocator: Some(allocator),
                    caller,
                }
            }
            Err(_) => Self {
                inner: iter,
                spawner: None,
                allocator: None,
                caller,
            },
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
        // Free all the over allocated entities.
        if let Some(ref mut allocator) = self.allocator {
            for e in allocator.by_ref() {
                self.spawner.as_mut().unwrap().allocator().free(e);
            }
        }
        // Apply any commands from those operations.
        if let Some(ref mut spawner) = self.spawner {
            // SAFETY: `self.spawner` will be dropped immediately after this call.
            unsafe { spawner.flush_commands() };
        }
    }
}

impl<I> Iterator for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let bundle = self.inner.next()?;
        let spawner = self.spawner.as_mut()?;
        move_as_ptr!(bundle);
        Some(
            if let Some(allocator) = &mut self.allocator
                && let Some(bulk) = allocator.next()
            {
                // SAFETY: bundle matches spawner type and we just allocated it
                unsafe {
                    spawner.spawn_at(bulk, bundle, self.caller);
                }
                bulk
            } else {
                // SAFETY: bundle matches spawner type
                unsafe { spawner.spawn(bundle, self.caller) }
            },
        )
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.spawner.is_none() {
            return (0, Some(0));
        }
        self.inner.size_hint()
    }
}

impl<I, T> ExactSizeIterator for SpawnBatchIter<'_, I>
where
    I: ExactSizeIterator<Item = T>,
    T: Bundle<Effect: NoBundleEffect>,
{
    fn len(&self) -> usize {
        if self.spawner.is_none() {
            return 0;
        }
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

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::Component;

    use super::*;

    #[derive(Clone, Copy, Component)]
    struct ComponentA;

    #[test]
    fn spawn_batch_does_not_leak_entities() {
        let mut world = World::new();
        world.spawn_batch((0u32..50).filter(|&i| i & 1 > 0).map(|_| ComponentA));
        let total_allocated = world.entity_allocator().inner.total_entity_indices();
        world.entity_allocator_mut().inner.flush_freed();
        world.entity_allocator_mut().alloc();
        let reused = world.entity_allocator().inner.total_entity_indices() == total_allocated;
        assert!(reused);
    }
}
