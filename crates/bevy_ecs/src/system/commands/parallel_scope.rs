use bevy_utils::Parallel;

use crate::{
    self as bevy_ecs,
    entity::Entities,
    prelude::World,
    system::{Deferred, SystemBuffer, SystemMeta, SystemParam},
};

use super::{CommandQueue, Commands};

#[derive(Default)]
struct ParallelCommandQueue {
    thread_queues: Parallel<CommandQueue>,
}

/// An alternative to [`Commands`] that can be used in parallel contexts, such as those in [`Query::par_iter`](crate::system::Query::par_iter)
///
/// Note: Because command application order will depend on how many threads are ran, non-commutative commands may result in non-deterministic results.
///
/// Example:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_tasks::ComputeTaskPool;
/// #
/// # #[derive(Component)]
/// # struct Velocity;
/// # impl Velocity { fn magnitude(&self) -> f32 { 42.0 } }
/// fn parallel_command_system(
///     mut query: Query<(Entity, &Velocity)>,
///     par_commands: ParallelCommands
/// ) {
///     query.par_iter().for_each(|(entity, velocity)| {
///         if velocity.magnitude() > 10.0 {
///             par_commands.command_scope(|mut commands| {
///                 commands.entity(entity).despawn();
///             });
///         }
///     });
/// }
/// # bevy_ecs::system::assert_is_system(parallel_command_system);
///```
#[derive(SystemParam)]
pub struct ParallelCommands<'w, 's> {
    state: Deferred<'s, ParallelCommandQueue>,
    entities: &'w Entities,
}

impl SystemBuffer for ParallelCommandQueue {
    #[inline]
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        #[cfg(feature = "trace")]
        let _system_span = _system_meta.commands_span.enter();
        for cq in self.thread_queues.iter_mut() {
            cq.apply(world);
        }
    }
}

impl<'w, 's> ParallelCommands<'w, 's> {
    /// Temporarily provides access to the [`Commands`] for the current thread.
    ///
    /// For an example, see the type-level documentation for [`ParallelCommands`].
    pub fn command_scope<R>(&self, f: impl FnOnce(Commands) -> R) -> R {
        self.state.thread_queues.scope(|queue| {
            let commands = Commands::new_from_entities(queue, self.entities);
            f(commands)
        })
    }
}
