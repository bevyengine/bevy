use crate::{
    render_resource::DynamicUniformVec,
    renderer::{RenderDevice, RenderQueue},
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component,
    prelude::*,
    query::{FilterFetch, QueryItem, ReadOnlyFetch, WorldQuery},
    system::{
        lifetimeless::{Read, SCommands, SQuery},
        RunSystem, SystemParamItem,
    },
};
use crevice::std140::AsStd140;
use std::{marker::PhantomData, ops::Deref};

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

pub trait ExtractComponent: Component {
    type Query: WorldQuery;
    type Filter: WorldQuery;
    fn extract_component(item: QueryItem<Self::Query>) -> Self;
}

/// Extracts assets into gpu-usable data
pub struct UniformComponentPlugin<C>(PhantomData<fn() -> C>);

impl<C> Default for UniformComponentPlugin<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: Component + AsStd140 + Clone> Plugin for UniformComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .insert_resource(ComponentUniforms::<C>::default())
            .add_system_to_stage(
                RenderStage::Prepare,
                prepare_uniform_components::<C>.system(),
            );
    }
}

pub struct ComponentUniforms<C: Component + AsStd140> {
    uniforms: DynamicUniformVec<C>,
}

impl<C: Component + AsStd140> Deref for ComponentUniforms<C> {
    type Target = DynamicUniformVec<C>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.uniforms
    }
}

impl<C: Component + AsStd140> ComponentUniforms<C> {
    #[inline]
    pub fn uniforms(&self) -> &DynamicUniformVec<C> {
        &self.uniforms
    }
}

impl<C: Component + AsStd140> Default for ComponentUniforms<C> {
    fn default() -> Self {
        Self {
            uniforms: Default::default(),
        }
    }
}

fn prepare_uniform_components<C: Component>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut component_uniforms: ResMut<ComponentUniforms<C>>,
    components: Query<(Entity, &C)>,
) where
    C: AsStd140 + Clone,
{
    let len = components.iter().len();
    component_uniforms
        .uniforms
        .reserve_and_clear(len, &render_device);
    for (entity, component) in components.iter() {
        commands
            .get_or_spawn(entity)
            .insert(DynamicUniformIndex::<C> {
                index: component_uniforms.uniforms.push(component.clone()),
                marker: PhantomData,
            });
    }

    component_uniforms.uniforms.write_buffer(&render_queue);
}

pub struct ExtractComponentPlugin<C, F = ()>(PhantomData<fn() -> (C, F)>);

impl<C, F> Default for ExtractComponentPlugin<C, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: ExtractComponent> Plugin for ExtractComponentPlugin<C>
where
    <C::Query as WorldQuery>::Fetch: ReadOnlyFetch,
    <C::Filter as WorldQuery>::Fetch: FilterFetch,
{
    fn build(&self, app: &mut App) {
        let system = ExtractComponentSystem::<C>::system(&mut app.world);
        let render_app = app.sub_app(RenderApp);
        render_app.add_system_to_stage(RenderStage::Extract, system);
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

pub struct ExtractComponentSystem<C: ExtractComponent>(PhantomData<C>);

impl<C: ExtractComponent> RunSystem for ExtractComponentSystem<C>
where
    <C::Filter as WorldQuery>::Fetch: FilterFetch,
    <C::Query as WorldQuery>::Fetch: ReadOnlyFetch,
{
    type Param = (
        SCommands,
        Local<'static, usize>,
        SQuery<(Entity, C::Query), C::Filter>,
    );

    fn run((mut commands, mut previous_len, query): SystemParamItem<Self::Param>) {
        let mut values = Vec::with_capacity(*previous_len);
        for (entity, query_item) in query.iter() {
            values.push((entity, (C::extract_component(query_item),)));
        }
        *previous_len = values.len();
        commands.insert_or_spawn_batch(values);
    }
}
