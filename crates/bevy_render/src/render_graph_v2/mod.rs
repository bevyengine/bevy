use crate::renderer::{RenderDevice, RenderQueue};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{ResMut, Resource},
    world::World,
};
use bevy_utils::HashMap;
use wgpu::TextureDescriptor;

#[derive(Resource)]
pub struct RenderGraph {
    next_id: u16,
    resources: HashMap<RenderGraphResourceId, TextureDescriptor<'static>>,
    nodes: Vec<RenderGraphNode>,
}

impl RenderGraph {
    pub fn create_resource(
        &mut self,
        descriptor: TextureDescriptor<'static>,
    ) -> RenderGraphResource {
        let id = self.next_id;
        self.next_id += 1;

        self.resources.insert(id, descriptor);

        RenderGraphResource { id, generation: 0 }
    }

    pub fn add_node(
        &mut self,
        resources: &[RenderGraphResource],
        runner: impl FnOnce(&RenderDevice, &RenderQueue) + Send + Sync + 'static,
    ) {
        self.nodes.push(RenderGraphNode {
            resources: resources.into(),
            runner: Box::new(runner),
        });
    }

    pub fn build(&mut self, render_device: &RenderDevice) {
        // TODO: Create bind group layouts, pipelines, textures/buffers, and bind groups
    }

    pub fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // TODO
    }
}

pub struct RenderGraphResource {
    id: RenderGraphResourceId,
    generation: u16,
}

impl Clone for RenderGraphResource {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            generation: self.generation + 1,
        }
    }
}

type RenderGraphResourceId = u16;

struct RenderGraphNode {
    resources: Box<[RenderGraphResource]>,
    runner: Box<dyn FnOnce(&RenderDevice, &RenderQueue) + Send + Sync + 'static>,
}

#[derive(Component)]
pub struct ViewRenderGraphConfigurator(
    pub Box<dyn Fn(Entity, &World, &mut RenderGraph) + Send + Sync>,
);

pub fn setup_view_render_graph_nodes(world: &mut World) {
    world.resource_scope::<RenderGraph, _>(|world, mut render_graph| {
        // TODO: Probably want to cache the QueryState
        for (view_entity, configurator) in world
            .query::<(Entity, &ViewRenderGraphConfigurator)>()
            .iter(world)
        {
            (configurator.0)(view_entity, world, &mut render_graph);
        }
    });
}

pub fn run_render_graph(
    mut render_graph: ResMut<RenderGraph>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) {
    render_graph.build(render_device);
    render_graph.run(render_device, render_queue);
}
