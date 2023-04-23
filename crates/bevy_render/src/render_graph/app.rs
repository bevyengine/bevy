use bevy_app::App;
use bevy_ecs::world::FromWorld;
use bevy_log::warn;

use super::{Node, RenderGraph};

/// Adds common [`RenderGraph`] operations to [`App`].
pub trait RenderGraphApp {
    // Add a sub graph to the [`RenderGraph`]
    fn add_render_sub_graph(&mut self, sub_graph_name: &'static str) -> &mut Self;
    /// Add a [`Node`] to the [`RenderGraph`]:
    /// * Create the [`Node`] using the [`FromWorld`] implementation
    /// * Add it to the graph
    fn add_render_graph_node<T: Node + FromWorld>(
        &mut self,
        sub_graph_name: &'static str,
        node_name: &'static str,
    ) -> &mut Self;
    /// Automatically add the required node edges based on the given ordering
    fn add_render_graph_edges(
        &mut self,
        sub_graph_name: &'static str,
        edges: &[&'static str],
    ) -> &mut Self;
    /// Add node edge to the specified graph
    fn add_render_graph_edge(
        &mut self,
        sub_graph_name: &'static str,
        output_edge: &'static str,
        input_edge: &'static str,
    ) -> &mut Self;
}

impl RenderGraphApp for App {
    fn add_render_graph_node<T: Node + FromWorld>(
        &mut self,
        sub_graph_name: &'static str,
        node_name: &'static str,
    ) -> &mut Self {
        let node = T::from_world(&mut self.world);
        let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_node on the RenderApp",
        );
        if let Some(graph) = render_graph.get_sub_graph_mut(sub_graph_name) {
            graph.add_node(node_name, node);
        } else {
            warn!("Tried adding a render graph node to {sub_graph_name} but the sub graph doesn't exist");
        }
        self
    }

    fn add_render_graph_edges(
        &mut self,
        sub_graph_name: &'static str,
        edges: &[&'static str],
    ) -> &mut Self {
        let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_edges on the RenderApp",
        );
        if let Some(graph) = render_graph.get_sub_graph_mut(sub_graph_name) {
            graph.add_node_edges(edges);
        } else {
            warn!("Tried adding render graph edges to {sub_graph_name} but the sub graph doesn't exist");
        }
        self
    }

    fn add_render_graph_edge(
        &mut self,
        sub_graph_name: &'static str,
        output_edge: &'static str,
        input_edge: &'static str,
    ) -> &mut Self {
        let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_edge on the RenderApp",
        );
        if let Some(graph) = render_graph.get_sub_graph_mut(sub_graph_name) {
            graph.add_node_edge(output_edge, input_edge);
        } else {
            warn!("Tried adding a render graph edge to {sub_graph_name} but the sub graph doesn't exist");
        }
        self
    }

    fn add_render_sub_graph(&mut self, sub_graph_name: &'static str) -> &mut Self {
        let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_sub_graph on the RenderApp",
        );
        render_graph.add_sub_graph(sub_graph_name, RenderGraph::default());
        self
    }
}
