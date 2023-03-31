use crate::{
    render_graph::{NodeState, RenderGraph, SlotInfos, SlotLabel, SlotType, SlotValue},
    render_resource::{Buffer, Sampler, TextureView},
};
use bevy_ecs::entity::Entity;
use std::borrow::Cow;
use thiserror::Error;

/// A command that signals the graph runner to run the sub graph corresponding to the `name`
/// with the specified `inputs` next.
pub struct RunSubGraph {
    pub name: Cow<'static, str>,
    pub inputs: Vec<SlotValue>,
    pub view_entity: Option<Entity>,
}

/// The context with all graph information required to run a [`Node`](super::Node).
/// This context is created for each node by the `RenderGraphRunner`.
///
/// The slot input can be read from here and the outputs must be written back to the context for
/// passing them onto the next node.
///
/// Sub graphs can be queued for running by adding a [`RunSubGraph`] command to the context.
/// After the node has finished running the graph runner is responsible for executing the sub graphs.
pub struct RenderGraphContext<'a> {
    graph: &'a RenderGraph,
    node: &'a NodeState,
    inputs: &'a [SlotValue],
    outputs: &'a mut [Option<SlotValue>],
    run_sub_graphs: Vec<RunSubGraph>,
    /// The view_entity associated with the render graph being executed
    /// This is optional because you aren't required to have a view_entity for a node.
    /// For example, compute shader nodes don't have one.
    /// It should always be set when the RenderGraph is running on a View.
    view_entity: Option<Entity>,
}

impl<'a> RenderGraphContext<'a> {
    /// Creates a new render graph context for the `node`.
    pub fn new(
        graph: &'a RenderGraph,
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

    // TODO: should this return an Arc or a reference?
    /// Retrieves the input slot value referenced by the `label` as a [`TextureView`].
    pub fn get_input_texture(
        &self,
        label: impl Into<SlotLabel>,
    ) -> Result<&TextureView, InputSlotError> {
        let label = label.into();
        match self.get_input(label.clone())? {
            SlotValue::TextureView(value) => Ok(value),
            value => Err(InputSlotError::MismatchedSlotType {
                label,
                actual: value.slot_type(),
                expected: SlotType::TextureView,
            }),
        }
    }

    /// Retrieves the input slot value referenced by the `label` as a [`Sampler`].
    pub fn get_input_sampler(
        &self,
        label: impl Into<SlotLabel>,
    ) -> Result<&Sampler, InputSlotError> {
        let label = label.into();
        match self.get_input(label.clone())? {
            SlotValue::Sampler(value) => Ok(value),
            value => Err(InputSlotError::MismatchedSlotType {
                label,
                actual: value.slot_type(),
                expected: SlotType::Sampler,
            }),
        }
    }

    /// Retrieves the input slot value referenced by the `label` as a [`Buffer`].
    pub fn get_input_buffer(&self, label: impl Into<SlotLabel>) -> Result<&Buffer, InputSlotError> {
        let label = label.into();
        match self.get_input(label.clone())? {
            SlotValue::Buffer(value) => Ok(value),
            value => Err(InputSlotError::MismatchedSlotType {
                label,
                actual: value.slot_type(),
                expected: SlotType::Buffer,
            }),
        }
    }

    /// Retrieves the input slot value referenced by the `label` as an [`Entity`].
    pub fn get_input_entity(&self, label: impl Into<SlotLabel>) -> Result<Entity, InputSlotError> {
        let label = label.into();
        match self.get_input(label.clone())? {
            SlotValue::Entity(value) => Ok(*value),
            value => Err(InputSlotError::MismatchedSlotType {
                label,
                actual: value.slot_type(),
                expected: SlotType::Entity,
            }),
        }
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
        let slot = self
            .output_info()
            .get_slot(slot_index)
            .expect("slot is valid");
        if value.slot_type() != slot.slot_type {
            return Err(OutputSlotError::MismatchedSlotType {
                label,
                actual: slot.slot_type,
                expected: value.slot_type(),
            });
        }
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
        name: impl Into<Cow<'static, str>>,
        inputs: Vec<SlotValue>,
        view_entity: Option<Entity>,
    ) -> Result<(), RunSubGraphError> {
        let name = name.into();
        let sub_graph = self
            .graph
            .get_sub_graph(&name)
            .ok_or_else(|| RunSubGraphError::MissingSubGraph(name.clone()))?;
        if let Some(input_node) = sub_graph.get_input_node() {
            for (i, input_slot) in input_node.input_slots.iter().enumerate() {
                if let Some(input_value) = inputs.get(i) {
                    if input_slot.slot_type != input_value.slot_type() {
                        return Err(RunSubGraphError::MismatchedInputSlotType {
                            graph_name: name,
                            slot_index: i,
                            actual: input_value.slot_type(),
                            expected: input_slot.slot_type,
                            label: input_slot.name.clone().into(),
                        });
                    }
                } else {
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
            name,
            inputs,
            view_entity,
        });

        Ok(())
    }

    /// Finishes the context for this [`Node`](super::Node) by
    /// returning the sub graphs to run next.
    pub fn finish(self) -> Vec<RunSubGraph> {
        self.run_sub_graphs
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RunSubGraphError {
    #[error("attempted to run sub-graph `{0}`, but it does not exist")]
    MissingSubGraph(Cow<'static, str>),
    #[error("attempted to pass inputs to sub-graph `{0}`, which has no input slots")]
    SubGraphHasNoInputs(Cow<'static, str>),
    #[error("sub graph (name: `{graph_name:?}`) could not be run because slot `{slot_name}` at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        graph_name: Cow<'static, str>,
    },
    #[error("attempted to use the wrong type for input slot")]
    MismatchedInputSlotType {
        graph_name: Cow<'static, str>,
        slot_index: usize,
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum OutputSlotError {
    #[error("output slot `{0:?}` does not exist")]
    InvalidSlot(SlotLabel),
    #[error("attempted to output a value of type `{actual}` to output slot `{label:?}`, which has type `{expected}`")]
    MismatchedSlotType {
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum InputSlotError {
    #[error("input slot `{0:?}` does not exist")]
    InvalidSlot(SlotLabel),
    #[error("attempted to retrieve a value of type `{actual}` from input slot `{label:?}`, which has type `{expected}`")]
    MismatchedSlotType {
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}
