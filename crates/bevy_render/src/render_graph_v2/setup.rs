use std::{mem, ops::Deref, sync::Arc};

use bevy_app::{App, Plugin};

use bevy_ecs::{system::Resource, world::World};

use crate::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::PipelineCache,
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp,
};

use super::{RenderGraph, RenderGraphBuilder, RenderGraphCachedResources};

pub struct RenderGraphPlugin;

#[derive(Resource, ExtractResource, Clone)]
pub struct RenderGraphSetup {
    configurator: Arc<dyn Fn(&mut RenderGraphBuilder) + Send + Sync + 'static>,
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
    world.resource_scope::<RenderGraphCachedResources, ()>(|world, mut resource_cache| {
        world.resource_scope::<PipelineCache, ()>(|world, mut pipeline_cache| {
            let mut graph = RenderGraph::new();
            let device = world.resource::<RenderDevice>();
            let queue = world.resource::<RenderQueue>();

            let mut builder = RenderGraphBuilder {
                graph: &mut graph,
                resource_cache: &mut resource_cache,
                pipeline_cache: &mut pipeline_cache,
                world: &world,
                render_device: &device,
            };

            let configurator = world.resource::<RenderGraphSetup>().configurator.deref();
            (configurator)(&mut builder);
            mem::drop(builder);

            // graph.create_queued_resources(&mut resource_cache, &mut pipeline_cache, device, world);

            // graph.run(world, device, queue);
        });
    });
}
