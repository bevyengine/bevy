use bevy_app::App;
use bevy_ecs::world::FromWorld;

use super::{Node, RenderGraph};

/// Utility function to add a [`Node`] to the [`RenderGraph`]
/// * Create the [`Node`] using the [`FromWorld`] implementation
/// * Add it to the graph
/// * Automatically add the required node edges based on the given ordering
pub fn add_node<T: Node + FromWorld>(
    render_app: &mut App,
    sub_graph_name: &'static str,
    node_name: &'static str,
    edges: &[&'static str],
) {
    let node = T::from_world(&mut render_app.world);
    let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

    let graph = render_graph.get_sub_graph_mut(sub_graph_name).unwrap();
    graph.add_node(node_name, node);
    graph.add_node_edges(edges);
}
