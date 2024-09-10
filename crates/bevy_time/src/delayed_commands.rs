use std::time::Duration;

use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query, Res},
    world::CommandQueue,
};

use crate::{Time, Timer, TimerMode};
use private::DelayedCommandQueue;

/// A [`Commands`]-like type allowing for each queued [`Command`] to be delayed
/// until a certain amount of time has elapsed.
///
/// [`Command`]: bevy_ecs::world::Command
pub struct DelayedCommands<'w, 's> {
    commands: Commands<'w, 's>,
    delayed_queue: CommandQueue,
    timer: Timer,
}

/// System responsible for applying delayed [`CommandQueues`](`CommandQueue`).
pub fn apply_delayed_commands(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DelayedCommandQueue)>,
) {
    for (entity, mut delayed_commands) in query.iter_mut() {
        delayed_commands.timer.tick(time.delta());

        if delayed_commands.timer.finished() {
            commands.append(&mut delayed_commands.queue);
            commands.entity(entity).despawn();
        }
    }
}

impl<'w, 's> Drop for DelayedCommands<'w, 's> {
    fn drop(&mut self) {
        let mut queue = CommandQueue::default();
        queue.append(&mut self.delayed_queue);

        let timer = self.timer.clone();

        self.commands.spawn(DelayedCommandQueue { queue, timer });
    }
}

impl<'w, 's> DelayedCommands<'w, 's> {
    /// Get a [`Commands`] item which will delay all queued [commands].
    ///
    /// [commands]: bevy_ecs::world::Command
    pub fn as_commands<'a>(&'a mut self) -> Commands<'w, 'a> {
        Commands::new_from_commands(&mut self.delayed_queue, &self.commands)
    }
}

/// Extension trait for the [`Commands`] type providing time-related functionality.
pub trait CommandsExt<'w, 's>: private::Sealed {
    /// Create a [`DelayedCommands`] from this [`Commands`] with a delay equal
    /// to the provided [`Duration`].
    /// You can then create a new [`Commands`] from this [`DelayedCommands`] which
    /// will delay all provided [commands].
    ///
    /// Note: Commands which reserve [`entities`](`Entity`) will immediately allocate
    /// their required [`Entity`] IDs, but all operations will still be delayed.
    /// For example, `commands.spawn(MyBundle)` will spawn an [`Entity`] immediately,
    /// but the bundle `MyBundle` will _not_ be inserted until the delay has elapsed.
    ///
    /// # Examples
    ///
    /// ## Despawn after 5 seconds
    ///
    /// ```rust
    /// # use std::time::Duration;
    /// # use bevy_time::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// fn spawn_temporary_entities(mut commands: Commands) {
    ///     let entity = commands.spawn_empty().id();
    ///
    ///     commands
    ///         .after(Duration::from_secs(5))
    ///         .as_commands()
    ///         .entity(entity)
    ///         .despawn();
    /// }
    /// ```
    ///
    /// [commands]: bevy_ecs::world::Command
    fn after<'a>(&'a mut self, duration: Duration) -> DelayedCommands<'w, 'a>;
}

impl<'w, 's> CommandsExt<'w, 's> for Commands<'w, 's> {
    fn after<'a>(&'a mut self, duration: Duration) -> DelayedCommands<'w, 'a> {
        let timer = Timer::new(duration, TimerMode::Once);
        let delayed_queue = CommandQueue::default();

        DelayedCommands {
            commands: self.reborrow(),
            delayed_queue,
            timer,
        }
    }
}

mod private {
    use bevy_ecs::{component::Component, world::CommandQueue};

    use crate::Timer;

    pub trait Sealed {}

    impl<'w, 's> Sealed for bevy_ecs::system::Commands<'w, 's> {}

    /// [`Component`] storing queued [commands] which have been delayed
    /// until after [`timer`](Self::timer) has finished.
    ///
    /// [commands]: bevy_ecs::world::Command
    #[derive(Component)]
    pub struct DelayedCommandQueue {
        pub queue: CommandQueue,
        pub timer: Timer,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use bevy_app::{App, Startup, Update};
    use bevy_ecs::{system::Resource, world::World};

    use super::*;

    #[test]
    fn test() {
        #[derive(Resource, PartialEq, Eq, Debug)]
        struct Flag(bool);

        let mut app = App::new();

        app.insert_resource(Time::new(Instant::now()).as_generic());
        app.insert_resource(Flag(false));
        app.add_systems(Update, apply_delayed_commands);
        app.add_systems(Startup, |mut commands: Commands| {
            commands
                .after(Duration::from_secs(1000))
                .as_commands()
                .add(|world: &mut World| {
                    *world.resource_mut::<Flag>() = Flag(true);
                });

            commands.spawn_empty();
        });

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(false)));

        app.update();

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(false)));

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(999));

        app.update();

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(false)));

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));

        app.update();

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(true)));
    }
}
