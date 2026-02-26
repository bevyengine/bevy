use alloc::vec::Vec;
use bevy_ecs::{prelude::*, system::command::spawn_batch, world::CommandQueue};
use bevy_platform::collections::HashMap;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::time::Duration;

use crate::Time;

/// A wrapper over [`Commands`] that stores [`CommandQueue`]s to be applied with given delays.
///
/// When dropped, the queues are spawned into the world as new entities with
/// [`DelayedCommandQueue`] components, and then checked by the
/// [`check_delayed_command_queues`] system.
pub struct DelayedCommands<'w, 's> {
    /// Used to own queues and deduplicate them by their duration.
    queues: HashMap<Duration, CommandQueue>,

    /// The wrapped `Commands` - used to provision out new `Commands`
    /// and to spawn the queues as entities when the struct is dropped.
    commands: Commands<'w, 's>,
}

impl<'w, 's> DelayedCommands<'w, 's> {
    /// Return a [`Commands`] whose commands will be delayed by `duration`.
    #[must_use = "The returned Commands must be used to submit commands with this delay."]
    pub fn duration(&mut self, duration: Duration) -> Commands<'w, '_> {
        // Fetch a queue with the given duration or create one
        let queue = self.queues.entry(duration).or_default();
        // Return a new `Commands` to write commands to the queue
        self.commands.rebound_to(queue)
    }

    /// Return a [`Commands`] whose commands will be delayed by `secs` seconds.
    #[inline]
    #[must_use = "The returned Commands must be used to submit commands with this delay."]
    pub fn secs(&mut self, secs: f32) -> Commands<'w, '_> {
        self.duration(Duration::from_secs_f32(secs))
    }

    /// Drains and spawns the contained command queues as [`DelayedCommandQueue`] entities.
    fn submit(&mut self) {
        let mut queues = self
            .queues
            .drain()
            .map(|(submit_at, queue)| DelayedCommandQueue { submit_at, queue })
            .collect::<Vec<_>>();

        self.commands.queue(move |world: &mut World| {
            // We use the default Time<()> here intentionally to support custom clocks
            let time = world.resource::<Time>();
            let elapsed = time.elapsed();
            for queue in queues.iter_mut() {
                // Turn relative delays into absolute elapsed times
                queue.submit_at += elapsed;
            }
            spawn_batch(queues).apply(world);
        });
    }
}

/// Extension trait for [`Commands`] that provides delayed command functionality.
pub trait DelayedCommandsExt<'w> {
    /// Returns a [`DelayedCommands`] instance that can be used to queue
    /// commands to be submitted at a later point in time.
    ///
    /// When dropped, the [`DelayedCommands`] submits spawn commands that will
    /// spawn [`DelayedCommandQueue`] entities. The entities are checked
    /// by the [`check_delayed_command_queues`] system, and their queues are
    /// submitted when the specified time has elapsed.
    ///
    /// # Usage
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_time::DelayedCommandsExt;
    /// fn my_system(mut commands: Commands) {
    ///     // Spawn an entity after one second
    ///     commands.delayed().secs(1.0).spawn_empty();
    /// }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    ///
    /// Entity allocation happens immediately even if the spawn command is delayed.
    /// This allows you to queue delayed commands on an entity that hasn't been spawned yet.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_time::DelayedCommandsExt;
    /// fn my_system(mut commands: Commands) {
    ///     let mut delayed = commands.delayed();
    ///     // spawn an entity after 1 second, then despawn it a second later
    ///     let entity = delayed.secs(1.0).spawn_empty().id();
    ///     delayed.secs(2.0).entity(entity).despawn();
    /// }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    ///
    /// # Timing
    ///
    /// Delayed commands are currently checked against the default clock in the [`PreUpdate`]
    /// schedule. There's currently no way to specify different clocks for different
    /// delayed commands - this is a limitation of the system and if you need this behavior
    /// you'll likely have to implement your own delay system.
    ///
    /// [`PreUpdate`]: bevy_app::PreUpdate
    fn delayed(&mut self) -> DelayedCommands<'w, '_>;
}

impl<'w, 's> DelayedCommandsExt<'w> for Commands<'w, 's> {
    fn delayed(&mut self) -> DelayedCommands<'w, '_> {
        DelayedCommands {
            commands: self.reborrow(),
            queues: HashMap::default(),
        }
    }
}

impl<'w, 's> Drop for DelayedCommands<'w, 's> {
    fn drop(&mut self) {
        self.submit();
    }
}

/// A component with a [`CommandQueue`] to be submitted later.
///
/// Queues in these components are checked automatically by the
/// [`check_delayed_command_queues`] added by [`TimePlugin`] and submitted when
/// the default clock's elapsed time exceeds `submit_at`.
///
/// [`TimePlugin`]: crate::TimePlugin
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DelayedCommandQueue {
    /// The elapsed time from startup when `queue` should be submitted.
    pub submit_at: Duration,

    /// The queue to be submitted when time is up.
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub queue: CommandQueue,
}

/// The system used to check [`DelayedCommandQueue`]s, which are usually spawned
/// by [`DelayedCommands`]. When the elapsed time exceeds a queue's `submit_at` time,
/// the contained `queue` is appended to the system's [`Commands`].
pub fn check_delayed_command_queues(
    queues: Query<(Entity, &mut DelayedCommandQueue)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let elapsed = time.elapsed();
    for (e, mut queue) in queues {
        if queue.submit_at <= elapsed {
            // Write the contained delayed commands to the world.
            commands.append(&mut queue.queue);
            commands.entity(e).despawn();
        }
    }
}

#[cfg(test)]
#[expect(clippy::print_stdout, reason = "Allowed in tests.")]
mod tests {
    use core::time::Duration;
    use std::println;

    use bevy_app::{App, Startup};
    use bevy_ecs::{component::Component, system::Commands};

    use crate::{DelayedCommandsExt, TimePlugin, TimeUpdateStrategy};

    #[derive(Component)]
    struct DummyComponent;

    #[test]
    fn delayed_queues_should_run_with_time_plugin_enabled() {
        fn queue_commands(mut commands: Commands) {
            commands.delayed().secs(0.1).spawn(DummyComponent);

            commands.spawn(DummyComponent);

            let mut delayed_cmds = commands.delayed();
            delayed_cmds.secs(0.5).spawn(DummyComponent);

            let mut in_1_sec = delayed_cmds.duration(Duration::from_secs_f32(1.0));
            in_1_sec.spawn(DummyComponent);
            in_1_sec.spawn(DummyComponent);
            in_1_sec.spawn(DummyComponent);
        }

        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_systems(Startup, queue_commands)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                0.2,
            )));

        for frame in 0..10 {
            app.update();
            let dummy_count = app
                .world_mut()
                .query::<&DummyComponent>()
                .iter(app.world())
                .count();

            println!("Frame {frame}, {dummy_count} dummies spawned");

            match frame {
                0 => {
                    assert_eq!(dummy_count, 1);
                }
                1 | 2 => {
                    assert_eq!(dummy_count, 2);
                }
                3 | 4 => {
                    assert_eq!(dummy_count, 3);
                }
                _ => {
                    assert_eq!(dummy_count, 6);
                }
            }
        }
    }
}
