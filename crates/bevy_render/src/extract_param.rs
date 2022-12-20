use crate::MainWorld;
use bevy_ecs::{
    prelude::*,
    system::{
        ReadOnlySystemParam, ResState, SystemMeta, SystemParam, SystemParamItem, SystemParamState,
        SystemState,
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
///     for cloud in &clouds {
///         commands.get_or_spawn(cloud).insert(Cloud);
///     }
/// }
/// ```
///
/// [`RenderStage::Extract`]: crate::RenderStage::Extract
/// [Window]: bevy_window::Window
pub struct Extract<'w, 's, P>
where
    P: ReadOnlySystemParam + 'static,
{
    item: SystemParamItem<'w, 's, P>,
}

impl<'w, 's, P> SystemParam for Extract<'w, 's, P>
where
    P: ReadOnlySystemParam,
{
    type State = ExtractState<P>;
}

#[doc(hidden)]
pub struct ExtractState<P: SystemParam + 'static> {
    state: SystemState<P>,
    main_world_state: ResState<MainWorld>,
}

// SAFETY: only accesses MainWorld resource with read only system params using ResState,
// which is initialized in init()
unsafe impl<P: SystemParam + 'static> SystemParamState for ExtractState<P>
where
    P: ReadOnlySystemParam + 'static,
{
    type Item<'w, 's> = Extract<'w, 's, P>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let mut main_world = world.resource_mut::<MainWorld>();
        Self {
            state: SystemState::new(&mut main_world),
            main_world_state: ResState::init(world, system_meta),
        }
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
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
    P: ReadOnlySystemParam,
{
    type Target = SystemParamItem<'w, 's, P>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<'w, 's, P: SystemParam> DerefMut for Extract<'w, 's, P>
where
    P: ReadOnlySystemParam,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<'a, 'w, 's, P: SystemParam> IntoIterator for &'a Extract<'w, 's, P>
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
