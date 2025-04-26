use bevy_ecs::{resource::Resource, world::World};
use bevy_platform::collections::HashMap;

use crate::render_graph::{
    GraphInput, InternedRenderLabel, InternedRenderSubGraph, IntoRenderNodeArray, RenderLabel,
    RenderSubGraph,
};

use super::{Edge, EdgeExistence, NodeState, Setup, SetupGraphError, SlotLabel};

#[derive(Resource, Default)]
pub struct SetupGraph {
    nodes: HashMap<InternedRenderLabel, NodeState>,
    sub_graphs: HashMap<InternedRenderSubGraph, SetupGraph>,
}

impl SetupGraph {
    /// Updates all nodes and sub graphs of the render graph. Should be called before executing it.
    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.node.update(world);
        }

        for sub_graph in self.sub_graphs.values_mut() {
            sub_graph.update(world);
        }
    }

    /// Removes the [`Edge::SlotEdge`] from the graph. If any nodes or slots do not exist then
    /// nothing happens.
    pub fn remove_slot_edge(
        &mut self,
        output_node: impl RenderLabel,
        output_slot: impl Into<SlotLabel>,
        input_node: impl RenderLabel,
        input_slot: impl Into<SlotLabel>,
    ) -> Result<(), SetupGraphError> {
        let output_slot = output_slot.into();
        let input_slot = input_slot.into();

        let output_node = output_node.intern();
        let input_node = input_node.intern();

        let output_index = self
            .get_node_state(output_node)?
            .output_slots
            .get_slot_index(output_slot.clone())
            .ok_or(SetupGraphError::InvalidOutputNodeSlot(output_slot))?;
        let input_index = self
            .get_node_state(input_node)?
            .input_slots
            .get_slot_index(input_slot.clone())
            .ok_or(SetupGraphError::InvalidInputNodeSlot(input_slot))?;

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

    /// Adds the `sub_graph` with the `label` to the graph.
    /// If the label is already present replaces it instead.
    pub fn add_sub_graph(&mut self, label: impl RenderSubGraph, sub_graph: SetupGraph) {
        self.sub_graphs.insert(label.intern(), sub_graph);
    }

    /// Retrieves the [`NodeState`] referenced by the `label` mutably.
    pub fn get_node_state_mut(
        &mut self,
        label: impl RenderLabel,
    ) -> Result<&mut NodeState, SetupGraphError> {
        let label = label.intern();
        self.nodes
            .get_mut(&label)
            .ok_or(SetupGraphError::InvalidNode(label))
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

    /// Verifies that the edge existence is as expected and
    /// checks that slot edges are connected correctly.
    pub fn validate_edge(
        &mut self,
        edge: &Edge,
        should_exist: EdgeExistence,
    ) -> Result<(), SetupGraphError> {
        if should_exist == EdgeExistence::Exists && !self.has_edge(edge) {
            return Err(SetupGraphError::EdgeDoesNotExist(edge.clone()));
        } else if should_exist == EdgeExistence::DoesNotExist && self.has_edge(edge) {
            return Err(SetupGraphError::EdgeAlreadyExists(edge.clone()));
        }

        match *edge {
            Edge::SlotEdge {
                input_node,
                input_index,
                ..
            } => {
                let input_node_state = self.get_node_state(input_node)?;

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
                        return Err(SetupGraphError::NodeInputSlotAlreadyOccupied {
                            node: input_node,
                            input_slot: input_index,
                            occupied_by_node: *current_output_node,
                        });
                    }
                }
            }
            Edge::NodeEdge { .. } => { /* nothing to validate here */ }
        }

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
    ) -> Result<(), SetupGraphError> {
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
                    SetupGraphError::EdgeAlreadyExists(_) => {}
                    _ => panic!("{err:?}"),
                }
            }
        }
    }

    /// Adds the `node` with the `label` to the graph.
    /// If the label is already present replaces it instead.
    pub fn add_node<T>(&mut self, label: impl RenderLabel, node: T)
    where
        T: Setup,
    {
        let label = label.intern();
        let node_state = NodeState::new(label, node);
        self.nodes.insert(label, node_state);
    }

    /// Retrieves the sub graph corresponding to the `label`.
    pub fn get_sub_graph(&self, label: impl RenderSubGraph) -> Option<&SetupGraph> {
        self.sub_graphs.get(&label.intern())
    }

    /// Retrieves the sub graph corresponding to the `label` mutably.
    pub fn get_sub_graph_mut(&mut self, label: impl RenderSubGraph) -> Option<&mut SetupGraph> {
        self.sub_graphs.get_mut(&label.intern())
    }

    /// Returns an iterator over a tuple of the input edges and the corresponding output nodes
    /// for the node referenced by the label.
    pub fn iter_node_inputs(
        &self,
        label: impl RenderLabel,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, SetupGraphError> {
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
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, SetupGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .output_edges()
            .iter()
            .map(|edge| (edge, edge.get_input_node()))
            .map(move |(edge, input_node)| (edge, self.get_node_state(input_node).unwrap())))
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    /// Retrieves the [`NodeState`] referenced by the `label`.
    pub fn get_node_state(&self, label: impl RenderLabel) -> Result<&NodeState, SetupGraphError> {
        let label = label.intern();
        self.nodes
            .get(&label)
            .ok_or(SetupGraphError::InvalidNode(label))
    }

    pub fn get_input_node(&self) -> Option<&NodeState> {
        self.get_node_state(GraphInput).ok()
    }
}
