use crate::{
    render_graph::{NodeState, RenderGraph, SlotInfos, SlotLabel, SlotType, SlotValue},
    render_resource::{Buffer, Sampler, TextureView},
};
use bevy_ecs::entity::Entity;
use std::borrow::Cow;
use thiserror::Error;

pub struct RunSubGraph {
    pub name: Cow<'static, str>,
    pub inputs: Vec<SlotValue>,
}

pub struct RenderGraphContext<'a> {
    graph: &'a RenderGraph,
    node: &'a NodeState,
    inputs: &'a [SlotValue],
    outputs: &'a mut [Option<SlotValue>],
    run_sub_graphs: Vec<RunSubGraph>,
}

impl<'a> RenderGraphContext<'a> {
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
        }
    }

    #[inline]
    pub fn inputs(&self) -> &[SlotValue] {
        self.inputs
    }

    pub fn input_info(&self) -> &SlotInfos {
        &self.node.input_slots
    }

    pub fn output_info(&self) -> &SlotInfos {
        &self.node.output_slots
    }

    pub fn get_input(&self, label: impl Into<SlotLabel>) -> Result<&SlotValue, InputSlotError> {
        let label = label.into();
        let index = self
            .input_info()
            .get_slot_index(label.clone())
            .ok_or(InputSlotError::InvalidSlot(label))?;
        Ok(&self.inputs[index])
    }

    // TODO: should this return an Arc or a reference?
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

    pub fn run_sub_graph(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        inputs: Vec<SlotValue>,
    ) -> Result<(), RunSubGraphError> {
        let name = name.into();
        let sub_graph = self
            .graph
            .get_sub_graph(&name)
            .ok_or_else(|| RunSubGraphError::MissingSubGraph(name.clone()))?;
        if let Some(input_node) = sub_graph.input_node() {
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

        self.run_sub_graphs.push(RunSubGraph { name, inputs });

        Ok(())
    }

    pub fn finish(self) -> Vec<RunSubGraph> {
        self.run_sub_graphs
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RunSubGraphError {
    #[error("tried to run a non-existent sub-graph")]
    MissingSubGraph(Cow<'static, str>),
    #[error("passed in inputs, but this sub-graph doesn't have any")]
    SubGraphHasNoInputs(Cow<'static, str>),
    #[error("sub graph (name: '{graph_name:?}') could not be run because slot '{slot_name}' at index {slot_index} has no value")]
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
    #[error("slot does not exist")]
    InvalidSlot(SlotLabel),
    #[error("attempted to assign the wrong type to slot")]
    MismatchedSlotType {
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum InputSlotError {
    #[error("slot does not exist")]
    InvalidSlot(SlotLabel),
    #[error("attempted to retrieve the wrong type from input slot")]
    MismatchedSlotType {
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}
