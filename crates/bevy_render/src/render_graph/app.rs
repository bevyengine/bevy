use bevy_app::App;
use bevy_ecs::world::FromWorld;

use super::{Node, RenderGraph};

/// Adds common [`RenderGraph`] operations to [`App`].
pub trait RenderGraphApp {
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

        let graph = render_graph.sub_graph_mut(sub_graph_name);
        graph.add_node(node_name, node);
        self
    }

    fn add_render_graph_edges(
        &mut self,
        sub_graph_name: &'static str,
        edges: &[&'static str],
    ) -> &mut Self {
        let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_node on the RenderApp",
        );
        let graph = render_graph.sub_graph_mut(sub_graph_name);
        graph.add_node_edges(edges);
        self
    }

    fn add_render_graph_edge(
        &mut self,
        sub_graph_name: &'static str,
        output_edge: &'static str,
        input_edge: &'static str,
    ) -> &mut Self {
        let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_node on the RenderApp",
        );
        let graph = render_graph.sub_graph_mut(sub_graph_name);
        graph.add_node_edge(output_edge, input_edge);
        self
    }
}
