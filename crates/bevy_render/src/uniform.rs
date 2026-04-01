use crate::{
    render_resource::{encase::internal::WriteInto, DynamicUniformBuffer, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    GpuResourceAppExt, Render, RenderApp, RenderSystems,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{component::Component, prelude::*};
use core::{marker::PhantomData, ops::Deref};

/// Stores the index of a uniform inside of [`ComponentUniforms`].
#[derive(Component)]
pub struct DynamicUniformIndex<C: Component> {
    index: u32,
    marker: PhantomData<C>,
}

impl<C: Component> DynamicUniformIndex<C> {
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
}

/// This plugin prepares the components of the corresponding type for the GPU
/// by transforming them into uniforms.
///
/// They can then be accessed from the [`ComponentUniforms`] resource.
/// For referencing the newly created uniforms a [`DynamicUniformIndex`] is inserted
/// for every processed entity.
///
/// Therefore it sets up the [`RenderSystems::Prepare`] step
/// for the specified [`ExtractComponent`](`crate::extract_component::ExtractComponent`).
pub struct UniformComponentPlugin<C>(PhantomData<fn() -> C>);

impl<C> Default for UniformComponentPlugin<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: Component + ShaderType + WriteInto + Clone> Plugin for UniformComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_gpu_resource::<ComponentUniforms<C>>()
                .add_systems(
                    Render,
                    prepare_uniform_components::<C>.in_set(RenderSystems::PrepareResources),
                );
        }
    }
}

/// Stores all uniforms of the component type.
#[derive(Resource)]
pub struct ComponentUniforms<C: Component + ShaderType> {
    uniforms: DynamicUniformBuffer<C>,
}

impl<C: Component + ShaderType> Deref for ComponentUniforms<C> {
    type Target = DynamicUniformBuffer<C>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.uniforms
    }
}

impl<C: Component + ShaderType> ComponentUniforms<C> {
    #[inline]
    pub fn uniforms(&self) -> &DynamicUniformBuffer<C> {
        &self.uniforms
    }
}

impl<C: Component + ShaderType> Default for ComponentUniforms<C> {
    fn default() -> Self {
        Self {
            uniforms: Default::default(),
        }
    }
}

/// This system prepares all components of the corresponding component type.
/// They are transformed into uniforms and stored in the [`ComponentUniforms`] resource.
fn prepare_uniform_components<C>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut component_uniforms: ResMut<ComponentUniforms<C>>,
    components: Query<(Entity, &C)>,
) where
    C: Component + ShaderType + WriteInto + Clone,
{
    let components_iter = components.iter();
    let count = components_iter.len();
    let Some(mut writer) =
        component_uniforms
            .uniforms
            .get_writer(count, &render_device, &render_queue)
    else {
        return;
    };
    let entities = components_iter
        .map(|(entity, component)| {
            (
                entity,
                DynamicUniformIndex::<C> {
                    index: writer.write(component),
                    marker: PhantomData,
                },
            )
        })
        .collect::<Vec<_>>();
    commands.try_insert_batch(entities);
}
