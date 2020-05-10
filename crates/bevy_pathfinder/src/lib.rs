mod device;
mod pathfinder_node;
use bevy_app::{AppBuilder, AppPlugin};
pub use device::*;

use bevy_render::render_graph::RenderGraph;
use pathfinder_node::PathfinderNode;

#[derive(Default)]
pub struct PathfinderPlugin;

impl AppPlugin for PathfinderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut render_graph = app.resources().get_mut::<RenderGraph>().unwrap();
        render_graph.add_node_named("pathfinder", PathfinderNode::default());
    }
}
