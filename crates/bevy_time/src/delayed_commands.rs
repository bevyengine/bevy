use crate::Time;
use alloc::boxed::Box;
use bevy_ecs::prelude::*;
use bevy_log::warn;
use core::time::Duration;

/// A [`Command`] that will be executed after a specified delay has elapsed.
///
/// This can be helpful for scheduling actions at some point in the future.
///
/// This works by moving the supplied command into a component that is spawned on an entity.
/// Delayed command entities are ticked via [`tick_delayed_commands`],
/// which is typically run in [`First`] as part of [`TimePlugin`].
#[derive(Component)]
pub struct DelayedCommand {
    pub delay: Duration,
    pub command: Box<dyn Command + Send + Sync + 'static>,
}

impl DelayedCommand {
    pub fn new(delay: Duration, command: impl Command + Send + Sync + 'static) -> Self {
        Self {
            delay,
            command: Box::new(command),
        }
    }
}

impl Command for DelayedCommand {
    /// Spawns a new entity with the [`DelayedCommand`] as a component.
    fn apply(self, world: &mut World) {
        world.spawn(self);
    }
}

pub fn tick_delayed_commands(
    mut commands: Commands,
    time: Res<Time>,
    mut delayed_commands: Query<(Entity, &mut DelayedCommand)>,
) {
    let delta = time.delta();
    for (entity, mut delayed_command) in delayed_commands.iter_mut() {
        delayed_command.delay -= delta;
        if delayed_command.delay <= Duration::ZERO {
            commands.entity(entity).queue(EvaluateDelayedCommand);
        }
    }
}

/// An [`EntityCommand`] that causes a delayed command to be evaluated.
///
/// This will send the command to the [`CommandQueue`] for execution,
/// and clean up the entity that held the delayed command.
struct EvaluateDelayedCommand;

impl EntityCommand for EvaluateDelayedCommand {
    fn apply(self, mut entity_world_mut: EntityWorldMut) -> () {
        // Take the DelayedCommand component from the entity,
        // allowing us to execute the command and clean up the entity
        // without cloning the command.
        let Some(delayed_command) = entity_world_mut.take::<DelayedCommand>() else {
            warn!(
                "Entity {} does not have a DelayedCommand component at the time of evaluation",
                entity_world_mut.id()
            );
            entity_world_mut.despawn();

            return;
        };

        // Clean up the entity that held the delayed command
        let entity = entity_world_mut.id();
        let world = entity_world_mut.into_world_mut();
        world.despawn(entity);

        // Execute the delayed command
        world.commands().queue(delayed_command.command);
    }
}
