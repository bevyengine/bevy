mod compute_pass;
mod configurator;
mod node;
mod resource;

use self::{
    node::RenderGraphNode,
    resource::{RenderGraphResource, RenderGraphResourceId},
};
use crate::renderer::{RenderDevice, RenderQueue};
use bevy_ecs::system::{ResMut, Resource};
use bevy_utils::HashMap;
use wgpu::TextureDescriptor;

// Roadmap:
// 1. Autobuild (and cache) bind group layouts, textures, bind groups, and compute pipelines
// 2. Run the graph in the correct order (figure out how the API should handle command encoders/buffers)
// 3. Buffer and sampler support
// 4. Allow importing external textures
// 5. Temporal resources
// 6. Start porting the engine as a proof of concept/demo, and fill in missing features (e.g. raster nodes)
// 7. Auto-insert CPU profiling, GPU profiling, and GPU debug markers (probably need some concept of a group of render nodes)
// 8. Documentation, write an example, and cleanup

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

    pub(crate) fn build(&mut self, render_device: &RenderDevice) {
        // TODO: Create bind group layouts, pipelines, textures/buffers, and bind groups
    }

    pub(crate) fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
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
