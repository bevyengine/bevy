use alloc::borrow::Cow;

use crate::render_graph::{InternedRenderSubGraph, RenderLabel, RenderSubGraph};
use bevy_ecs::{entity::Entity, intern::Interned};
use thiserror::Error;

use super::{NodeState, SetupGraph, SlotInfos, SlotLabel, SlotValue};

pub struct RunSubGraph {
    pub sub_graph: InternedRenderSubGraph,
    pub inputs: Vec<SlotValue>,
    pub view_entity: Option<Entity>,
}

pub struct SetupGraphContext<'a> {
    graph: &'a SetupGraph,
    node: &'a NodeState,
    inputs: &'a [SlotValue],
    outputs: &'a mut [Option<SlotValue>],
    run_sub_graphs: Vec<RunSubGraph>,
    /// The `view_entity` associated with the render graph being executed
    /// This is optional because you aren't required to have a `view_entity` for a node.
    /// For example, compute shader nodes don't have one.
    /// It should always be set when the [`RenderGraph`] is running on a View.
    view_entity: Option<Entity>,
}

impl<'a> SetupGraphContext<'a> {
    /// Creates a new setup graph context for the `node`.
    pub fn new(
        graph: &'a SetupGraph,
        node: &'a NodeState,
        inputs: &'a [SlotValue],
        outputs: &'a mut [Option<SlotValue>],
    ) -> Self {
        Self {
            graph,
            node,
            inputs,
            outputs,
            run_sub_graphs: Vec::new(),
            view_entity: None,
        }
    }

    /// Returns the input slot values for the node.
    #[inline]
    pub fn inputs(&self) -> &[SlotValue] {
        self.inputs
    }

    /// Returns the [`SlotInfos`] of the inputs.
    pub fn input_info(&self) -> &SlotInfos {
        &self.node.input_slots
    }

    /// Returns the [`SlotInfos`] of the outputs.
    pub fn output_info(&self) -> &SlotInfos {
        &self.node.output_slots
    }

    /// Retrieves the input slot value referenced by the `label`.
    pub fn get_input(&self, label: impl Into<SlotLabel>) -> Result<&SlotValue, InputSlotError> {
        let label = label.into();
        let index = self
            .input_info()
            .get_slot_index(label.clone())
            .ok_or(InputSlotError::InvalidSlot(label))?;
        Ok(&self.inputs[index])
    }

    /// Sets the output slot value referenced by the `label`.
    pub fn set_output(
        &mut self,
        label: impl Into<SlotLabel>,
        value: impl Into<SlotValue>,
    ) -> Result<(), OutputSlotError> {
        let label = label.into();
        let value = value.into();
        let slot_index = self
            .output_info()
            .get_slot_index(label.clone())
            .ok_or_else(|| OutputSlotError::InvalidSlot(label.clone()))?;

        self.outputs[slot_index] = Some(value);
        Ok(())
    }

    pub fn view_entity(&self) -> Entity {
        self.view_entity.unwrap()
    }

    pub fn get_view_entity(&self) -> Option<Entity> {
        self.view_entity
    }

    pub fn set_view_entity(&mut self, view_entity: Entity) {
        self.view_entity = Some(view_entity);
    }

    /// Queues up a sub graph for execution after the node has finished running.
    pub fn run_sub_graph(
        &mut self,
        name: impl RenderSubGraph,
        inputs: Vec<SlotValue>,
        view_entity: Option<Entity>,
    ) -> Result<(), RunSubGraphError> {
        let name = name.intern();
        let sub_graph = self
            .graph
            .get_sub_graph(name)
            .ok_or(RunSubGraphError::MissingSubGraph(name))?;
        if let Some(input_node) = sub_graph.get_input_node() {
            for (i, input_slot) in input_node.input_slots.iter().enumerate() {
                if inputs.get(i).is_none() {
                    return Err(RunSubGraphError::MissingInput {
                        slot_index: i,
                        slot_name: input_slot.name.clone(),
                        graph_name: name,
                    });
                }
            }
        } else if !inputs.is_empty() {
            return Err(RunSubGraphError::SubGraphHasNoInputs(name));
        }

        self.run_sub_graphs.push(RunSubGraph {
            sub_graph: name,
            inputs,
            view_entity,
        });

        Ok(())
    }

    /// Returns a human-readable label for this node, for debugging purposes.
    pub fn label(&self) -> Interned<dyn RenderLabel> {
        self.node.label
    }

    /// Finishes the context for this [`Node`](super::Node) by
    /// returning the sub graphs to run next.
    pub fn finish(self) -> Vec<RunSubGraph> {
        self.run_sub_graphs
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RunSubGraphError {
    #[error("attempted to run sub-graph `{0:?}`, but it does not exist")]
    MissingSubGraph(InternedRenderSubGraph),
    #[error("attempted to pass inputs to sub-graph `{0:?}`, which has no input slots")]
    SubGraphHasNoInputs(InternedRenderSubGraph),
    #[error("sub graph (name: `{graph_name:?}`) could not be run because slot `{slot_name}` at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        graph_name: InternedRenderSubGraph,
    },
    #[error("attempted to use the wrong type for input slot")]
    MismatchedInputSlotType {
        graph_name: InternedRenderSubGraph,
        slot_index: usize,
        label: SlotLabel,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum OutputSlotError {
    #[error("output slot `{0:?}` does not exist")]
    InvalidSlot(SlotLabel),
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum InputSlotError {
    #[error("input slot `{0:?}` does not exist")]
    InvalidSlot(SlotLabel),
}
