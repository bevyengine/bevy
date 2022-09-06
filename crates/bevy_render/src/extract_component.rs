use crate::{
    render_resource::{encase::internal::WriteInto, DynamicUniformBuffer, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    view::ComputedVisibility,
    Extract, RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component,
    prelude::*,
    query::{QueryItem, ReadOnlyWorldQuery, WorldQuery},
    system::lifetimeless::Read,
};
use std::{marker::PhantomData, ops::Deref};

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

/// Describes how a component gets extracted for rendering.
///
/// Therefore the component is transferred from the "app world" into the "render world"
/// in the [`RenderStage::Extract`](crate::RenderStage::Extract) step.
pub trait ExtractComponent: Component {
    /// ECS [`WorldQuery`] to fetch the components to extract.
    type Query: WorldQuery + ReadOnlyWorldQuery;
    /// Filters the entities with additional constraints.
    type Filter: WorldQuery + ReadOnlyWorldQuery;
    /// Defines how the component is transferred into the "render world".
    fn extract_component(item: QueryItem<Self::Query>) -> Self;
}

/// This plugin prepares the components of the corresponding type for the GPU
/// by transforming them into uniforms.
///
/// They can then be accessed from the [`ComponentUniforms`] resource.
/// For referencing the newly created uniforms a [`DynamicUniformIndex`] is inserted
/// for every processed entity.
///
/// Therefore it sets up the [`RenderStage::Prepare`](crate::RenderStage::Prepare) step
/// for the specified [`ExtractComponent`].
pub struct UniformComponentPlugin<C>(PhantomData<fn() -> C>);

impl<C> Default for UniformComponentPlugin<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: Component + ShaderType + WriteInto + Clone> Plugin for UniformComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(ComponentUniforms::<C>::default())
                .add_system_to_stage(RenderStage::Prepare, prepare_uniform_components::<C>);
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
fn prepare_uniform_components<C: Component>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut component_uniforms: ResMut<ComponentUniforms<C>>,
    components: Query<(Entity, &C)>,
) where
    C: ShaderType + WriteInto + Clone,
{
    component_uniforms.uniforms.clear();
    let entities = components
        .iter()
        .map(|(entity, component)| {
            (
                entity,
                (DynamicUniformIndex::<C> {
                    index: component_uniforms.uniforms.push(component.clone()),
                    marker: PhantomData,
                },),
            )
        })
        .collect::<Vec<_>>();
    commands.insert_or_spawn_batch(entities);

    component_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

/// This plugin extracts the components into the "render world".
///
/// Therefore it sets up the [`RenderStage::Extract`](crate::RenderStage::Extract) step
/// for the specified [`ExtractComponent`].
pub struct ExtractComponentPlugin<C, F = ()> {
    only_extract_visible: bool,
    marker: PhantomData<fn() -> (C, F)>,
}

impl<C, F> Default for ExtractComponentPlugin<C, F> {
    fn default() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }
}

impl<C, F> ExtractComponentPlugin<C, F> {
    pub fn extract_visible() -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
        }
    }
}

impl<C: ExtractComponent> Plugin for ExtractComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            if self.only_extract_visible {
                render_app
                    .add_system_to_stage(RenderStage::Extract, extract_visible_components::<C>);
            } else {
                render_app.add_system_to_stage(RenderStage::Extract, extract_components::<C>);
            }
        }
    }
}

impl<T: Asset> ExtractComponent for Handle<T> {
    type Query = Read<Handle<T>>;
    type Filter = ();

    #[inline]
    fn extract_component(handle: QueryItem<Self::Query>) -> Self {
        handle.clone_weak()
    }
}

/// This system extracts all components of the corresponding [`ExtractComponent`] type.
fn extract_components<C: ExtractComponent>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut query: Extract<Query<(Entity, C::Query), C::Filter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, query_item) in query.iter_mut() {
        values.push((entity, (C::extract_component(query_item),)));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

/// This system extracts all visible components of the corresponding [`ExtractComponent`] type.
fn extract_visible_components<C: ExtractComponent>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut query: Extract<Query<(Entity, &ComputedVisibility, C::Query), C::Filter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, computed_visibility, query_item) in query.iter_mut() {
        if computed_visibility.is_visible() {
            values.push((entity, (C::extract_component(query_item),)));
        }
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
