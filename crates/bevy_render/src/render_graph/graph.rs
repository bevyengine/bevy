use crate::{
    render_graph::{
        Edge, Node, NodeId, NodeLabel, NodeRunError, NodeState, RenderGraphContext,
        RenderGraphError,
    },
    renderer::RenderContext,
};
use bevy_ecs::{prelude::World, system::Resource};
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

use super::EdgeExistence;

/// The render graph configures the modular, parallel and re-usable render logic.
/// It is a retained and stateless (nodes themselves may have their own internal state) structure,
/// which can not be modified while it is executed by the graph runner.
///
/// The `RenderGraphRunner` is responsible for executing the entire graph each frame.
///
/// It consists of three main components: [`Nodes`](Node), [`Edges`](Edge)
/// and [`Slots`](super::SlotType).
///
/// Nodes are responsible for generating draw calls and operating on input and output slots.
/// Edges specify the order of execution for nodes and connect input and output slots together.
/// Slots describe the render resources created or used by the nodes.
///
/// Additionally a render graph can contain multiple sub graphs, which are run by the
/// corresponding nodes. Every render graph can have its own optional input node.
///
/// ## Example
/// Here is a simple render graph example with two nodes connected by a node edge.
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::prelude::World;
/// # use bevy_render::render_graph::{RenderGraph, Node, RenderGraphContext, NodeRunError};
/// # use bevy_render::renderer::RenderContext;
/// #
/// # struct MyNode;
/// #
/// # impl Node for MyNode {
/// #     fn run(&self, graph: &mut RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// let mut graph = RenderGraph::default();
/// graph.add_node("input_node", MyNode);
/// graph.add_node("output_node", MyNode);
/// graph.add_node_edge("output_node", "input_node");
/// ```
#[derive(Resource, Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    sub_graphs: HashMap<Cow<'static, str>, RenderGraph>,
}

impl RenderGraph {
    /// The name of the [`GraphInputNode`] of this graph. Used to connect other nodes to it.
    pub const INPUT_NODE_NAME: &'static str = "GraphInputNode";

    /// Updates all nodes and sub graphs of the render graph. Should be called before executing it.
    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.node.update(world);
        }

        for sub_graph in self.sub_graphs.values_mut() {
            sub_graph.update(world);
        }
    }

    /// Adds the `node` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    pub fn add_node<T>(&mut self, name: impl Into<Cow<'static, str>>, node: T) -> NodeId
    where
        T: Node,
    {
        let id = NodeId::new();
        let name = name.into();
        let mut node_state = NodeState::new(id, node);
        node_state.name = Some(name.clone());
        self.nodes.insert(id, node_state);
        self.node_names.insert(name, id);
        id
    }

    /// Adds the `node` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    /// Also adds `node_edges` based on the order of the given `edges`.
    pub fn add_node_with_edges<T>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        node: T,
        edges: &[&'static str],
    ) -> NodeId
    where
        T: Node,
    {
        let id = self.add_node(name, node);
        for window in edges.windows(2) {
            let [a, b] = window else { break; };
            self.add_node_edge(*a, *b);
        }
        id
    }

    /// Removes the `node` with the `name` from the graph.
    /// If the name is does not exist, nothing happens.
    pub fn remove_node(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(), RenderGraphError> {
        let name = name.into();
        if let Some(id) = self.node_names.remove(&name) {
            if let Some(node_state) = self.nodes.remove(&id) {
                // Remove all edges from other nodes to this one. Note that as we're removing this
                // node, we don't need to remove its input edges
                for input_edge in node_state.edges.input_edges().iter() {
                    match input_edge {
                        Edge::NodeEdge {
                            input_node: _,
                            output_node,
                        } => {
                            if let Ok(output_node) = self.get_node_state_mut(*output_node) {
                                output_node.edges.remove_output_edge(input_edge.clone())?;
                            }
                        }
                    }
                }
                // Remove all edges from this node to other nodes. Note that as we're removing this
                // node, we don't need to remove its output edges
                for output_edge in node_state.edges.output_edges().iter() {
                    match output_edge {
                        Edge::NodeEdge {
                            output_node: _,
                            input_node,
                        } => {
                            if let Ok(input_node) = self.get_node_state_mut(*input_node) {
                                input_node.edges.remove_input_edge(output_edge.clone())?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Retrieves the [`NodeState`] referenced by the `label`.
    pub fn get_node_state(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<&NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    /// Retrieves the [`NodeState`] referenced by the `label` mutably.
    pub fn get_node_state_mut(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get_mut(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    /// Retrieves the [`NodeId`] referenced by the `label`.
    pub fn get_node_id(&self, label: impl Into<NodeLabel>) -> Result<NodeId, RenderGraphError> {
        let label = label.into();
        match label {
            NodeLabel::Id(id) => Ok(id),
            NodeLabel::Name(ref name) => self
                .node_names
                .get(name)
                .cloned()
                .ok_or(RenderGraphError::InvalidNode(label)),
        }
    }

    /// Adds the [`Edge::NodeEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node`.
    ///
    /// Fails if any invalid [`NodeLabel`] is given.
    ///
    /// # See also
    ///
    /// - [`add_node_edge`](Self::add_node_edge) for an infallible version.
    pub fn try_add_node_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let edge = Edge::NodeEdge {
            output_node: output_node_id,
            input_node: input_node_id,
        };

        self.validate_edge(&edge, EdgeExistence::DoesNotExist)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    /// Adds the [`Edge::NodeEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node`.
    ///
    /// # Panics
    ///
    /// Panics if any invalid [`NodeLabel`] is given.
    ///
    /// # See also
    ///
    /// - [`try_add_node_edge`](Self::try_add_node_edge) for a fallible version.
    pub fn add_node_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) {
        self.try_add_node_edge(output_node, input_node).unwrap();
    }

    /// Removes the [`Edge::NodeEdge`] from the graph. If either node does not exist then nothing
    /// happens.
    pub fn remove_node_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let edge = Edge::NodeEdge {
            output_node: output_node_id,
            input_node: input_node_id,
        };

        self.validate_edge(&edge, EdgeExistence::Exists)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.remove_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.remove_input_edge(edge)?;

        Ok(())
    }

    /// Verifies that the edge existence is as expected and
    /// checks that slot edges are connected correctly.
    pub fn validate_edge(
        &mut self,
        edge: &Edge,
        should_exist: EdgeExistence,
    ) -> Result<(), RenderGraphError> {
        if should_exist == EdgeExistence::Exists && !self.has_edge(edge) {
            return Err(RenderGraphError::EdgeDoesNotExist(edge.clone()));
        } else if should_exist == EdgeExistence::DoesNotExist && self.has_edge(edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge.clone()));
        }

        Ok(())
    }

    /// Checks whether the `edge` already exists in the graph.
    pub fn has_edge(&self, edge: &Edge) -> bool {
        let output_node_state = self.get_node_state(edge.get_output_node());
        let input_node_state = self.get_node_state(edge.get_input_node());
        if let Ok(output_node_state) = output_node_state {
            if output_node_state.edges.output_edges().contains(edge) {
                if let Ok(input_node_state) = input_node_state {
                    if input_node_state.edges.input_edges().contains(edge) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Returns an iterator over the [`NodeStates`](NodeState).
    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    /// Returns an iterator over the [`NodeStates`](NodeState), that allows modifying each value.
    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut NodeState> {
        self.nodes.values_mut()
    }

    /// Returns an iterator over the sub graphs.
    pub fn iter_sub_graphs(&self) -> impl Iterator<Item = (&str, &RenderGraph)> {
        self.sub_graphs
            .iter()
            .map(|(name, graph)| (name.as_ref(), graph))
    }

    /// Returns an iterator over the sub graphs, that allows modifying each value.
    pub fn iter_sub_graphs_mut(&mut self) -> impl Iterator<Item = (&str, &mut RenderGraph)> {
        self.sub_graphs
            .iter_mut()
            .map(|(name, graph)| (name.as_ref(), graph))
    }

    /// Returns an iterator over a tuple of the input edges and the corresponding output nodes
    /// for the node referenced by the label.
    pub fn iter_node_inputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .input_edges()
            .iter()
            .map(|edge| (edge, edge.get_output_node()))
            .map(move |(edge, output_node_id)| {
                (edge, self.get_node_state(output_node_id).unwrap())
            }))
    }

    /// Returns an iterator over a tuple of the output edges and the corresponding input nodes
    /// for the node referenced by the label.
    pub fn iter_node_outputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .output_edges()
            .iter()
            .map(|edge| (edge, edge.get_input_node()))
            .map(move |(edge, input_node_id)| (edge, self.get_node_state(input_node_id).unwrap())))
    }

    /// Adds the `sub_graph` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    pub fn add_sub_graph(&mut self, name: impl Into<Cow<'static, str>>, sub_graph: RenderGraph) {
        self.sub_graphs.insert(name.into(), sub_graph);
    }

    /// Removes the `sub_graph` with the `name` from the graph.
    /// If the name does not exist then nothing happens.
    pub fn remove_sub_graph(&mut self, name: impl Into<Cow<'static, str>>) {
        self.sub_graphs.remove(&name.into());
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn get_sub_graph(&self, name: impl AsRef<str>) -> Option<&RenderGraph> {
        self.sub_graphs.get(name.as_ref())
    }

    /// Retrieves the sub graph corresponding to the `name` mutably.
    pub fn get_sub_graph_mut(&mut self, name: impl AsRef<str>) -> Option<&mut RenderGraph> {
        self.sub_graphs.get_mut(name.as_ref())
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn sub_graph(&self, name: impl AsRef<str>) -> &RenderGraph {
        self.sub_graphs.get(name.as_ref()).unwrap()
    }

    /// Retrieves the sub graph corresponding to the `name` mutably.
    pub fn sub_graph_mut(&mut self, name: impl AsRef<str>) -> &mut RenderGraph {
        self.sub_graphs.get_mut(name.as_ref()).unwrap()
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?} {:?}", node.id, node.name)?;
        }

        Ok(())
    }
}

/// A [`Node`] which acts as an entry point for a [`RenderGraph`] with custom inputs.
/// It has the same input and output slots and simply copies them over when run.
pub struct GraphInputNode {}

impl Node for GraphInputNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        render_graph::{
            Edge, Node, NodeId, NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError,
        },
        renderer::RenderContext,
    };
    use bevy_ecs::world::{FromWorld, World};
    use bevy_utils::HashSet;

    #[derive(Debug)]
    struct TestNode {}

    impl Node for TestNode {
        fn run(
            &self,
            _: &mut RenderGraphContext,
            _: &mut RenderContext,
            _: &World,
        ) -> Result<(), NodeRunError> {
            Ok(())
        }
    }

    fn input_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
        graph
            .iter_node_inputs(name)
            .unwrap()
            .map(|(_edge, node)| node.id)
            .collect::<HashSet<NodeId>>()
    }

    fn output_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
        graph
            .iter_node_outputs(name)
            .unwrap()
            .map(|(_edge, node)| node.id)
            .collect::<HashSet<NodeId>>()
    }

    #[test]
    fn test_graph_edges() {
        let mut graph = RenderGraph::default();
        let a_id = graph.add_node("A", TestNode {});
        let b_id = graph.add_node("B", TestNode {});
        let c_id = graph.add_node("C", TestNode {});
        let d_id = graph.add_node("D", TestNode {});

        graph.add_node_edge("A", "C");
        graph.add_node_edge("B", "C");
        graph.add_node_edge("C", "D");

        assert!(input_nodes("A", &graph).is_empty(), "A has no inputs");
        assert!(
            output_nodes("A", &graph) == HashSet::from_iter(vec![c_id]),
            "A outputs to C"
        );

        assert!(input_nodes("B", &graph).is_empty(), "B has no inputs");
        assert!(
            output_nodes("B", &graph) == HashSet::from_iter(vec![c_id]),
            "B outputs to C"
        );

        assert!(
            input_nodes("C", &graph) == HashSet::from_iter(vec![a_id, b_id]),
            "A and B input to C"
        );
        assert!(
            output_nodes("C", &graph) == HashSet::from_iter(vec![d_id]),
            "C outputs to D"
        );

        assert!(
            input_nodes("D", &graph) == HashSet::from_iter(vec![c_id]),
            "C inputs to D"
        );
        assert!(output_nodes("D", &graph).is_empty(), "D has no outputs");
    }

    #[test]
    fn test_edge_already_exists() {
        let mut graph = RenderGraph::default();

        graph.add_node("A", TestNode {});
        graph.add_node("B", TestNode {});

        graph.add_node_edge("A", "B");
        assert_eq!(
            graph.try_add_node_edge("A", "B"),
            Err(RenderGraphError::EdgeAlreadyExists(Edge::NodeEdge {
                output_node: graph.get_node_id("A").unwrap(),
                input_node: graph.get_node_id("B").unwrap(),
            })),
            "Adding to a duplicate edge should return an error"
        );
    }

    #[test]
    fn test_add_node_with_edges() {
        struct SimpleNode;
        impl Node for SimpleNode {
            fn run(
                &self,
                _graph: &mut RenderGraphContext,
                _render_context: &mut RenderContext,
                _world: &World,
            ) -> Result<(), NodeRunError> {
                Ok(())
            }
        }
        impl FromWorld for SimpleNode {
            fn from_world(_world: &mut World) -> Self {
                Self
            }
        }

        let mut graph = RenderGraph::default();
        let a_id = graph.add_node("A", SimpleNode);
        let c_id = graph.add_node("C", SimpleNode);

        // A and C need to exist first
        let b_id = graph.add_node_with_edges("B", SimpleNode, &["A", "B", "C"]);

        assert!(
            output_nodes("A", &graph) == HashSet::from_iter(vec![b_id]),
            "A -> B"
        );
        assert!(
            input_nodes("B", &graph) == HashSet::from_iter(vec![a_id]),
            "B -> C"
        );
        assert!(
            output_nodes("B", &graph) == HashSet::from_iter(vec![c_id]),
            "B -> C"
        );
        assert!(
            input_nodes("C", &graph) == HashSet::from_iter(vec![b_id]),
            "B -> C"
        );
    }
}
