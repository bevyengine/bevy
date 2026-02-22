use crate::MainWorld;
use bevy_ecs::{
    change_detection::Tick,
    prelude::*,
    query::FilteredAccessSet,
    system::{
        ReadOnlySystemParam, SharedStates, SystemMeta, SystemParam, SystemParamItem,
        SystemParamValidationError, SystemState,
    },
    world::unsafe_world_cell::UnsafeWorldCell,
};
use core::ops::{Deref, DerefMut};

/// A helper for accessing [`MainWorld`] content using a system parameter.
///
/// A [`SystemParam`] adapter which applies the contained `SystemParam` to the [`World`]
/// contained in [`MainWorld`]. This parameter only works for systems run
/// during the [`ExtractSchedule`](crate::ExtractSchedule).
///
/// This requires that the contained [`SystemParam`] does not mutate the world, as it
/// uses a read-only reference to [`MainWorld`] internally.
///
/// ## Context
///
/// [`ExtractSchedule`] is used to extract (move) data from the simulation world ([`MainWorld`]) to the
/// render world. The render world drives rendering each frame (generally to a `Window`).
/// This design is used to allow performing calculations related to rendering a prior frame at the same
/// time as the next frame is simulated, which increases throughput (FPS).
///
/// [`Extract`] is used to get data from the main world during [`ExtractSchedule`].
///
/// ## Examples
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_render::Extract;
/// use bevy_render::sync_world::RenderEntity;
/// # #[derive(Component)]
/// // Do make sure to sync the cloud entities before extracting them.
/// # struct Cloud;
/// fn extract_clouds(mut commands: Commands, clouds: Extract<Query<RenderEntity, With<Cloud>>>) {
///     for cloud in &clouds {
///         commands.entity(cloud).insert(Cloud);
///     }
/// }
/// ```
///
/// [`ExtractSchedule`]: crate::ExtractSchedule
/// [Window]: bevy_window::Window
pub struct Extract<'w, 's, P>
where
    P: ReadOnlySystemParam + 'static,
{
    item: SystemParamItem<'w, 's, P>,
}

#[doc(hidden)]
pub struct ExtractState<P: SystemParam + 'static> {
    state: SystemState<P>,
    main_world_state: <Res<'static, MainWorld> as SystemParam>::State,
}

// SAFETY: The only `World` access (`Res<MainWorld>`) is read-only.
unsafe impl<P> ReadOnlySystemParam for Extract<'_, '_, P> where P: ReadOnlySystemParam {}

// SAFETY: The only `World` access is properly registered by `Res<MainWorld>::init_state`.
// This call will also ensure that there are no conflicts with prior params.
unsafe impl<P> SystemParam for Extract<'_, '_, P>
where
    P: ReadOnlySystemParam,
{
    type State = ExtractState<P>;
    type Item<'w, 's> = Extract<'w, 's, P>;

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        let mut main_world = world.resource_mut::<MainWorld>();
        ExtractState {
            state: SystemState::new(&mut main_world),
            // SAFETY: caller upholds requirements
            main_world_state: unsafe { Res::<MainWorld>::init_state(world, shared_states) },
        }
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        Res::<MainWorld>::init_access(
            &state.main_world_state,
            system_meta,
            component_access_set,
            world,
        );
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to world data registered in `init_state`.
        let result = unsafe { world.get_resource_by_id(state.main_world_state) };
        let Some(main_world) = result else {
            return Err(SystemParamValidationError::invalid::<Self>(
                "`MainWorld` resource does not exist",
            ));
        };
        // SAFETY: Type is guaranteed by `SystemState`.
        let main_world: &World = unsafe { main_world.deref() };
        // SAFETY: We provide the main world on which this system state was initialized on.
        unsafe {
            SystemState::<P>::validate_param(
                &mut state.state,
                main_world.as_unsafe_world_cell_readonly(),
            )
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY:
        // - The caller ensures that `world` is the same one that `init_state` was called with.
        // - The caller ensures that no other `SystemParam`s will conflict with the accesses we have registered.
        let main_world = unsafe {
            Res::<MainWorld>::get_param(
                &mut state.main_world_state,
                system_meta,
                world,
                change_tick,
            )
        };
        let item = state.state.get(main_world.into_inner());
        Extract { item }
    }
}

impl<'w, 's, P> Deref for Extract<'w, 's, P>
where
    P: ReadOnlySystemParam,
{
    type Target = SystemParamItem<'w, 's, P>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<'w, 's, P> DerefMut for Extract<'w, 's, P>
where
    P: ReadOnlySystemParam,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<'a, 'w, 's, P> IntoIterator for &'a Extract<'w, 's, P>
where
    P: ReadOnlySystemParam,
    &'a SystemParamItem<'w, 's, P>: IntoIterator,
{
    type Item = <&'a SystemParamItem<'w, 's, P> as IntoIterator>::Item;
    type IntoIter = <&'a SystemParamItem<'w, 's, P> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.item).into_iter()
    }
}
