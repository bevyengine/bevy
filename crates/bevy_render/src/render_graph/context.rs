use crate::render_graph::{SlotType, SlotValue};
use bevy_ecs::entity::Entity;
use std::borrow::Cow;
use thiserror::Error;

use super::{GraphLabel, RenderGraphId, RenderGraphs, SlotValues};

/// A command that signals the graph runner to run the sub graph corresponding to the `name`
/// with the specified `inputs` next.
pub struct RunSubGraph {
    pub id: RenderGraphId,
    pub inputs: SlotValues,
}

#[derive(Default)]
pub struct RunSubGraphs {
    commands: Vec<RunSubGraph>,
}

impl RunSubGraphs {
    pub fn drain(self) -> impl Iterator<Item = RunSubGraph> {
        self.commands.into_iter()
    }

    pub fn run(
        &mut self,
        ctx: &RenderGraphContext,
        name: impl Into<GraphLabel>,
        inputs: impl Into<SlotValues>,
    ) -> Result<(), RunSubGraphError> {
        // TODO: Assert that the inputs match the graph
        let label = name.into();
        let id_option = ctx.graphs.get_graph_id(label.clone());
        let id = match id_option {
            None => {
                return Err(RunSubGraphError::MissingSubGraph(label));
            }
            Some(id) => id,
        };

        self.commands.push(RunSubGraph {
            id,
            inputs: inputs.into(),
        });
        Ok(())
    }
}

/// The context with all graph information required to run a [`Node`](super::Node).
/// This context is created for each node by the `RenderGraphRunner`.
///
/// The slot input can be read from here
pub struct RenderGraphContext<'a> {
    inputs: &'a SlotValues,
    graphs: &'a RenderGraphs,
}

impl<'a> RenderGraphContext<'a> {
    /// Creates a new render graph context.
    pub fn new(inputs: &'a SlotValues, graphs: &'a RenderGraphs) -> Self {
        Self { inputs, graphs }
    }

    /// Returns the input slot values for the node.
    #[inline]
    pub fn inputs(&self) -> &SlotValues {
        self.inputs
    }

    pub fn get_entity(&self, label: impl Into<&'static str>) -> Result<&Entity, SlotError> {
        let label = label.into();

        match self.inputs.get_value(&label)? {
            SlotValue::Entity(e) => Ok(e),
            val => Err(SlotError::MismatchedSlotType {
                label,
                expected: SlotType::Entity,
                actual: val.slot_type(),
            }),
        }
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RunSubGraphError {
    #[error("tried to run a non-existent sub-graph")]
    MissingSubGraph(GraphLabel),
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
        label: &'static str,
        expected: SlotType,
        actual: SlotType,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SlotError {
    #[error("slot does not exist")]
    InvalidSlot(&'static str),
    #[error("attempted to retrieve the wrong type from input slot")]
    MismatchedSlotType {
        label: &'static str,
        expected: SlotType,
        actual: SlotType,
    },
}
