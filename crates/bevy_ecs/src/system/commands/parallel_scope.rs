use std::cell::Cell;

use thread_local::ThreadLocal;

use crate::{
    entity::Entities,
    prelude::World,
    system::{SystemMeta, SystemParam},
};

use super::{CommandQueue, Commands};

/// The internal [`SystemParam`] state of the [`ParallelCommands`] type
#[doc(hidden)]
#[derive(Default)]
pub struct ParallelCommandsState {
    thread_local_storage: ThreadLocal<Cell<CommandQueue>>,
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
pub struct ParallelCommands<'w, 's> {
    state: &'s mut ParallelCommandsState,
    entities: &'w Entities,
}

// SAFETY: no component or resource access to report
unsafe impl SystemParam for ParallelCommands<'_, '_> {
    type State = ParallelCommandsState;
    type Item<'w, 's> = ParallelCommands<'w, 's>;

    fn init_state(_: &mut World, _: &mut crate::system::SystemMeta) -> Self::State {
        ParallelCommandsState::default()
    }

    fn apply(state: &mut Self::State, _system_meta: &SystemMeta, world: &mut World) {
        #[cfg(feature = "trace")]
        let _system_span =
            bevy_utils::tracing::info_span!("system_commands", name = _system_meta.name())
                .entered();
        for cq in &mut state.thread_local_storage {
            cq.get_mut().apply(world);
        }
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _: &crate::system::SystemMeta,
        world: &'w World,
        _: u32,
    ) -> Self::Item<'w, 's> {
        ParallelCommands {
            state,
            entities: world.entities(),
        }
    }
}

impl<'w, 's> ParallelCommands<'w, 's> {
    pub fn command_scope<R>(&self, f: impl FnOnce(Commands) -> R) -> R {
        let store = &self.state.thread_local_storage;
        let command_queue_cell = store.get_or_default();
        let mut command_queue = command_queue_cell.take();

        let r = f(Commands::new_from_entities(
            &mut command_queue,
            self.entities,
        ));

        command_queue_cell.set(command_queue);
        r
    }
}
