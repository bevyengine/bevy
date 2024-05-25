use crate::{
    render_graph::{
        Edge, Node, NodeRunError, NodeState, RenderGraphContext, RenderGraphError, RenderLabel,
        SlotInfo, SlotLabel,
    },
    renderer::RenderContext,
};
use bevy_ecs::{define_label, intern::Interned, prelude::World, system::Resource};
use bevy_utils::HashMap;
use std::fmt::Debug;

use super::{EdgeExistence, InternedRenderLabel, IntoRenderNodeArray};

pub use bevy_render_macros::RenderSubGraph;

define_label!(
    /// A strongly-typed class of labels used to identify a [`SubGraph`] in a render graph.
    RenderSubGraph,
    RENDER_SUB_GRAPH_INTERNER
);

/// A shorthand for `Interned<dyn RenderSubGraph>`.
pub type InternedRenderSubGraph = Interned<dyn RenderSubGraph>;

/// The render graph configures the modular and re-usable render logic.
/// It is a retained and stateless (nodes themselves may have their own internal state) structure,
/// which can not be modified while it is executed by the graph runner.
///
/// The [`RenderGraphRunner`](crate::renderer::graph_runner::RenderGraphRunner) is responsible for executing the entire graph each frame.
/// It will execute each node in the graph in the correct order, based on the edges between the nodes.
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
/// ```ignore
/// # TODO: Remove when #10645 is fixed
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::prelude::World;
/// # use bevy_render::render_graph::{RenderGraph, RenderLabel, Node, RenderGraphContext, NodeRunError};
/// # use bevy_render::renderer::RenderContext;
/// #
/// #[derive(RenderLabel)]
/// enum Labels {
///     A,
///     B,
/// }
///
/// # struct MyNode;
/// #
/// # impl Node for MyNode {
/// #     fn run(&self, graph: &mut RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// let mut graph = RenderGraph::default();
/// graph.add_node(Labels::A, MyNode);
/// graph.add_node(Labels::B, MyNode);
/// graph.add_node_edge(Labels::B, Labels::A);
/// ```
#[derive(Resource, Default)]
pub struct RenderGraph {
    nodes: HashMap<InternedRenderLabel, NodeState>,
    sub_graphs: HashMap<InternedRenderSubGraph, RenderGraph>,
}

/// The label for the input node of a graph. Used to connect other nodes to it.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct GraphInput;

impl RenderGraph {
    /// Updates all nodes and sub graphs of the render graph. Should be called before executing it.
    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.node.update(world);
        }

        for sub_graph in self.sub_graphs.values_mut() {
            sub_graph.update(world);
        }
    }

    /// Creates an [`GraphInputNode`] with the specified slots if not already present.
    pub fn set_input(&mut self, inputs: Vec<SlotInfo>) {
        assert!(
            matches!(
                self.get_node_state(GraphInput),
                Err(RenderGraphError::InvalidNode(_))
            ),
            "Graph already has an input node"
        );

        self.add_node(GraphInput, GraphInputNode { inputs });
    }

    /// Returns the [`NodeState`] of the input node of this graph.
    ///
    /// # See also
    ///
    /// - [`input_node`](Self::input_node) for an unchecked version.
    #[inline]
    pub fn get_input_node(&self) -> Option<&NodeState> {
        self.get_node_state(GraphInput).ok()
    }

    /// Returns the [`NodeState`] of the input node of this graph.
    ///
    /// # Panics
    ///
    /// Panics if there is no input node set.
    ///
    /// # See also
    ///
    /// - [`get_input_node`](Self::get_input_node) for a version which returns an [`Option`] instead.
    #[inline]
    pub fn input_node(&self) -> &NodeState {
        self.get_input_node().unwrap()
    }

    /// Adds the `node` with the `label` to the graph.
    /// If the label is already present replaces it instead.
    pub fn add_node<T>(&mut self, label: impl RenderLabel, node: T)
    where
        T: Node,
    {
        let label = label.intern();
        let node_state = NodeState::new(label, node);
        self.nodes.insert(label, node_state);
    }

    /// Add `node_edge`s based on the order of the given `edges` array.
    ///
    /// Defining an edge that already exists is not considered an error with this api.
    /// It simply won't create a new edge.
    pub fn add_node_edges<const N: usize>(&mut self, edges: impl IntoRenderNodeArray<N>) {
        for window in edges.into_array().windows(2) {
            let [a, b] = window else {
                break;
            };
            if let Err(err) = self.try_add_node_edge(*a, *b) {
                match err {
                    // Already existing edges are very easy to produce with this api
                    // and shouldn't cause a panic
                    RenderGraphError::EdgeAlreadyExists(_) => {}
                    _ => panic!("{err:?}"),
                }
            }
        }
    }

    /// Removes the `node` with the `label` from the graph.
    /// If the label does not exist, nothing happens.
    pub fn remove_node(&mut self, label: impl RenderLabel) -> Result<(), RenderGraphError> {
        let label = label.intern();
        if let Some(node_state) = self.nodes.remove(&label) {
            // Remove all edges from other nodes to this one. Note that as we're removing this
            // node, we don't need to remove its input edges
            for input_edge in node_state.edges.input_edges() {
                match input_edge {
                    Edge::SlotEdge { output_node, .. }
                    | Edge::NodeEdge {
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
            for output_edge in node_state.edges.output_edges() {
                match output_edge {
                    Edge::SlotEdge {
                        output_node: _,
                        output_index: _,
                        input_node,
                        input_index: _,
                    }
                    | Edge::NodeEdge {
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

        Ok(())
    }

    /// Retrieves the [`NodeState`] referenced by the `label`.
    pub fn get_node_state(&self, label: impl RenderLabel) -> Result<&NodeState, RenderGraphError> {
        let label = label.intern();
        self.nodes
            .get(&label)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    /// Retrieves the [`NodeState`] referenced by the `label` mutably.
    pub fn get_node_state_mut(
        &mut self,
        label: impl RenderLabel,
    ) -> Result<&mut NodeState, RenderGraphError> {
        let label = label.intern();
        self.nodes
            .get_mut(&label)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    /// Retrieves the [`Node`] referenced by the `label`.
    pub fn get_node<T>(&self, label: impl RenderLabel) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state(label).and_then(|n| n.node())
    }

    /// Retrieves the [`Node`] referenced by the `label` mutably.
    pub fn get_node_mut<T>(&mut self, label: impl RenderLabel) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state_mut(label).and_then(|n| n.node_mut())
    }

    /// Adds the [`Edge::SlotEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node` and also connects the `output_slot` to the `input_slot`.
    ///
    /// Fails if any invalid [`RenderLabel`]s or [`SlotLabel`]s are given.
    ///
    /// # See also
    ///
    /// - [`add_slot_edge`](Self::add_slot_edge) for an infallible version.
    pub fn try_add_slot_edge(
        &mut self,
        output_node: impl RenderLabel,
        output_slot: impl Into<SlotLabel>,
        input_node: impl RenderLabel,
        input_slot: impl Into<SlotLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_slot = output_slot.into();
        let input_slot = input_slot.into();

        let output_node = output_node.intern();
        let input_node = input_node.intern();

        let output_index = self
            .get_node_state(output_node)?
            .output_slots
            .get_slot_index(output_slot.clone())
            .ok_or(RenderGraphError::InvalidOutputNodeSlot(output_slot))?;
        let input_index = self
            .get_node_state(input_node)?
            .input_slots
            .get_slot_index(input_slot.clone())
            .ok_or(RenderGraphError::InvalidInputNodeSlot(input_slot))?;

        let edge = Edge::SlotEdge {
            output_node,
            output_index,
            input_node,
            input_index,
        };

        self.validate_edge(&edge, EdgeExistence::DoesNotExist)?;

        {
            let output_node = self.get_node_state_mut(output_node)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    /// Adds the [`Edge::SlotEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node` and also connects the `output_slot` to the `input_slot`.
    ///
    /// # Panics
    ///
    /// Any invalid [`RenderLabel`]s or [`SlotLabel`]s are given.
    ///
    /// # See also
    ///
    /// - [`try_add_slot_edge`](Self::try_add_slot_edge) for a fallible version.
    pub fn add_slot_edge(
        &mut self,
        output_node: impl RenderLabel,
        output_slot: impl Into<SlotLabel>,
        input_node: impl RenderLabel,
        input_slot: impl Into<SlotLabel>,
    ) {
        self.try_add_slot_edge(output_node, output_slot, input_node, input_slot)
            .unwrap();
    }

    /// Removes the [`Edge::SlotEdge`] from the graph. If any nodes or slots do not exist then
    /// nothing happens.
    pub fn remove_slot_edge(
        &mut self,
        output_node: impl RenderLabel,
        output_slot: impl Into<SlotLabel>,
        input_node: impl RenderLabel,
        input_slot: impl Into<SlotLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_slot = output_slot.into();
        let input_slot = input_slot.into();

        let output_node = output_node.intern();
        let input_node = input_node.intern();

        let output_index = self
            .get_node_state(output_node)?
            .output_slots
            .get_slot_index(output_slot.clone())
            .ok_or(RenderGraphError::InvalidOutputNodeSlot(output_slot))?;
        let input_index = self
            .get_node_state(input_node)?
            .input_slots
            .get_slot_index(input_slot.clone())
            .ok_or(RenderGraphError::InvalidInputNodeSlot(input_slot))?;

        let edge = Edge::SlotEdge {
            output_node,
            output_index,
            input_node,
            input_index,
        };

        self.validate_edge(&edge, EdgeExistence::Exists)?;

        {
            let output_node = self.get_node_state_mut(output_node)?;
            output_node.edges.remove_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node)?;
        input_node.edges.remove_input_edge(edge)?;

        Ok(())
    }

    /// Adds the [`Edge::NodeEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node`.
    ///
    /// Fails if any invalid [`RenderLabel`] is given.
    ///
    /// # See also
    ///
    /// - [`add_node_edge`](Self::add_node_edge) for an infallible version.
    pub fn try_add_node_edge(
        &mut self,
        output_node: impl RenderLabel,
        input_node: impl RenderLabel,
    ) -> Result<(), RenderGraphError> {
        let output_node = output_node.intern();
        let input_node = input_node.intern();

        let edge = Edge::NodeEdge {
            output_node,
            input_node,
        };

        self.validate_edge(&edge, EdgeExistence::DoesNotExist)?;

        {
            let output_node = self.get_node_state_mut(output_node)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    /// Adds the [`Edge::NodeEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node`.
    ///
    /// # Panics
    ///
    /// Panics if any invalid [`RenderLabel`] is given.
    ///
    /// # See also
    ///
    /// - [`try_add_node_edge`](Self::try_add_node_edge) for a fallible version.
    pub fn add_node_edge(&mut self, output_node: impl RenderLabel, input_node: impl RenderLabel) {
        self.try_add_node_edge(output_node, input_node).unwrap();
    }

    /// Removes the [`Edge::NodeEdge`] from the graph. If either node does not exist then nothing
    /// happens.
    pub fn remove_node_edge(
        &mut self,
        output_node: impl RenderLabel,
        input_node: impl RenderLabel,
    ) -> Result<(), RenderGraphError> {
        let output_node = output_node.intern();
        let input_node = input_node.intern();

        let edge = Edge::NodeEdge {
            output_node,
            input_node,
        };

        self.validate_edge(&edge, EdgeExistence::Exists)?;

        {
            let output_node = self.get_node_state_mut(output_node)?;
            output_node.edges.remove_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node)?;
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

        match *edge {
            Edge::SlotEdge {
                output_node,
                output_index,
                input_node,
                input_index,
            } => {
                let output_node_state = self.get_node_state(output_node)?;
                let input_node_state = self.get_node_state(input_node)?;

                let output_slot = output_node_state
                    .output_slots
                    .get_slot(output_index)
                    .ok_or(RenderGraphError::InvalidOutputNodeSlot(SlotLabel::Index(
                        output_index,
                    )))?;
                let input_slot = input_node_state.input_slots.get_slot(input_index).ok_or(
                    RenderGraphError::InvalidInputNodeSlot(SlotLabel::Index(input_index)),
                )?;

                if let Some(Edge::SlotEdge {
                    output_node: current_output_node,
                    ..
                }) = input_node_state.edges.input_edges().iter().find(|e| {
                    if let Edge::SlotEdge {
                        input_index: current_input_index,
                        ..
                    } = e
                    {
                        input_index == *current_input_index
                    } else {
                        false
                    }
                }) {
                    if should_exist == EdgeExistence::DoesNotExist {
                        return Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
                            node: input_node,
                            input_slot: input_index,
                            occupied_by_node: *current_output_node,
                        });
                    }
                }

                if output_slot.slot_type != input_slot.slot_type {
                    return Err(RenderGraphError::MismatchedNodeSlots {
                        output_node,
                        output_slot: output_index,
                        input_node,
                        input_slot: input_index,
                    });
                }
            }
            Edge::NodeEdge { .. } => { /* nothing to validate here */ }
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
    pub fn iter_sub_graphs(&self) -> impl Iterator<Item = (InternedRenderSubGraph, &RenderGraph)> {
        self.sub_graphs.iter().map(|(name, graph)| (*name, graph))
    }

    /// Returns an iterator over the sub graphs, that allows modifying each value.
    pub fn iter_sub_graphs_mut(
        &mut self,
    ) -> impl Iterator<Item = (InternedRenderSubGraph, &mut RenderGraph)> {
        self.sub_graphs
            .iter_mut()
            .map(|(name, graph)| (*name, graph))
    }

    /// Returns an iterator over a tuple of the input edges and the corresponding output nodes
    /// for the node referenced by the label.
    pub fn iter_node_inputs(
        &self,
        label: impl RenderLabel,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .input_edges()
            .iter()
            .map(|edge| (edge, edge.get_output_node()))
            .map(move |(edge, output_node)| (edge, self.get_node_state(output_node).unwrap())))
    }

    /// Returns an iterator over a tuple of the output edges and the corresponding input nodes
    /// for the node referenced by the label.
    pub fn iter_node_outputs(
        &self,
        label: impl RenderLabel,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .output_edges()
            .iter()
            .map(|edge| (edge, edge.get_input_node()))
            .map(move |(edge, input_node)| (edge, self.get_node_state(input_node).unwrap())))
    }

    /// Adds the `sub_graph` with the `label` to the graph.
    /// If the label is already present replaces it instead.
    pub fn add_sub_graph(&mut self, label: impl RenderSubGraph, sub_graph: RenderGraph) {
        self.sub_graphs.insert(label.intern(), sub_graph);
    }

    /// Removes the `sub_graph` with the `label` from the graph.
    /// If the label does not exist then nothing happens.
    pub fn remove_sub_graph(&mut self, label: impl RenderSubGraph) {
        self.sub_graphs.remove(&label.intern());
    }

    /// Retrieves the sub graph corresponding to the `label`.
    pub fn get_sub_graph(&self, label: impl RenderSubGraph) -> Option<&RenderGraph> {
        self.sub_graphs.get(&label.intern())
    }

    /// Retrieves the sub graph corresponding to the `label` mutably.
    pub fn get_sub_graph_mut(&mut self, label: impl RenderSubGraph) -> Option<&mut RenderGraph> {
        self.sub_graphs.get_mut(&label.intern())
    }

    /// Retrieves the sub graph corresponding to the `label`.
    ///
    /// # Panics
    ///
    /// Panics if any invalid subgraph label is given.
    ///
    /// # See also
    ///
    /// - [`get_sub_graph`](Self::get_sub_graph) for a fallible version.
    pub fn sub_graph(&self, label: impl RenderSubGraph) -> &RenderGraph {
        let label = label.intern();
        self.sub_graphs
            .get(&label)
            .unwrap_or_else(|| panic!("Subgraph {label:?} not found"))
    }

    /// Retrieves the sub graph corresponding to the `label` mutably.
    ///
    /// # Panics
    ///
    /// Panics if any invalid subgraph label is given.
    ///
    /// # See also
    ///
    /// - [`get_sub_graph_mut`](Self::get_sub_graph_mut) for a fallible version.
    pub fn sub_graph_mut(&mut self, label: impl RenderSubGraph) -> &mut RenderGraph {
        let label = label.intern();
        self.sub_graphs
            .get_mut(&label)
            .unwrap_or_else(|| panic!("Subgraph {label:?} not found"))
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.label)?;
            writeln!(f, "  in: {:?}", node.input_slots)?;
            writeln!(f, "  out: {:?}", node.output_slots)?;
        }

        Ok(())
    }
}

/// A [`Node`] which acts as an entry point for a [`RenderGraph`] with custom inputs.
/// It has the same input and output slots and simply copies them over when run.
pub struct GraphInputNode {
    inputs: Vec<SlotInfo>,
}

impl Node for GraphInputNode {
    fn input(&self) -> Vec<SlotInfo> {
        self.inputs.clone()
    }

    fn output(&self) -> Vec<SlotInfo> {
        self.inputs.clone()
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        for i in 0..graph.inputs().len() {
            let input = graph.inputs()[i].clone();
            graph.set_output(i, input)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        render_graph::{
            node::IntoRenderNodeArray, Edge, InternedRenderLabel, Node, NodeRunError, RenderGraph,
            RenderGraphContext, RenderGraphError, RenderLabel, SlotInfo, SlotType,
        },
        renderer::RenderContext,
    };
    use bevy_ecs::world::{FromWorld, World};
    use bevy_utils::HashSet;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    enum TestLabel {
        A,
        B,
        C,
        D,
    }

    #[derive(Debug)]
    struct TestNode {
        inputs: Vec<SlotInfo>,
        outputs: Vec<SlotInfo>,
    }

    impl TestNode {
        pub fn new(inputs: usize, outputs: usize) -> Self {
            TestNode {
                inputs: (0..inputs)
                    .map(|i| SlotInfo::new(format!("in_{i}"), SlotType::TextureView))
                    .collect(),
                outputs: (0..outputs)
                    .map(|i| SlotInfo::new(format!("out_{i}"), SlotType::TextureView))
                    .collect(),
            }
        }
    }

    impl Node for TestNode {
        fn input(&self) -> Vec<SlotInfo> {
            self.inputs.clone()
        }

        fn output(&self) -> Vec<SlotInfo> {
            self.outputs.clone()
        }

        fn run(
            &self,
            _: &mut RenderGraphContext,
            _: &mut RenderContext,
            _: &World,
        ) -> Result<(), NodeRunError> {
            Ok(())
        }
    }

    fn input_nodes(label: impl RenderLabel, graph: &RenderGraph) -> HashSet<InternedRenderLabel> {
        graph
            .iter_node_inputs(label)
            .unwrap()
            .map(|(_edge, node)| node.label)
            .collect::<HashSet<InternedRenderLabel>>()
    }

    fn output_nodes(label: impl RenderLabel, graph: &RenderGraph) -> HashSet<InternedRenderLabel> {
        graph
            .iter_node_outputs(label)
            .unwrap()
            .map(|(_edge, node)| node.label)
            .collect::<HashSet<InternedRenderLabel>>()
    }

    #[test]
    fn test_graph_edges() {
        let mut graph = RenderGraph::default();
        graph.add_node(TestLabel::A, TestNode::new(0, 1));
        graph.add_node(TestLabel::B, TestNode::new(0, 1));
        graph.add_node(TestLabel::C, TestNode::new(1, 1));
        graph.add_node(TestLabel::D, TestNode::new(1, 0));

        graph.add_slot_edge(TestLabel::A, "out_0", TestLabel::C, "in_0");
        graph.add_node_edge(TestLabel::B, TestLabel::C);
        graph.add_slot_edge(TestLabel::C, 0, TestLabel::D, 0);

        assert!(
            input_nodes(TestLabel::A, &graph).is_empty(),
            "A has no inputs"
        );
        assert_eq!(
            output_nodes(TestLabel::A, &graph),
            HashSet::from_iter((TestLabel::C,).into_array()),
            "A outputs to C"
        );

        assert!(
            input_nodes(TestLabel::B, &graph).is_empty(),
            "B has no inputs"
        );
        assert_eq!(
            output_nodes(TestLabel::B, &graph),
            HashSet::from_iter((TestLabel::C,).into_array()),
            "B outputs to C"
        );

        assert_eq!(
            input_nodes(TestLabel::C, &graph),
            HashSet::from_iter((TestLabel::A, TestLabel::B).into_array()),
            "A and B input to C"
        );
        assert_eq!(
            output_nodes(TestLabel::C, &graph),
            HashSet::from_iter((TestLabel::D,).into_array()),
            "C outputs to D"
        );

        assert_eq!(
            input_nodes(TestLabel::D, &graph),
            HashSet::from_iter((TestLabel::C,).into_array()),
            "C inputs to D"
        );
        assert!(
            output_nodes(TestLabel::D, &graph).is_empty(),
            "D has no outputs"
        );
    }

    #[test]
    fn test_get_node_typed() {
        struct MyNode {
            value: usize,
        }

        impl Node for MyNode {
            fn run(
                &self,
                _: &mut RenderGraphContext,
                _: &mut RenderContext,
                _: &World,
            ) -> Result<(), NodeRunError> {
                Ok(())
            }
        }

        let mut graph = RenderGraph::default();

        graph.add_node(TestLabel::A, MyNode { value: 42 });

        let node: &MyNode = graph.get_node(TestLabel::A).unwrap();
        assert_eq!(node.value, 42, "node value matches");

        let result: Result<&TestNode, RenderGraphError> = graph.get_node(TestLabel::A);
        assert_eq!(
            result.unwrap_err(),
            RenderGraphError::WrongNodeType,
            "expect a wrong node type error"
        );
    }

    #[test]
    fn test_slot_already_occupied() {
        let mut graph = RenderGraph::default();

        graph.add_node(TestLabel::A, TestNode::new(0, 1));
        graph.add_node(TestLabel::B, TestNode::new(0, 1));
        graph.add_node(TestLabel::C, TestNode::new(1, 1));

        graph.add_slot_edge(TestLabel::A, 0, TestLabel::C, 0);
        assert_eq!(
            graph.try_add_slot_edge(TestLabel::B, 0, TestLabel::C, 0),
            Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
                node: TestLabel::C.intern(),
                input_slot: 0,
                occupied_by_node: TestLabel::A.intern(),
            }),
            "Adding to a slot that is already occupied should return an error"
        );
    }

    #[test]
    fn test_edge_already_exists() {
        let mut graph = RenderGraph::default();

        graph.add_node(TestLabel::A, TestNode::new(0, 1));
        graph.add_node(TestLabel::B, TestNode::new(1, 0));

        graph.add_slot_edge(TestLabel::A, 0, TestLabel::B, 0);
        assert_eq!(
            graph.try_add_slot_edge(TestLabel::A, 0, TestLabel::B, 0),
            Err(RenderGraphError::EdgeAlreadyExists(Edge::SlotEdge {
                output_node: TestLabel::A.intern(),
                output_index: 0,
                input_node: TestLabel::B.intern(),
                input_index: 0,
            })),
            "Adding to a duplicate edge should return an error"
        );
    }

    #[test]
    fn test_add_node_edges() {
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
        graph.add_node(TestLabel::A, SimpleNode);
        graph.add_node(TestLabel::B, SimpleNode);
        graph.add_node(TestLabel::C, SimpleNode);

        graph.add_node_edges((TestLabel::A, TestLabel::B, TestLabel::C));

        assert_eq!(
            output_nodes(TestLabel::A, &graph),
            HashSet::from_iter((TestLabel::B,).into_array()),
            "A -> B"
        );
        assert_eq!(
            input_nodes(TestLabel::B, &graph),
            HashSet::from_iter((TestLabel::A,).into_array()),
            "A -> B"
        );
        assert_eq!(
            output_nodes(TestLabel::B, &graph),
            HashSet::from_iter((TestLabel::C,).into_array()),
            "B -> C"
        );
        assert_eq!(
            input_nodes(TestLabel::C, &graph),
            HashSet::from_iter((TestLabel::B,).into_array()),
            "B -> C"
        );
    }
}
