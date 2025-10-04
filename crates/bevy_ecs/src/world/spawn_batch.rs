use bevy_ptr::move_as_ptr;

use crate::{
    bundle::{Bundle, BundleSpawner, NoBundleEffect},
    change_detection::MaybeLocation,
    entity::{Entity, EntitySetIterator},
    world::World,
};
use core::iter::{FusedIterator, Peekable};

/// An iterator that spawns a series of entities and returns the [ID](Entity) of
/// each spawned entity.
///
/// If this iterator is not fully exhausted, any remaining entities will be spawned when this type is dropped.
pub struct SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    inner: Peekable<I>,
    spawner: Option<BundleSpawner<'w>>,
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
        let change_tick = world.change_tick();
        let mut inner = iter.peekable();

        let spawner = if let Some(bundle) = inner.peek() {
            let mut spawner = BundleSpawner::new::<I::Item>(world, change_tick, bundle);
            if I::Item::count_fragmenting_values() == 0 {
                spawner.reserve_storage(length);
            }
            Some(spawner)
        } else {
            None
        };

        Self {
            inner,
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

        if let Some(spawner) = &mut self.spawner {
            // SAFETY: `self.spawner` will be dropped immediately after this call.
            unsafe { spawner.flush_commands() }
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

        let spawner = self.spawner.as_mut().unwrap();

        if I::Item::count_fragmenting_values() > 0 {
            spawner.replace(&bundle);
        }

        move_as_ptr!(bundle);
        // SAFETY:
        // - The spawner matches `I::Item`'s type.
        // - `I::Item::Effect: NoBundleEffect`, thus [`apply_effect`] does not need to be called.
        // - `bundle` is not accessed or dropped after this function call.
        unsafe { Some(spawner.spawn::<I::Item>(bundle, self.caller)) }
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
