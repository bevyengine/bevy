use std::cell::Cell;

use thread_local::ThreadLocal;

use crate::{
    entity::Entities,
    prelude::World,
    system::{SystemParam, SystemParamFetch, SystemParamState},
};

use super::{CommandQueue, Commands};

#[derive(Default)]
pub struct ParallelCommandsState {
    tls: ThreadLocal<Cell<CommandQueue>>,
}

/// An alternative to [`Commands`] that can be used in parallel contexts, such as those in [`Query::par_for_each`]
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
///     pool: Res<ComputeTaskPool>,
///     par_commands: ParallelCommands
/// ) {
///     query.par_for_each(&pool, 32, |(entity, velocity)| {
///         if velocity.magnitude() > 10.0 {
///             par_commands.command_scope(|mut commands| {
///                 commands.entity(entity).despawn();
///             });
///         }
///     });
/// }
/// # bevy_ecs::system::assert_is_system(parallel_command_system);
///```
/// [Query::par_for_each]: crate::system::Query::par_for_each
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

// SAFE: no component or resource access to report
unsafe impl SystemParamState for ParallelCommandsState {
    fn init(_: &mut World, _: &mut crate::system::SystemMeta) -> Self {
        Self::default()
    }

    fn apply(&mut self, world: &mut World) {
        for cq in self.tls.iter_mut() {
            cq.get_mut().apply(world);
        }
    }
}

impl<'w, 's> ParallelCommands<'w, 's> {
    pub fn command_scope<R>(&self, f: impl FnOnce(Commands) -> R) -> R {
        let tls = &self.state.tls;
        let tl_cq = tls.get_or_default();
        let mut cq = tl_cq.take();

        let r = f(Commands::new_from_entities(&mut cq, self.entities));

        tl_cq.set(cq);
        r
    }
}
