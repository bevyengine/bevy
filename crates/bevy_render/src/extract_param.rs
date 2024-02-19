use crate::MainWorld;
use bevy_ecs::{
    component::Tick,
    prelude::*,
    system::{ReadOnlySystemParam, SystemMeta, SystemParam, SystemParamItem, SystemState},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use std::ops::{Deref, DerefMut};

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
/// # #[derive(Component)]
/// # struct Cloud;
/// fn extract_clouds(mut commands: Commands, clouds: Extract<Query<Entity, With<Cloud>>>) {
///     for cloud in &clouds {
///         commands.get_or_spawn(cloud).insert(Cloud);
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

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let mut main_world = world.resource_mut::<MainWorld>();
        ExtractState {
            state: SystemState::new(&mut main_world),
            main_world_state: Res::<MainWorld>::init_state(world, system_meta),
        }
    }

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
