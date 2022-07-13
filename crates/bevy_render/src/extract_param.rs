use crate::MainWorld;
use bevy_ecs::{
    prelude::*,
    system::{
        ReadOnlySystemParamFetch, ResState, SystemMeta, SystemParam, SystemParamFetch,
        SystemParamState, SystemState,
    },
};
use std::ops::{Deref, DerefMut};

/// A helper for accessing [`MainWorld`] content using a system parameter.
///
/// A [`SystemParam`] adapter which applies the contained `SystemParam` to the [`World`]
/// contained in [`MainWorld`]. This parameter only works for systems run
/// during [`RenderStage::Extract`].
///
/// This requires that the contained [`SystemParam`] does not mutate the world, as it
/// uses a read-only reference to [`MainWorld`] internally.
///
/// ## Context
///
/// [`RenderStage::Extract`] is used to extract (move) data from the simulation world ([`MainWorld`]) to the
/// render world. The render world drives rendering each frame (generally to a [Window]).
/// This design is used to allow performing calculations related to rendering a prior frame at the same
/// time as the next frame is simulated, which increases throughput (FPS).
///
/// [`Extract`] is used to get data from the main world during [`RenderStage::Extract`].
///
/// ## Examples
///
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_render::Extract;
/// # #[derive(Component)]
/// # struct Cloud;
/// fn extract_clouds(mut commands: Commands, clouds: Extract<Query<Entity, With<Cloud>>>) {
///     for cloud in clouds.iter() {
///         commands.get_or_spawn(cloud).insert(Cloud);
///     }
/// }
/// ```
///
/// [`RenderStage::Extract`]: crate::RenderStage::Extract
/// [Window]: bevy_window::Window
pub struct Extract<'w, 's, P: SystemParam + 'static>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    item: <P::Fetch as SystemParamFetch<'w, 's>>::Item,
}

impl<'w, 's, P: SystemParam> SystemParam for Extract<'w, 's, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    type Fetch = ExtractState<P>;
}

#[doc(hidden)]
pub struct ExtractState<P: SystemParam> {
    state: SystemState<P>,
    main_world_state: ResState<MainWorld>,
}

// SAFETY: only accesses MainWorld resource with read only system params using ResState,
// which is initialized in init()
unsafe impl<P: SystemParam + 'static> SystemParamState for ExtractState<P> {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let mut main_world = world.resource_mut::<MainWorld>();
        Self {
            state: SystemState::new(&mut main_world),
            main_world_state: ResState::init(world, system_meta),
        }
    }
}

impl<'w, 's, P: SystemParam + 'static> SystemParamFetch<'w, 's> for ExtractState<P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    type Item = Extract<'w, 's, P>;

    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let main_world = ResState::<MainWorld>::get_param(
            &mut state.main_world_state,
            system_meta,
            world,
            change_tick,
        );
        let item = state.state.get(main_world.into_inner());
        Extract { item }
    }
}

impl<'w, 's, P: SystemParam> Deref for Extract<'w, 's, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    type Target = <P::Fetch as SystemParamFetch<'w, 's>>::Item;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<'w, 's, P: SystemParam> DerefMut for Extract<'w, 's, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}
