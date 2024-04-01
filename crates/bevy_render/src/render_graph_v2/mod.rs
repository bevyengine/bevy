mod compute_pass;
mod node;
mod resource;
mod view;

use self::{
    node::RenderGraphNode,
    resource::{RenderGraphResource, RenderGraphResourceId},
};
use crate::renderer::{RenderDevice, RenderQueue};
use bevy_ecs::system::{ResMut, Resource};
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

    pub fn add_node(&mut self, node: impl Into<RenderGraphNode>) {
        self.nodes.push(node.into());
    }

    pub fn build(&mut self, render_device: &RenderDevice) {
        // TODO: Create bind group layouts, pipelines, textures/buffers, and bind groups
    }

    pub fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // TODO
    }
}

pub fn run_render_graph(
    mut render_graph: ResMut<RenderGraph>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) {
    render_graph.build(render_device);
    render_graph.run(render_device, render_queue);
}
