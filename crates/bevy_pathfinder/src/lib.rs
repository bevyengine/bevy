mod device;
mod pathfinder_node;
use bevy_app::{AppBuilder, AppPlugin};
pub use device::*;

use bevy_render::{
    base_render_graph,
    render_graph::{
        nodes::{WindowSwapChainNode, WindowTextureNode},
        RenderGraph,
    },
};
use pathfinder_node::PathfinderNode;

#[derive(Default)]
pub struct PathfinderPlugin;

impl AppPlugin for PathfinderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut render_graph = app.resources().get_mut::<RenderGraph>().unwrap();
        render_graph.add_node_named("pathfinder", PathfinderNode::default());
        render_graph
            .add_slot_edge(
                base_render_graph::node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                "pathfinder",
                PathfinderNode::IN_COLOR_TEXTURE,
            )
            .unwrap();
        render_graph
            .add_slot_edge(
                base_render_graph::node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                "pathfinder",
                PathfinderNode::IN_DEPTH_STENCIL_TEXTURE,
            )
            .unwrap();
    }
}
