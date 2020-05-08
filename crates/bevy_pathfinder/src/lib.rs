mod device;
mod resource_loader;
mod pathfinder_node;
mod shaders;
use bevy_app::{AppBuilder, AppPlugin};
pub use device::*;
pub use resource_loader::*;

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
