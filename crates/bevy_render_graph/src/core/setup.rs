use std::{any::TypeId, marker::PhantomData, ops::Deref, slice::Iter, sync::Arc};

use bevy_app::{App, Plugin};

use bevy_ecs::{
    component::{Component, ComponentId, ComponentIdFor},
    entity::Entity,
    schedule::{IntoSystemConfigs, SystemSet},
    system::{Query, ResMut, Resource},
    world::{EntityWorldMut, World},
};

use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::PipelineCache,
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bevy_utils::HashMap;

use super::{RenderGraph, RenderGraphBuilder, RenderGraphCachedResources};

pub struct RenderGraphPlugin;

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, SystemSet)]
struct ConfigureRenderGraph;

// #[derive(Resource, ExtractResource, Clone)]
// pub struct RenderGraphSetupFunc {
//     configurator: Arc<dyn Fn(RenderGraphBuilder) + Send + Sync + 'static>,
// }
//
// impl RenderGraphSetupFunc {
//     pub fn set_render_graph(
//         &mut self,
//         configurator: impl Fn(RenderGraphBuilder) + Send + Sync + 'static,
//     ) {
//         self.configurator = Arc::new(configurator);
//     }
// }

#[derive(Resource, Default)]
struct RenderGraphConfigurators {
    configurators:
        HashMap<(Entity, ComponentId), Box<dyn Fn(RenderGraphBuilder) + Send + Sync + 'static>>,
}

impl RenderGraphConfigurators {
    fn add<S: for<'g> Configurator<Output<'g> = ()>>(
        &mut self,
        entity: Entity,
        id: &ComponentIdFor<S>,
        configurator: S,
    ) {
        self.configurators.insert(
            (entity, id.get()),
            Box::new(move |builder| configurator.configure(builder)),
        );
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
            .map(|((entity, _), f)| (*entity, f.deref()))
    }
}

pub trait Configurator: Clone + ExtractComponent {
    type Output<'g>;

    fn configure<'g>(&self, graph: RenderGraphBuilder<'_, 'g>) -> Self::Output<'g>;
}

pub struct ConfiguratorPlugin<S: for<'g> Configurator<Output<'g> = ()>>(PhantomData<S>);

impl<S: for<'g> Configurator<Output<'g> = ()>> Default for ConfiguratorPlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: for<'g> Configurator<Output<'g> = ()>> Plugin for ConfiguratorPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<S>::default())
            .add_systems(
                Render,
                queue_render_graphs::<S>
                    .in_set(ConfigureRenderGraph)
                    .ambiguous_with(ConfigureRenderGraph),
            );
    }
}

fn queue_render_graphs<S: for<'g> Configurator<Output<'g> = ()>>(
    configurators: Query<(Entity, &S)>,
    id: ComponentIdFor<S>,
    mut graph_configurators: ResMut<RenderGraphConfigurators>,
) {
    for (entity, config) in &configurators {
        graph_configurators.add(entity, &id, config.clone())
    }
}

fn reset_configurators(mut graph_configurators: ResMut<RenderGraphConfigurators>) {
    graph_configurators.reset();
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

impl Plugin for RenderGraphPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_plugins(ConfiguratorPlugin::<CameraRenderGraph>::default())
                .init_resource::<RenderGraphCachedResources>()
                .init_resource::<RenderGraphConfigurators>()
                .add_systems(
                    Render,
                    run_render_graph
                        .in_set(RenderSet::Render)
                        .after(ConfigureRenderGraph),
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

            for (entity, config) in configurators.iter() {
                let builder = RenderGraphBuilder {
                    graph: &mut render_graph,
                    world,
                    view_entity: world.entity(entity),
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
