use crate::system::{ExclusiveSystemParam, SystemMeta};
use crate::{
    component::Tick,
    storage::SparseSetIndex,
    system::{ReadOnlySystemParam, SystemParam},
    world::{FromWorld, World},
};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::unsafe_world_cell::UnsafeWorldCell;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
// We use usize here because that is the largest `Atomic` we want to require
/// A unique identifier for a [`World`].
///
/// The trait [`FromWorld`] is implemented for this type, which returns the
/// ID of the world passed to [`FromWorld::from_world`].
// Note that this *is* used by external crates as well as for internal safety checks
pub struct WorldId(usize);

/// The next [`WorldId`].
static MAX_WORLD_ID: AtomicUsize = AtomicUsize::new(0);

impl WorldId {
    /// Create a new, unique [`WorldId`]. Returns [`None`] if the supply of unique
    /// [`WorldId`]s has been exhausted
    ///
    /// Please note that the [`WorldId`]s created from this method are unique across
    /// time - if a given [`WorldId`] is [`Drop`]ped its value still cannot be reused
    pub fn new() -> Option<Self> {
        MAX_WORLD_ID
            // We use `Relaxed` here since this atomic only needs to be consistent with itself
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                val.checked_add(1)
            })
            .map(WorldId)
            .ok()
    }
}

impl FromWorld for WorldId {
    #[inline]
    fn from_world(world: &mut World) -> Self {
        world.id()
    }
}

// SAFETY: No world data is accessed.
unsafe impl ReadOnlySystemParam for WorldId {}

// SAFETY: No world data is accessed.
unsafe impl SystemParam for WorldId {
    type State = ();

    type Item<'world, 'state> = WorldId;

    fn init_state(_: &mut World, _: &mut crate::system::SystemMeta) -> Self::State {}

    unsafe fn get_param<'world, 'state>(
        _: &'state mut Self::State,
        _: &crate::system::SystemMeta,
        world: UnsafeWorldCell<'world>,
        _: Tick,
    ) -> Self::Item<'world, 'state> {
        world.id()
    }
}

impl ExclusiveSystemParam for WorldId {
    type State = WorldId;
    type Item<'s> = WorldId;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        world.id()
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        *state
    }
}

impl SparseSetIndex for WorldId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.0
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_ids_unique() {
        let ids = std::iter::repeat_with(WorldId::new)
            .take(50)
            .map(Option::unwrap)
            .collect::<Vec<_>>();
        for (i, &id1) in ids.iter().enumerate() {
            // For the first element, i is 0 - so skip 1
            for &id2 in ids.iter().skip(i + 1) {
                assert_ne!(id1, id2, "WorldIds should not repeat");
            }
        }
    }

    #[test]
    fn world_id_system_param() {
        fn test_system(world_id: WorldId) -> WorldId {
            world_id
        }

        let mut world = World::default();
        let system_id = world.register_system(test_system);
        let world_id = world.run_system(system_id).unwrap();
        assert_eq!(world.id(), world_id);
    }

    #[test]
    fn world_id_exclusive_system_param() {
        fn test_system(_world: &mut World, world_id: WorldId) -> WorldId {
            world_id
        }

        let mut world = World::default();
        let system_id = world.register_system(test_system);
        let world_id = world.run_system(system_id).unwrap();
        assert_eq!(world.id(), world_id);
    }

    // We cannot use this test as-is, as it causes other tests to panic due to using the same atomic variable.
    // #[test]
    // #[should_panic]
    // fn panic_on_overflow() {
    //     MAX_WORLD_ID.store(usize::MAX - 50, Ordering::Relaxed);
    //     std::iter::repeat_with(WorldId::new)
    //         .take(500)
    //         .for_each(|_| ());
    // }
}
