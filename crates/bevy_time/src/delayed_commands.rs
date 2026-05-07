use alloc::vec::Vec;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_platform::collections::{hash_map::Entry, HashMap};
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
    queues: HashMap<Duration, PendingDelayedCommandQueue>,

    /// The wrapped `Commands` - used to provision out new `Commands`
    /// and to spawn the queues as entities when the struct is dropped.
    commands: Commands<'w, 's>,
}

struct PendingDelayedCommandQueue {
    entity: Entity,
    queue: CommandQueue,
}

/// A handle that can be used to cancel delayed commands.
///
/// This handle represents a [`DelayedCommandQueue`] entity. Despawning the
/// entity before its timer expires prevents the delayed commands from running.
/// Commands queued for the same delay by the same [`DelayedCommands`] share a
/// handle and are cancelled together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DelayedCommandHandle {
    entity: Entity,
}

impl DelayedCommandHandle {
    /// Returns the entity that tracks the delayed commands.
    #[inline]
    pub fn entity(self) -> Entity {
        self.entity
    }

    /// Cancels the delayed commands.
    ///
    /// This is equivalent to despawning the entity returned by [`Self::entity`].
    /// If the commands have already run, this does nothing.
    #[inline]
    pub fn cancel(self, commands: &mut Commands) {
        commands.entity(self.entity).try_despawn();
    }
}

impl<'w, 's> DelayedCommands<'w, 's> {
    /// Return a [`Commands`] whose commands will be delayed by `duration`.
    #[must_use = "The returned Commands must be used to submit commands with this delay."]
    pub fn duration(&mut self, duration: Duration) -> Commands<'w, '_> {
        self.duration_with_handle(duration).0
    }

    /// Return a [`Commands`] whose commands will be delayed by `duration`, and
    /// a handle that can be used to cancel them.
    ///
    /// Commands queued for the same `duration` share a queue and will return
    /// the same handle.
    #[must_use = "The returned Commands must be used to submit commands with this delay."]
    pub fn duration_with_handle(
        &mut self,
        duration: Duration,
    ) -> (Commands<'w, '_>, DelayedCommandHandle) {
        // Fetch a queue with the given duration or create one
        let queue = match self.queues.entry(duration) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(PendingDelayedCommandQueue {
                entity: self.commands.spawn_empty().id(),
                queue: CommandQueue::default(),
            }),
        };

        let handle = DelayedCommandHandle {
            entity: queue.entity,
        };
        // Return a new `Commands` to write commands to the queue
        (self.commands.rebound_to(&mut queue.queue), handle)
    }

    /// Return a [`Commands`] whose commands will be delayed by `secs` seconds.
    #[inline]
    #[must_use = "The returned Commands must be used to submit commands with this delay."]
    pub fn secs(&mut self, secs: f32) -> Commands<'w, '_> {
        self.duration(Duration::from_secs_f32(secs))
    }

    /// Return a [`Commands`] whose commands will be delayed by `secs` seconds,
    /// and a handle that can be used to cancel them.
    ///
    /// Commands queued for the same `secs` value share a queue and will return
    /// the same handle.
    #[inline]
    #[must_use = "The returned Commands must be used to submit commands with this delay."]
    pub fn secs_with_handle(&mut self, secs: f32) -> (Commands<'w, '_>, DelayedCommandHandle) {
        self.duration_with_handle(Duration::from_secs_f32(secs))
    }

    /// Drains and spawns the contained command queues as [`DelayedCommandQueue`] entities.
    fn submit(&mut self) {
        let mut queues = self
            .queues
            .drain()
            .map(|(submit_at, pending)| {
                (
                    pending.entity,
                    DelayedCommandQueue {
                        submit_at,
                        queue: pending.queue,
                    },
                )
            })
            .collect::<Vec<_>>();

        self.commands.queue(move |world: &mut World| {
            // We use the default Time<()> here intentionally to support custom clocks
            let time = world.resource::<Time>();
            let elapsed = time.elapsed();
            for (_, queue) in queues.iter_mut() {
                // Turn relative delays into absolute elapsed times
                queue.submit_at += elapsed;
            }
            for (entity, queue) in queues.drain(..) {
                if let Ok(mut entity) = world.get_entity_mut(entity) {
                    entity.insert(queue);
                }
            }
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
    /// You can cancel a delayed command queue by using the handle returned from
    /// [`DelayedCommands::secs_with_handle`] or [`DelayedCommands::duration_with_handle`].
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_time::DelayedCommandsExt;
    /// #[derive(Resource)]
    /// struct SpawnHandle(Entity);
    ///
    /// fn queue_spawn(mut commands: Commands) {
    ///     let handle = {
    ///         let mut delayed = commands.delayed();
    ///         let (mut after_one_second, handle) = delayed.secs_with_handle(1.0);
    ///         after_one_second.spawn_empty();
    ///         handle
    ///     };
    ///     commands.insert_resource(SpawnHandle(handle.entity()));
    /// }
    ///
    /// fn cancel_spawn(mut commands: Commands, handle: Res<SpawnHandle>) {
    ///     commands.entity(handle.0).try_despawn();
    /// }
    /// # bevy_ecs::system::assert_is_system(queue_spawn);
    /// # bevy_ecs::system::assert_is_system(cancel_spawn);
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

    use bevy_app::{App, Startup, Update};
    use bevy_ecs::{
        component::Component,
        prelude::{Res, Resource},
        system::Commands,
    };

    use crate::{DelayedCommandHandle, DelayedCommandsExt, TimePlugin, TimeUpdateStrategy};

    #[derive(Component)]
    struct DummyComponent;

    #[derive(Resource)]
    struct DelayedSpawn(DelayedCommandHandle);

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

    #[test]
    fn delayed_queues_can_be_cancelled() {
        fn queue_commands(mut commands: Commands) {
            let handle = {
                let mut delayed_cmds = commands.delayed();
                let (mut delayed, handle) = delayed_cmds.secs_with_handle(0.1);
                delayed.spawn(DummyComponent);
                handle
            };
            commands.insert_resource(DelayedSpawn(handle));
        }

        fn cancel_commands(mut commands: Commands, delayed_spawn: Res<DelayedSpawn>) {
            delayed_spawn.0.cancel(&mut commands);
        }

        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_systems(Startup, queue_commands)
            .add_systems(Update, cancel_commands)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                0.05,
            )));

        for _ in 0..10 {
            app.update();
        }

        let dummy_count = app
            .world_mut()
            .query::<&DummyComponent>()
            .iter(app.world())
            .count();

        assert_eq!(dummy_count, 0);
    }
}
