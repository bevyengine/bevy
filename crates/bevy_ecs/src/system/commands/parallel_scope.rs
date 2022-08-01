use std::cell::Cell;

use thread_local::ThreadLocal;

use crate::{
    entity::Entities,
    prelude::World,
    system::{SystemParam, SystemParamFetch, SystemParamState},
};

use super::{CommandQueue, Commands};

#[doc(hidden)]
#[derive(Default)]
/// The internal [`SystemParamState`] of the [`ParallelCommands`] type
pub struct ParallelCommandsState {
    thread_local_storage: ThreadLocal<Cell<CommandQueue>>,
}

/// An alternative to [`Commands`] that can be used in parallel contexts, such as those in [`Query::par_for_each`](crate::system::Query::par_for_each)
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
///     query.par_for_each(32, |(entity, velocity)| {
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

impl SystemParam for ParallelCommands<'_, '_> {
    type Fetch = ParallelCommandsState;
}

impl<'w, 's> SystemParamFetch<'w, 's> for ParallelCommandsState {
    type Item = ParallelCommands<'w, 's>;

    unsafe fn get_param(
        state: &'s mut Self,
        _: &crate::system::SystemMeta,
        world: &'w World,
        _: u32,
    ) -> Self::Item {
        ParallelCommands {
            state,
            entities: world.entities(),
        }
    }
}

// SAFETY: no component or resource access to report
unsafe impl SystemParamState for ParallelCommandsState {
    fn init(_: &mut World, _: &mut crate::system::SystemMeta) -> Self {
        Self::default()
    }

    fn apply(&mut self, world: &mut World) {
        for cq in &mut self.thread_local_storage {
            cq.get_mut().apply(world);
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
