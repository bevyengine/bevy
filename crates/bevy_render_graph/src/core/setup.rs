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
    configurator: Arc<dyn Fn(&mut RenderGraphBuilder) + Send + Sync + 'static>,
}

impl RenderGraphSetup {
    pub fn set_render_graph(
        &mut self,
        configurator: impl Fn(&mut RenderGraphBuilder) + Send + Sync + 'static,
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
    world.resource_scope::<RenderGraphCachedResources, ()>(|world, mut resource_cache| {
        world.resource_scope::<PipelineCache, ()>(|world, mut pipeline_cache| {
            let render_device = world.resource::<RenderDevice>();
            let render_queue = world.resource::<RenderQueue>();

            let mut builder = RenderGraphBuilder {
                graph: RenderGraph::new(),
                resource_cache: &mut resource_cache,
                pipeline_cache: &mut pipeline_cache,
                world,
                render_device,
            };

            let configurator = world.resource::<RenderGraphSetup>().configurator.deref();
            (configurator)(&mut builder);

            builder.create_queued_resources();
            builder.run(render_queue);
        });
    });
}
