//! Traits for various [`crate::world::World`] operations.

use crate::prelude::Bundle;

/// Entity spawning functionality.
pub trait Spawn {
    /// Type returned from the spawn functions.
    type SpawnOutput<'a>
    where
        Self: 'a;

    /// Spawns an entity without providing components.
    /// Depending on the implementation, this doesn't mean that there will be no components,
    /// it's just that you don't need to provide any.
    fn spawn_empty(&mut self) -> Self::SpawnOutput<'_>;

    /// Spawns an entity and attaches the provided bundle to it.
    fn spawn<B: Bundle>(&mut self, bundle: B) -> Self::SpawnOutput<'_>;
}
