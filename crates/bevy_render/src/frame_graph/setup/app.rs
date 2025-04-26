use bevy_app::{App, SubApp};
use bevy_ecs::world::FromWorld;
use tracing::warn;

use crate::render_graph::{IntoRenderNodeArray, RenderLabel, RenderSubGraph};

use super::{Setup, SetupGraph};

/// Adds common [`SetupGraph`] operations to [`SubApp`] (and [`App`]).
pub trait SetupGraphApp {
    // Add a sub graph to the [`SetupGraph`]
    fn add_setup_sub_graph(&mut self, sub_graph: impl RenderSubGraph) -> &mut Self;
    /// Add a [`Setup`] to the [`SetupGraph`]:
    /// * Create the [`Setup`] using the [`FromWorld`] implementation
    /// * Add it to the graph
    fn add_setup_graph_node<T: Setup + FromWorld>(
        &mut self,
        sub_graph: impl RenderSubGraph,
        node_label: impl RenderLabel,
    ) -> &mut Self;
    /// Automatically add the required node edges based on the given ordering
    fn add_setup_graph_edges<const N: usize>(
        &mut self,
        sub_graph: impl RenderSubGraph,
        edges: impl IntoRenderNodeArray<N>,
    ) -> &mut Self;

    /// Add node edge to the specified graph
    fn add_setup_graph_edge(
        &mut self,
        sub_graph: impl RenderSubGraph,
        output_node: impl RenderLabel,
        input_node: impl RenderLabel,
    ) -> &mut Self;
}

impl SetupGraphApp for SubApp {
    fn add_setup_graph_node<T: Setup + FromWorld>(
        &mut self,
        sub_graph: impl RenderSubGraph,
        node_label: impl RenderLabel,
    ) -> &mut Self {
        let sub_graph = sub_graph.intern();
        let node = T::from_world(self.world_mut());
        let mut setup_graph = self.world_mut().get_resource_mut::<SetupGraph>().expect(
            "SetupGraph not found. Make sure you are using add_render_graph_node on the RenderApp",
        );
        if let Some(graph) = setup_graph.get_sub_graph_mut(sub_graph) {
            graph.add_node(node_label, node);
        } else {
            warn!(
                "Tried adding a render graph node to {sub_graph:?} but the sub graph doesn't exist"
            );
        }
        self
    }

    fn add_setup_graph_edges<const N: usize>(
        &mut self,
        sub_graph: impl RenderSubGraph,
        edges: impl IntoRenderNodeArray<N>,
    ) -> &mut Self {
        let sub_graph = sub_graph.intern();
        let mut setup_graph = self.world_mut().get_resource_mut::<SetupGraph>().expect(
            "SetupGraph not found. Make sure you are using add_render_graph_edges on the RenderApp",
        );
        if let Some(graph) = setup_graph.get_sub_graph_mut(sub_graph) {
            graph.add_node_edges(edges);
        } else {
            warn!(
                "Tried adding render graph edges to {sub_graph:?} but the sub graph doesn't exist"
            );
        }
        self
    }

    fn add_setup_graph_edge(
        &mut self,
        sub_graph: impl RenderSubGraph,
        output_node: impl RenderLabel,
        input_node: impl RenderLabel,
    ) -> &mut Self {
        let sub_graph = sub_graph.intern();
        let mut setup_graph = self.world_mut().get_resource_mut::<SetupGraph>().expect(
            "SetupGraph not found. Make sure you are using add_render_graph_edge on the RenderApp",
        );
        if let Some(graph) = setup_graph.get_sub_graph_mut(sub_graph) {
            graph.add_node_edge(output_node, input_node);
        } else {
            warn!(
                "Tried adding a render graph edge to {sub_graph:?} but the sub graph doesn't exist"
            );
        }
        self
    }

    fn add_setup_sub_graph(&mut self, sub_graph: impl RenderSubGraph) -> &mut Self {
        let mut setup_graph = self.world_mut().get_resource_mut::<SetupGraph>().expect(
            "SetupGraph not found. Make sure you are using add_render_sub_graph on the RenderApp",
        );
        setup_graph.add_sub_graph(sub_graph, SetupGraph::default());
        self
    }
}

impl SetupGraphApp for App {
    fn add_setup_graph_node<T: Setup + FromWorld>(
        &mut self,
        sub_graph: impl RenderSubGraph,
        node_label: impl RenderLabel,
    ) -> &mut Self {
        SubApp::add_setup_graph_node::<T>(self.main_mut(), sub_graph, node_label);
        self
    }

    fn add_setup_graph_edge(
        &mut self,
        sub_graph: impl RenderSubGraph,
        output_node: impl RenderLabel,
        input_node: impl RenderLabel,
    ) -> &mut Self {
        SubApp::add_setup_graph_edge(self.main_mut(), sub_graph, output_node, input_node);
        self
    }

    fn add_setup_graph_edges<const N: usize>(
        &mut self,
        sub_graph: impl RenderSubGraph,
        edges: impl IntoRenderNodeArray<N>,
    ) -> &mut Self {
        SubApp::add_setup_graph_edges(self.main_mut(), sub_graph, edges);
        self
    }

    fn add_setup_sub_graph(&mut self, sub_graph: impl RenderSubGraph) -> &mut Self {
        SubApp::add_setup_sub_graph(self.main_mut(), sub_graph);
        self
    }
}
