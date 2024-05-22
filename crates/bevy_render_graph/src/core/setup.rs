use std::{ops::Deref, sync::Arc};

use bevy_app::{App, Plugin};

use bevy_ecs::{system::Resource, world::World};

use bevy_render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::PipelineCache,
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp,
};

use super::{RenderGraph, RenderGraphBuilder, RenderGraphCachedResources};

pub struct RenderGraphPlugin;

#[derive(Resource, ExtractResource, Clone)]
pub struct RenderGraphSetup {
    configurator: Arc<dyn Fn(RenderGraphBuilder) + Send + Sync + 'static>,
}

impl RenderGraphSetup {
    pub fn set_render_graph(
        &mut self,
        configurator: impl Fn(RenderGraphBuilder) + Send + Sync + 'static,
    ) {
        self.configurator = Arc::new(configurator);
    }
}

impl Plugin for RenderGraphPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_plugins(ExtractResourcePlugin::<RenderGraphSetup>::default());
            render_app.init_resource::<RenderGraphCachedResources>();
            render_app.add_systems(Render, run_render_graph);
        }
    }
}

fn run_render_graph(world: &mut World) {
    world.resource_scope::<RenderGraphCachedResources, ()>(|world, mut cached_resources| {
        world.resource_scope::<PipelineCache, ()>(|world, mut pipeline_cache| {
            let render_device = world.resource::<RenderDevice>();
            let render_queue = world.resource::<RenderQueue>();
            let mut render_graph = RenderGraph::new();

            let builder = RenderGraphBuilder {
                graph: &mut render_graph,
                resource_cache: &mut cached_resources,
                pipeline_cache: &mut pipeline_cache,
                world,
                render_device,
            };

            let configurator = world.resource::<RenderGraphSetup>().configurator.deref();
            (configurator)(builder);

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
