use super::{Edge, Node, NodeId, NodeLabel, NodeState, RenderGraphError, SlotLabel, SystemNode};
use legion::prelude::{Executor, Resources, Schedulable};
use std::{borrow::Cow, collections::HashMap, fmt::Debug};

#[derive(Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    new_node_systems: Vec<Box<dyn Schedulable>>,
    node_system_executor: Option<Executor>,
}

impl RenderGraph {
    pub fn add_node<T>(&mut self, node: T) -> NodeId
    where
        T: Node,
    {
        let id = NodeId::new();
        self.nodes.insert(id, NodeState::new(id, node));
        id
    }

    pub fn add_node_named<T>(&mut self, name: impl Into<Cow<'static, str>>, node: T) -> NodeId
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

    pub fn add_system_node<T>(&mut self, node: T, resources: &mut Resources) -> NodeId
    where
        T: SystemNode + 'static,
    {
        self.new_node_systems.push(node.get_system(resources));
        self.add_node(node)
    }

    pub fn add_system_node_named<T>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        node: T,
        resources: &Resources,
    ) -> NodeId
    where
        T: SystemNode + 'static,
    {
        self.new_node_systems.push(node.get_system(resources));
        self.add_node_named(name, node)
    }

    pub fn get_node_state(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<&NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get(&node_id)
            .ok_or_else(|| RenderGraphError::InvalidNode(label))
    }

    pub fn get_node_state_mut(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get_mut(&node_id)
            .ok_or_else(|| RenderGraphError::InvalidNode(label))
    }

    pub fn get_node_id(&self, label: impl Into<NodeLabel>) -> Result<NodeId, RenderGraphError> {
        let label = label.into();
        match label {
            NodeLabel::Id(id) => Ok(id),
            NodeLabel::Name(ref name) => self
                .node_names
                .get(name)
                .cloned()
                .ok_or_else(|| RenderGraphError::InvalidNode(label)),
        }
    }

    pub fn get_node<T>(&self, label: impl Into<NodeLabel>) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state(label).and_then(|n| n.node())
    }

    pub fn get_node_mut<T>(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state_mut(label).and_then(|n| n.node_mut())
    }

    pub fn add_slot_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        output_slot: impl Into<SlotLabel>,
        input_node: impl Into<NodeLabel>,
        input_slot: impl Into<SlotLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let output_index = self
            .get_node_state(output_node_id)?
            .output_slots
            .get_slot_index(output_slot)?;
        let input_index = self
            .get_node_state(input_node_id)?
            .input_slots
            .get_slot_index(input_slot)?;

        let edge = Edge::SlotEdge {
            output_node: output_node_id,
            output_index,
            input_node: input_node_id,
            input_index,
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

    pub fn add_node_edge(
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

        self.validate_edge(&edge)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    pub fn validate_edge(&mut self, edge: &Edge) -> Result<(), RenderGraphError> {
        if self.has_edge(edge) {
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

                let output_slot = output_node_state.output_slots.get_slot(output_index)?;
                let input_slot = input_node_state.input_slots.get_slot(input_index)?;

                if let Some(Edge::SlotEdge {
                    output_node: current_output_node,
                    ..
                }) = input_node_state.edges.input_edges.iter().find(|e| {
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
                    return Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
                        node: input_node,
                        input_slot: input_index,
                        occupied_by_node: *current_output_node,
                    });
                }

                if output_slot.info.resource_type != input_slot.info.resource_type {
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

    pub fn has_edge(&self, edge: &Edge) -> bool {
        let output_node_state = self.get_node_state(edge.get_output_node());
        let input_node_state = self.get_node_state(edge.get_input_node());
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

    pub fn take_executor(&mut self) -> Option<Executor> {
        // rebuild executor if there are new systems
        if self.new_node_systems.len() > 0 {
            let mut systems = self
                .node_system_executor
                .take()
                .map(|executor| executor.into_vec())
                .unwrap_or_else(|| Vec::new());
            for system in self.new_node_systems.drain(..) {
                systems.push(system);
            }

            self.node_system_executor = Some(Executor::new(systems));
        }

        self.node_system_executor.take()
    }

    pub fn set_executor(&mut self, executor: Executor) {
        self.node_system_executor = Some(executor);
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut NodeState> {
        self.nodes.values_mut()
    }

    pub fn iter_node_inputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .input_edges
            .iter()
            .map(|edge| (edge, edge.get_output_node()))
            .map(move |(edge, output_node_id)| {
                (edge, self.get_node_state(output_node_id).unwrap())
            }))
    }

    pub fn iter_node_outputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .output_edges
            .iter()
            .map(|edge| (edge, edge.get_input_node()))
            .map(move |(edge, input_node_id)| (edge, self.get_node_state(input_node_id).unwrap())))
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.id)?;
            writeln!(f, "  in: {:?}", node.input_slots)?;
            writeln!(f, "  out: {:?}", node.output_slots)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RenderGraph;
    use crate::{
        render_graph::{Edge, Node, NodeId, RenderGraphError, ResourceSlotInfo, ResourceSlots},
        renderer::RenderContext, shader::FieldBindType,
    };
    use legion::prelude::{Resources, World};
    use std::{collections::HashSet, iter::FromIterator};

    #[derive(Debug)]
    struct TestNode {
        inputs: Vec<ResourceSlotInfo>,
        outputs: Vec<ResourceSlotInfo>,
    }

    impl TestNode {
        pub fn new(inputs: usize, outputs: usize) -> Self {
            TestNode {
                inputs: (0..inputs)
                    .map(|i| ResourceSlotInfo {
                        name: format!("in_{}", i).into(),
                        resource_type: FieldBindType::Texture,
                    })
                    .collect(),
                outputs: (0..outputs)
                    .map(|i| ResourceSlotInfo {
                        name: format!("out_{}", i).into(),
                        resource_type: FieldBindType::Texture,
                    })
                    .collect(),
            }
        }
    }

    impl Node for TestNode {
        fn input(&self) -> &[ResourceSlotInfo] {
            &self.inputs
        }

        fn output(&self) -> &[ResourceSlotInfo] {
            &self.outputs
        }
        fn update(
            &mut self,
            _: &World,
            _: &Resources,
            _: &mut dyn RenderContext,
            _: &ResourceSlots,
            _: &mut ResourceSlots,
        ) {
        }
    }

    #[test]
    pub fn test_graph_edges() {
        let mut graph = RenderGraph::default();
        let a_id = graph.add_node_named("A", TestNode::new(0, 1));
        let b_id = graph.add_node_named("B", TestNode::new(0, 1));
        let c_id = graph.add_node_named("C", TestNode::new(1, 1));
        let d_id = graph.add_node_named("D", TestNode::new(1, 0));

        graph.add_slot_edge("A", "out_0", "C", "in_0").unwrap();
        graph.add_node_edge("B", "C").unwrap();
        graph.add_slot_edge("C", 0, "D", 0).unwrap();

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
    pub fn test_get_node_typed() {
        struct MyNode {
            value: usize,
        }

        impl Node for MyNode {
            fn update(
                &mut self,
                _: &World,
                _: &Resources,
                _: &mut dyn RenderContext,
                _: &ResourceSlots,
                _: &mut ResourceSlots,
            ) {
            }
        }

        let mut graph = RenderGraph::default();

        graph.add_node_named("A", MyNode { value: 42 });

        let node: &MyNode = graph.get_node("A").unwrap();
        assert_eq!(node.value, 42, "node value matches");

        let result: Result<&TestNode, RenderGraphError> = graph.get_node("A");
        assert_eq!(
            result.unwrap_err(),
            RenderGraphError::WrongNodeType,
            "expect a wrong node type error"
        );
    }

    #[test]
    pub fn test_slot_already_occupied() {
        let mut graph = RenderGraph::default();

        graph.add_node_named("A", TestNode::new(0, 1));
        graph.add_node_named("B", TestNode::new(0, 1));
        graph.add_node_named("C", TestNode::new(1, 1));

        graph.add_slot_edge("A", 0, "C", 0).unwrap();
        assert_eq!(
            graph.add_slot_edge("B", 0, "C", 0),
            Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
                node: graph.get_node_id("C").unwrap(),
                input_slot: 0,
                occupied_by_node: graph.get_node_id("A").unwrap(),
            }),
            "Adding to a slot that is already occupied should return an error"
        );
    }

    #[test]
    pub fn test_edge_already_exists() {
        let mut graph = RenderGraph::default();

        graph.add_node_named("A", TestNode::new(0, 1));
        graph.add_node_named("B", TestNode::new(1, 0));

        graph.add_slot_edge("A", 0, "B", 0).unwrap();
        assert_eq!(
            graph.add_slot_edge("A", 0, "B", 0),
            Err(RenderGraphError::EdgeAlreadyExists(Edge::SlotEdge {
                output_node: graph.get_node_id("A").unwrap(),
                output_index: 0,
                input_node: graph.get_node_id("B").unwrap(),
                input_index: 0,
            })),
            "Adding to a duplicate edge should return an error"
        );
    }
}
