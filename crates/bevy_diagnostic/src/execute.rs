use bevy_app::App;
use bevy_ecs::prelude::{Commands, World};
use bevy_ecs::system::CommandQueue;

/// Extension trait used to execute [`Commands`] immediately on a [`World`].
pub trait Execute {
    fn execute<F: FnOnce(&World, Commands) -> R, R>(self, f: F) -> R;
}

impl Execute for &mut World {
    /// Creates a new [`Commands`] instances and passes it to the given function, then executes it on this [`World`].
    ///
    /// This function is not an efficient method of executing commands because it requires the creation of a
    /// dedicated [`CommandQueue`] per call. See documentation on [`Commands`] for proper usage.
    ///
    /// However, this function provides a convenient tool for diagnostics and testing because it allows you to
    /// invoke commands on a world immediately, without the need for a system.
    /// Therefore, its use should be reserved for special cases where performance or memory is not a concern.
    ///
    /// # Example
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_diagnostic::Execute;
    ///
    /// let mut world = World::default();
    /// let entity = world.execute(|_world, mut commands| {
    ///     commands.spawn_empty().id()
    /// });
    ///
    /// assert!(world.get_entity(entity).is_some());
    /// ```
    fn execute<F: FnOnce(&World, Commands) -> R, R>(self, f: F) -> R {
        let mut queue = CommandQueue::default();
        let commands = Commands::new(&mut queue, self);
        let result = f(self, commands);
        queue.apply(self);
        result
    }
}

impl Execute for &mut App {
    /// Invokes [`Execute`] on the [`World`] of this [`App`].
    fn execute<F: FnOnce(&World, Commands) -> R, R>(self, f: F) -> R {
        self.world.execute(f)
    }
}
