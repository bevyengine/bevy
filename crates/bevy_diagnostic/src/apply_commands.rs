use bevy_app::App;
use bevy_ecs::system::{CommandQueue, Commands};
use bevy_ecs::world::World;

/// Extension trait used to apply [`Commands`] immediately on a [`World`].
pub trait ApplyCommands {
    fn apply_commands<R, F: FnOnce(&World, Commands) -> R>(self, f: F) -> R;
}

impl ApplyCommands for &mut World {
    /// Applies some [`Commands`] on this [`World`] immediately.
    ///
    /// This function is not an efficient method of applying commands because it requires the creation of a
    /// dedicated [`CommandQueue`] per call.
    /// However, this function provides a convenient tool for diagnostics and testing because it allows you to
    /// invoke commands on a world immediately, without the need for a system.
    /// Therefore, its use should be reserved for special cases where performance is not a concern.
    ///
    /// See documentation on [`Commands`] for more details.
    ///
    /// # Example
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_diagnostic::ApplyCommands;
    ///
    /// let mut world = World::default();
    /// let entity = world.apply_commands(|_world, mut commands| {
    ///     commands.spawn_empty().id()
    /// });
    ///
    /// assert!(world.get_entity(entity).is_some());
    /// ```
    fn apply_commands<R, F: FnOnce(&World, Commands) -> R>(self, f: F) -> R {
        let mut command_queue = CommandQueue::default();
        let commands = Commands::new(&mut command_queue, self);
        let result = f(self, commands);
        command_queue.apply(self);
        result
    }
}

impl ApplyCommands for &mut App {
    /// Applies some [`Commands`] on the [`World`] of this [`App`] immediately.
    fn apply_commands<R, F: FnOnce(&World, Commands) -> R>(self, f: F) -> R {
        self.world.apply_commands(f)
    }
}
