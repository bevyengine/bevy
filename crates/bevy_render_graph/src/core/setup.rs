use std::{
    any::{type_name, TypeId},
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
};

use bevy_app::{App, Plugin};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    schedule::{IntoSystemConfigs, SystemSet},
    system::{Query, ResMut, Resource},
    world::World,
};

use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::PipelineCache,
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bevy_utils::EntityHashMap;

use super::{RenderGraph, RenderGraphBuilder, RenderGraphCachedResources};

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, SystemSet)]
struct ConfigureRenderGraph;

#[derive(Resource, Default, Clone)]
pub(super) struct RenderGraphConfigurators {
    configurators: EntityHashMap<Entity, ConfiguratorData>,
}

#[derive(Clone)]
struct ConfiguratorData {
    id: TypeId,
    name: &'static str,
    config: Option<Arc<dyn Fn(RenderGraphBuilder) + Send + Sync + 'static>>,
}

impl ConfiguratorData {
    fn new<T: MainConfigurator>(configurator: T) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
            config: Some(Arc::new(move |builder| configurator.configure(builder, ()))),
        }
    }

    fn new_auxiliary<T: for<'g> Configurator<'g>>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
            config: None,
        }
    }
}

impl RenderGraphConfigurators {
    pub fn add<T: MainConfigurator>(&mut self, entity: Entity, configurator: T) {
        if let Some(old_configurator) = self
            .configurators
            .insert(entity, ConfiguratorData::new(configurator))
        {
            if TypeId::of::<T>() != old_configurator.id {
                panic!(
                    "Attempted to add a render graph configurator of type {} to entity {}, which already contains a render graph configurator of type {}",
                    type_name::<T>(),
                    entity,
                    old_configurator.name
                );
            }
        };
    }

    pub fn add_auxiliary<T: for<'g> Configurator<'g>>(&mut self, entity: Entity) {
        if let Some(old_configurator) = self
            .configurators
            .insert(entity, ConfiguratorData::new_auxiliary::<T>())
        {
            if TypeId::of::<T>() != old_configurator.id {
                panic!(
                    "Attempted to add a render graph configurator of type {} to entity {}, which already contains a render graph configurator of type {}",
                    type_name::<T>(),
                    entity,
                    old_configurator.name
                );
            }
        }
    }

    fn reset(&mut self) {
        self.configurators.clear();
    }

    fn iter(
        &self,
    ) -> impl Iterator<
        Item = (
            Entity,
            &(dyn Fn(RenderGraphBuilder) + Send + Sync + 'static),
        ),
    > {
        self.configurators
            .iter()
            .filter_map(|(entity, configurator_data)| {
                configurator_data
                    .config
                    .as_ref()
                    .map(|config| (*entity, config.deref()))
            })
    }
}

pub trait Configurator<'g>: Clone + Component {
    type In: 'g;
    type Out: 'g;

    fn configure(&self, graph: RenderGraphBuilder<'_, 'g>, input: Self::In) -> Self::Out;
}

pub trait MainConfigurator: for<'g> Configurator<'g, In = (), Out = ()> {}

impl<T: for<'g> Configurator<'g, In = (), Out = ()>> MainConfigurator for T {}

pub struct MainConfiguratorPlugin<T: ExtractComponent + MainConfigurator>(PhantomData<T>);

impl<T: ExtractComponent + MainConfigurator> Default for MainConfiguratorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ExtractComponent + MainConfigurator> Plugin for MainConfiguratorPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<T>::default())
            .add_systems(
                Render,
                queue_configurators::<T>
                    .in_set(ConfigureRenderGraph)
                    .ambiguous_with(ConfigureRenderGraph),
            );
    }
}

pub struct AuxiliaryConfiguratorPlugin<T: ExtractComponent + for<'g> Configurator<'g>>(
    PhantomData<T>,
);

impl<T: ExtractComponent + for<'g> Configurator<'g>> Default for AuxiliaryConfiguratorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ExtractComponent + for<'g> Configurator<'g>> Plugin for AuxiliaryConfiguratorPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<T>::default())
            .add_systems(Render, queue_auxiliary_configurators::<T>);
    }
}

fn queue_configurators<T: MainConfigurator>(
    configurators: Query<(Entity, &T)>,
    mut graph_configurators: ResMut<RenderGraphConfigurators>,
) {
    for (entity, config) in &configurators {
        graph_configurators.add(entity, config.clone());
    }
}

fn queue_auxiliary_configurators<T: for<'g> Configurator<'g>>(
    configurators: Query<Entity, With<T>>,
    mut graph_configurators: ResMut<RenderGraphConfigurators>,
) {
    for entity in &configurators {
        graph_configurators.add_auxiliary::<T>(entity);
    }
}

#[derive(Component, ExtractComponent, Clone)]
pub struct CameraRenderGraph {
    configurator: Arc<dyn Fn(RenderGraphBuilder) + Send + Sync + 'static>,
}

impl CameraRenderGraph {
    pub fn new(configurator: impl Fn(RenderGraphBuilder) + Send + Sync + 'static) -> Self {
        Self {
            configurator: Arc::new(configurator),
        }
    }
}

impl<'g> Configurator<'g> for CameraRenderGraph {
    type In = ();
    type Out = ();

    #[inline]
    fn configure(&self, graph: RenderGraphBuilder<'_, 'g>, _: ()) -> Self::Out {
        (self.configurator)(graph);
    }
}

pub struct RenderGraphPlugin;

impl Plugin for RenderGraphPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_plugins(MainConfiguratorPlugin::<CameraRenderGraph>::default())
                .init_resource::<RenderGraphCachedResources>()
                .init_resource::<RenderGraphConfigurators>()
                .add_systems(
                    Render,
                    (
                        run_render_graph
                            .in_set(RenderSet::Render)
                            .after(ConfigureRenderGraph),
                        reset_configurators.in_set(RenderSet::Cleanup),
                    ),
                );
        }
    }
}

fn run_render_graph(world: &mut World) {
    world.resource_scope::<RenderGraphCachedResources, ()>(|world, mut cached_resources| {
        world.resource_scope::<PipelineCache, ()>(|world, mut pipeline_cache| {
            let render_device = world.resource::<RenderDevice>();
            let render_queue = world.resource::<RenderQueue>();
            let configurators = world.resource::<RenderGraphConfigurators>();
            let mut render_graph = RenderGraph::new();

            for (view_entity, config) in configurators.iter() {
                let builder = RenderGraphBuilder {
                    graph: &mut render_graph,
                    world,
                    view: world.entity(view_entity),
                    render_device,
                };

                (config)(builder);
            }

            render_graph.create_queued_resources(
                &mut cached_resources,
                &mut pipeline_cache,
                render_device,
                world,
            );

            render_graph.borrow_cached_resources(&cached_resources);
            render_graph.run(world, render_device, render_queue, &pipeline_cache);
        });
    });
}

fn reset_configurators(mut graph_configurators: ResMut<RenderGraphConfigurators>) {
    graph_configurators.reset();
}
