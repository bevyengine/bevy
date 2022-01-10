use crate::render_graph::{Edge, Node, NodeId, NodeLabel, NodeState, RenderGraphError};
use bevy_ecs::prelude::World;
use bevy_reflect::Uuid;
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

use super::{RunSubGraphError, SlotInfos};

/// A render graph configures the modular, parallel and re-usable render logic.
/// It is a retained and stateless (nodes itself may have their internal state) structure,
/// which can not be modified while it is executed by the graph runner.
///
/// The `RenderGraphRunner` is responsible for executing the entire graph each frame.
///
/// It consists of two main components: [`Nodes`](Node) and [`Edges`](Edge)
///
/// Nodes are responsible for generating draw calls (`record`) and queuing other graphs for execution (`queue_graphs`).
/// Edges specify the order of recording.
///
/// ## Example
/// Here is a simple render graph example with two nodes connected by an edge.
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::prelude::World;
/// # use bevy_render::render_graph::{RenderGraph, Node, RenderGraphContext, NodeRunError};
/// # use bevy_render::renderer::RenderContext;
/// #
/// # struct MyNode;
/// #
/// # impl Node for MyNode {
/// #     fn record(&self, graph: &RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// let mut graph = RenderGraph::new("my_graph");
/// graph.add_node("input_node", MyNode);
/// graph.add_node("output_node", MyNode);
/// graph.add_edge("output_node", "input_node").unwrap();
/// ```
pub struct RenderGraph {
    id: RenderGraphId,
    name: Cow<'static, str>,
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    sub_graphs: HashMap<Cow<'static, str>, RenderGraph>,
    slot_requirements: SlotInfos,
}

impl RenderGraph {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id: RenderGraphId::new(),
            name: name.into(),
            nodes: Default::default(),
            node_names: Default::default(),
            sub_graphs: Default::default(),
            slot_requirements: Default::default(),
        }
    }

    pub fn get_name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn id(&self) -> &RenderGraphId {
        &self.id
    }

    pub fn get_slot_requirements(&self) -> &SlotInfos {
        &self.slot_requirements
    }

    /// Updates all nodes and sub graphs of the render graph. Should be called before executing it.
    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.node.update(world);
        }

        for sub_graph in self.sub_graphs.values_mut() {
            sub_graph.update(world);
        }
    }

    /// ads
    pub fn update_requirements(&mut self) {
        todo!();
    }

    pub fn assert_matching_slots(&self, _infos: &SlotInfos) -> Result<(), RunSubGraphError> {
        todo!()
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
        node_state.name = name.clone();
        self.nodes.insert(id, node_state);
        self.node_names.insert(name, id);
        id
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

    /// Retrieves the [`Node`] referenced by the `label`.
    pub fn get_node<T>(&self, label: impl Into<NodeLabel>) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state(label).and_then(|n| n.node())
    }

    /// Retrieves the [`Node`] referenced by the `label` mutably.
    pub fn get_node_mut<T>(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state_mut(label).and_then(|n| n.node_mut())
    }

    /// Adds the [`Edge::NodeEdge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node`.
    pub fn add_edge(
        &mut self,
        before_node: impl Into<NodeLabel>,
        after_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let before_node_id = self.get_node_id(before_node)?;
        let after_node_id = self.get_node_id(after_node)?;

        let edge = Edge {
            before: before_node_id,
            after: after_node_id,
        };

        self.validate_edge(&edge)?;

        {
            let output_node = self.get_node_state_mut(before_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(after_node_id)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    /// Verifies that the edge is not already existing and
    /// checks that slot edges are connected correctly.
    pub fn validate_edge(&mut self, edge: &Edge) -> Result<(), RenderGraphError> {
        if self.has_edge(edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge.clone()));
        }
        // TODO: Check for cycles

        Ok(())
    }

    /// Checks whether the `edge` already exists in the graph.
    pub fn has_edge(&self, edge: &Edge) -> bool {
        let output_node_state = self.get_node_state(edge.get_after_node());
        let input_node_state = self.get_node_state(edge.get_before_node());
        if let Ok(output_node_state) = output_node_state {
            if output_node_state.edges.output_edges.contains(edge) {
                if let Ok(input_node_state) = input_node_state {
                    if input_node_state.edges.input_edges.contains(edge) {
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
    pub fn iter_node_dependencies(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = &NodeId>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .input_edges
            .iter()
            .map(|edge| (edge, edge.get_after_node()))
            .map(|(edge, _output_node_id)| &edge.before))
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
            .output_edges
            .iter()
            .map(|edge| (edge, edge.get_before_node()))
            .map(move |(edge, input_node_id)| (edge, self.get_node_state(input_node_id).unwrap())))
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.id)?;
            writeln!(f, "  requires: {:?}", node.required_slots)?;
        }

        Ok(())
    }
}

/// A [`RenderGraph`] identifier.
/// It automatically generates its own random uuid.
///
/// This id is used to reference the graph internally (queuing graphs, etc.).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RenderGraphId(Uuid);

impl RenderGraphId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        RenderGraphId(Uuid::new_v4())
    }

    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

/// The resource containing all the [`RenderGraph`]:s
#[derive(Default)]
pub struct RenderGraphs {
    graphs: HashMap<RenderGraphId, RenderGraph>,
    graph_names: HashMap<Cow<'static, str>, RenderGraphId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphLabel {
    Id(RenderGraphId),
    Name(Cow<'static, str>),
}

impl<N: Into<Cow<'static, str>>> From<N> for GraphLabel {
    fn from(name: N) -> Self {
        GraphLabel::Name(name.into())
    }
}

impl From<RenderGraphId> for GraphLabel {
    fn from(id: RenderGraphId) -> Self {
        GraphLabel::Id(id)
    }
}
impl<'a> From<&'a RenderGraphId> for GraphLabel {
    fn from(id: &'a RenderGraphId) -> Self {
        GraphLabel::Id(*id)
    }
}

impl RenderGraphs {
    /// Adds the `graph` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    pub fn add_graph(&mut self, graph: RenderGraph) {
        self.graph_names.insert(graph.name.clone(), graph.id);
        self.graphs.insert(graph.id, graph);
    }

    pub fn get_graph_id(&self, label: impl Into<GraphLabel>) -> Option<RenderGraphId> {
        match label.into() {
            GraphLabel::Id(id) => Some(id),
            GraphLabel::Name(ref k) => Some(*self.graph_names.get(k)?),
        }
    }

    pub fn get_graph_name(&self, label: impl Into<GraphLabel>) -> Option<Cow<'static, str>> {
        match label.into() {
            GraphLabel::Id(id) => self.graphs.get(&id).map(|g| g.name.clone()),
            GraphLabel::Name(k) => Some(k),
        }
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn get_graph(&self, label: impl Into<GraphLabel>) -> Option<&RenderGraph> {
        self.graphs.get(&self.get_graph_id(label)?)
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn get_graph_mut(&mut self, label: impl Into<GraphLabel>) -> Option<&mut RenderGraph> {
        self.graphs.get_mut(&self.get_graph_id(label)?)
    }

    pub fn update(&mut self, world: &mut World) {
        for graph in self.graphs.values_mut() {
            graph.update(world);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{
//         render_graph::{
//             Edge, Node, NodeId, NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError,
//             SlotInfo, SlotType,
//         },
//         renderer::RenderContext,
//     };
//     use bevy_ecs::world::World;
//     use bevy_utils::HashSet;

//     #[derive(Debug)]
//     struct TestNode {
//         inputs: Vec<SlotInfo>,
//         outputs: Vec<SlotInfo>,
//     }

//     impl TestNode {
//         pub fn new(inputs: usize, outputs: usize) -> Self {
//             TestNode {
//                 inputs: (0..inputs)
//                     .map(|i| SlotInfo::new(format!("in_{}", i), SlotType::TextureView))
//                     .collect(),
//                 outputs: (0..outputs)
//                     .map(|i| SlotInfo::new(format!("out_{}", i), SlotType::TextureView))
//                     .collect(),
//             }
//         }
//     }

//     impl Node for TestNode {
//         fn input(&self) -> Vec<SlotInfo> {
//             self.inputs.clone()
//         }

//         fn output(&self) -> Vec<SlotInfo> {
//             self.outputs.clone()
//         }

//         fn run(
//             &self,
//             _: &mut RenderGraphContext,
//             _: &mut RenderContext,
//             _: &World,
//         ) -> Result<(), NodeRunError> {
//             Ok(())
//         }
//     }

//     #[test]
//     fn test_graph_edges() {
//         let mut graph = RenderGraph::default();
//         let a_id = graph.add_node("A", TestNode::new(0, 1));
//         let b_id = graph.add_node("B", TestNode::new(0, 1));
//         let c_id = graph.add_node("C", TestNode::new(1, 1));
//         let d_id = graph.add_node("D", TestNode::new(1, 0));

//         graph.add_slot_edge("A", "out_0", "C", "in_0").unwrap();
//         graph.add_edge("B", "C").unwrap();
//         graph.add_slot_edge("C", 0, "D", 0).unwrap();

//         fn input_nodes(name: Cow<'static, str>, graph: &RenderGraph) -> HashSet<NodeId> {
//             graph
//                 .iter_node_inputs(name)
//                 .unwrap()
//                 .map(|(_edge, node)| node.id)
//                 .collect::<HashSet<NodeId>>()
//         }

//         fn output_nodes(name: Cow<'static, str>, graph: &RenderGraph) -> HashSet<NodeId> {
//             graph
//                 .iter_node_outputs(name)
//                 .unwrap()
//                 .map(|(_edge, node)| node.id)
//                 .collect::<HashSet<NodeId>>()
//         }

//         assert!(input_nodes("A", &graph).is_empty(), "A has no inputs");
//         assert!(
//             output_nodes("A", &graph) == HashSet::from_iter(vec![c_id]),
//             "A outputs to C"
//         );

//         assert!(input_nodes("B", &graph).is_empty(), "B has no inputs");
//         assert!(
//             output_nodes("B", &graph) == HashSet::from_iter(vec![c_id]),
//             "B outputs to C"
//         );

//         assert!(
//             input_nodes("C", &graph) == HashSet::from_iter(vec![a_id, b_id]),
//             "A and B input to C"
//         );
//         assert!(
//             output_nodes("C", &graph) == HashSet::from_iter(vec![d_id]),
//             "C outputs to D"
//         );

//         assert!(
//             input_nodes("D", &graph) == HashSet::from_iter(vec![c_id]),
//             "C inputs to D"
//         );
//         assert!(output_nodes("D", &graph).is_empty(), "D has no outputs");
//     }

//     #[test]
//     fn test_get_node_typed() {
//         struct MyNode {
//             value: usize,
//         }

//         impl Node for MyNode {
//             fn run(
//                 &self,
//                 _: &mut RenderGraphContext,
//                 _: &mut RenderContext,
//                 _: &World,
//             ) -> Result<(), NodeRunError> {
//                 Ok(())
//             }
//         }

//         let mut graph = RenderGraph::default();

//         graph.add_node("A", MyNode { value: 42 });

//         let node: &MyNode = graph.get_node("A").unwrap();
//         assert_eq!(node.value, 42, "node value matches");

//         let result: Result<&TestNode, RenderGraphError> = graph.get_node("A");
//         assert_eq!(
//             result.unwrap_err(),
//             RenderGraphError::WrongNodeType,
//             "expect a wrong node type error"
//         );
//     }

//     #[test]
//     fn test_slot_already_occupied() {
//         let mut graph = RenderGraph::default();

//         graph.add_node("A", TestNode::new(0, 1));
//         graph.add_node("B", TestNode::new(0, 1));
//         graph.add_node("C", TestNode::new(1, 1));

//         graph.add_slot_edge("A", 0, "C", 0).unwrap();
//         assert_eq!(
//             graph.add_slot_edge("B", 0, "C", 0),
//             Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
//                 node: graph.get_node_id("C").unwrap(),
//                 input_slot: 0,
//                 occupied_by_node: graph.get_node_id("A").unwrap(),
//             }),
//             "Adding to a slot that is already occupied should return an error"
//         );
//     }

//     #[test]
//     fn test_edge_already_exists() {
//         let mut graph = RenderGraph::default();

//         graph.add_node("A", TestNode::new(0, 1));
//         graph.add_node("B", TestNode::new(1, 0));

//         graph.add_slot_edge("A", 0, "B", 0).unwrap();
//         assert_eq!(
//             graph.add_slot_edge("A", 0, "B", 0),
//             Err(RenderGraphError::EdgeAlreadyExists(Edge::SlotEdge {
//                 output_node: graph.get_node_id("A").unwrap(),
//                 output_index: 0,
//                 input_node: graph.get_node_id("B").unwrap(),
//                 input_index: 0,
//             })),
//             "Adding to a duplicate edge should return an error"
//         );
//     }
// }
