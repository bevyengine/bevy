use crate::{
    bundle::{Bundle, BundleSpawner, NoBundleEffect},
    change_detection::MaybeLocation,
    entity::{Entity, EntitySetIterator},
    world::World,
};
use core::iter::FusedIterator;

enum BundleSpawnerOrWorld<'w> {
    World(&'w mut World),
    Spawner(BundleSpawner<'w>),
}

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
    spawner_or_world: BundleSpawnerOrWorld<'w>,
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

        let (lower, upper) = iter.size_hint();
        let length = upper.unwrap_or(lower);
        world.entities.reserve(length as u32);

        // We cannot reuse the same spawner if the bundle has any fragmenting by value components
        // since the target archetype depends on the bundle value, not just its type.
        let spawner_or_world = if I::Item::has_fragmenting_values() {
            BundleSpawnerOrWorld::World(world)
        } else {
            let change_tick = world.change_tick();
            let mut spawner = BundleSpawner::new_uniform::<I::Item>(world, change_tick);
            spawner.reserve_storage(length);
            BundleSpawnerOrWorld::Spawner(spawner)
        };
        Self {
            inner: iter,
            spawner_or_world,
            caller,
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
        match &mut self.spawner_or_world {
            BundleSpawnerOrWorld::World(world) => world.flush_commands(),
            BundleSpawnerOrWorld::Spawner(bundle_spawner) => {
                // SAFETY: `self.spawner` will be dropped immediately after this call.
                unsafe { bundle_spawner.flush_commands() }
            }
        }
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
        match &mut self.spawner_or_world {
            BundleSpawnerOrWorld::World(world) => {
                let change_tick = world.change_tick();
                let mut spawner = BundleSpawner::new(world, change_tick, &bundle);
                // SAFETY: bundle matches spawner type
                unsafe { Some(spawner.spawn(bundle, self.caller).0) }
            }
            BundleSpawnerOrWorld::Spawner(spawner) => {
                // SAFETY: bundle matches spawner type
                unsafe { Some(spawner.spawn(bundle, self.caller).0) }
            }
        }
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
