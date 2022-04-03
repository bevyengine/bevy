use crate::MainWorld;
use bevy_ecs::{
    prelude::*,
    system::{ReadOnlySystemParamFetch, SystemParam, SystemParamItem, SystemState},
};

/// Implementation detail of [`Extract`]
pub struct MainWorldState<P: SystemParam>(SystemState<P>);

impl<P: SystemParam> FromWorld for MainWorldState<P> {
    fn from_world(world: &mut World) -> Self {
        Self(SystemState::new(&mut world.resource_mut::<MainWorld>().0))
    }
}

/// A helper for accessing [`MainWorld`] content using a system parameter.
///
/// A [`SystemParam`] adapter which applies the contained `SystemParam` to the [`World`]
/// contained in [`MainWorld`]. This parameter only works for systems run
/// during [`RenderStage::Extract`].
///
/// This requires that the contained [`SystemParam`] does not mutate the world, as it
/// uses [`Res<MainWorld>`](Res). To get access to the contained `SystemParam`'s item, you
/// must use [`Extract::value`]. This is required because of lifetime limitations in
/// the `SystemParam` api.
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
/// fn extract_clouds(mut commands: Commands, mut clouds: Extract<Query<Entity, With<Cloud>>>) {
///     for cloud in clouds.value().iter() {
///         commands.get_or_spawn(cloud).insert(Cloud);
///     }
/// }
/// ```
///
/// [`RenderStage::Extract`]: crate::RenderStage::Extract
/// [Window]: bevy_window::Window
#[derive(SystemParam)]
pub struct Extract<'w, 's, P: SystemParam + 'static>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    state: Local<'s, MainWorldState<P>>,
    world: Res<'w, MainWorld>,
}

impl<'w, 's, P: SystemParam + 'static> Extract<'w, 's, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    pub fn value(&mut self) -> SystemParamItem<'_, '_, P> {
        self.state.0.get(&self.world)
    }
}
