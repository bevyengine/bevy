use crate::{
    render_graph::{
        Edge, Node, NodeId, NodeLabel, NodeRunError, NodeState, RenderGraphContext,
        RenderGraphError, SlotInfo, SlotLabel,
    },
    renderer::RenderContext,
};
use bevy_ecs::prelude::World;
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

use super::SlotInfos;

/// The render graph configures the modular, parallel and re-usable render logic.
/// It is a retained and stateless (nodes itself my have their internal state) structure,
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
/// corresponding nodes. Every render graph can have it’s own optional input node.
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
/// graph.add_node_edge("output_node", "input_node").unwrap();
/// ```
#[derive(Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    sub_graphs: HashMap<Cow<'static, str>, RenderGraph>,
    slot_requirements: SlotInfos
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

    /// ads
    pub fn update_requirements(&mut self) {
        todo!();
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
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let edge = Edge {
            before: output_node_id,
            after: input_node_id,
        };

        self.validate_edge(&edge)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
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
            .map(|(edge, output_node_id)| {
                &edge.before
            }))
    }

    /// Returns an iterator over a tuple of the ouput edges and the corresponding input nodes
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

    /// Adds the `sub_graph` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    pub fn add_sub_graph(&mut self, name: impl Into<Cow<'static, str>>, sub_graph: RenderGraph) {
        self.sub_graphs.insert(name.into(), sub_graph);
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn get_sub_graph(&self, name: impl AsRef<str>) -> Option<&RenderGraph> {
        self.sub_graphs.get(name.as_ref())
    }

    /// Retrieves the sub graph corresponding to the `name` mutably.
    pub fn get_sub_graph_mut(&mut self, name: impl AsRef<str>) -> Option<&mut RenderGraph> {
        self.sub_graphs.get_mut(name.as_ref())
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

//         fn input_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
//             graph
//                 .iter_node_inputs(name)
//                 .unwrap()
//                 .map(|(_edge, node)| node.id)
//                 .collect::<HashSet<NodeId>>()
//         }

//         fn output_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
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
