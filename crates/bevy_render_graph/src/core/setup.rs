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
    system::{Local, Query, ResMut, Resource},
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
    fn new<T: for<'g> Configurator<Output<'g> = ()>>(configurator: T) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
            config: Some(Arc::new(move |builder| configurator.configure(builder))),
        }
    }

    fn new_auxiliary<T: Configurator>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
            config: None,
        }
    }
}

impl RenderGraphConfigurators {
    pub fn add<T: for<'g> Configurator<Output<'g> = ()>>(
        &mut self,
        entity: Entity,
        configurator: T,
    ) {
        if let Some(old_configurator) = self
            .configurators
            .insert(entity, ConfiguratorData::new(configurator))
        {
            panic!(
                "Attempted to add a render graph configurator of type {} to entity {:?}, which already contains a render graph configurator of type {}",
                type_name::<T>(),
                entity,
                old_configurator.name
            );
        };
    }

    pub fn add_auxiliary<T: Configurator>(&mut self, entity: Entity) {
        if let Some(old_configurator) = self
            .configurators
            .insert(entity, ConfiguratorData::new_auxiliary::<T>())
        {
            panic!(
                "Attempted to add a render graph configurator of type {} to entity {:?}, which already contains a render graph configurator of type {}",
                type_name::<T>(),
                entity,
                old_configurator.name
            );
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

pub trait Configurator: Clone + ExtractComponent<Out = Self> {
    type Output<'g>;

    fn configure<'g>(&self, graph: RenderGraphBuilder<'_, 'g>) -> Self::Output<'g>;
}

pub struct ConfiguratorPlugin<T: for<'g> Configurator<Output<'g> = ()>>(PhantomData<T>);

impl<T: for<'g> Configurator<Output<'g> = ()>> Default for ConfiguratorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: for<'g> Configurator<Output<'g> = ()>> Plugin for ConfiguratorPlugin<T> {
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

pub struct AuxiliaryConfiguratorPlugin<T: Configurator>(PhantomData<T>);

impl<T: Configurator> Default for AuxiliaryConfiguratorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Configurator> Plugin for AuxiliaryConfiguratorPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<T>::default())
            .add_systems(Render, queue_auxiliary_configurators::<T>);
    }
}

fn queue_configurators<T: for<'g> Configurator<Output<'g> = ()>>(
    configurators: Query<(Entity, &T)>,
    mut graph_configurators: ResMut<RenderGraphConfigurators>,
) {
    for (entity, config) in &configurators {
        graph_configurators.add(entity, config.clone());
    }
}

fn queue_auxiliary_configurators<T: Configurator>(
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

impl Configurator for CameraRenderGraph {
    type Output<'g> = ();

    #[inline]
    fn configure<'g>(&self, graph: RenderGraphBuilder<'_, 'g>) -> Self::Output<'g> {
        (self.configurator)(graph);
    }
}

pub struct RenderGraphPlugin;

impl Plugin for RenderGraphPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_plugins(ConfiguratorPlugin::<CameraRenderGraph>::default())
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
                    entity: world.entity(view_entity),
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
