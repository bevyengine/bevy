use alloc::vec::Vec;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_platform::collections::HashMap;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::time::Duration;

use crate::{Time, Timer, TimerMode};

/// A wrapper over [`Commands`] that stores [`CommandQueue`]s to be applied with given delays.
///
/// When dropped, the queues are spawned into the world as new entities with
/// [`DelayedCommandQueue`] components, and then ticked by the [`tick_delayed_command_queues`].
///
/// [`tick_delayed_command_queues`]: crate::tick_delayed_command_queues
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
        let queues = self
            .queues
            .drain()
            .map(|(dur, queue)| DelayedCommandQueue {
                timer: Timer::new(dur, TimerMode::Once),
                queue,
            })
            .collect::<Vec<_>>();
        self.commands.spawn_batch(queues);
    }
}

/// Extension trait for `Commands` that provides delayed command functionality.
pub trait DelayedCommandsExt<'w> {
    /// Returns a [`DelayedCommands`] instance that can be used to queue
    /// commands to be submitted at a later point in time.
    ///
    /// When dropped, the [`DelayedCommands`] submits spawn commands that will
    /// spawn [`DelayedCommandQueue`] entities. The entities' timers are ticked
    /// by the [`tick_delayed_command_queues`] system, and their queues are
    /// submitted when the timer finishes.
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
    /// Delayed commands are currently ticked by the default clock in the [`PreUpdate`]
    /// schedule. There's currently no way to specify different clocks for different
    /// delayed commands - this is a limitation of the system and if you need this behavior
    /// you'll likely have to implement your own delay system.
    ///
    /// [`tick_delayed_command_queues`]: crate::tick_delayed_command_queues
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

/// A component with a [`Timer`] and a [`CommandQueue`] to be submitted later.
///
/// Timers in these components are ticked automatically by the [`tick_delayed_command_queues`]
/// added by [`TimePlugin`].
///
/// [`tick_delayed_command_queues`]: crate::tick_delayed_command_queues
/// [`TimePlugin`]: crate::TimePlugin
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DelayedCommandQueue {
    /// The timer that determines when the queue is submitted.
    pub timer: Timer,

    /// The queue to be submitted when the timer finishes.
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub queue: CommandQueue,
}

/// The system used to tick [`DelayedCommandQueue`] timers, which are usually
/// spawned by [`DelayedCommands`]. When the timer finishes, the contained queue
/// is appended to the system's own [`Commands`].
pub fn tick_delayed_command_queues(
    queues: Query<(Entity, &mut DelayedCommandQueue)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (e, mut queue) in queues {
        if queue.timer.tick(time.delta()).just_finished() {
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
