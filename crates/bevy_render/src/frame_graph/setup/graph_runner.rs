use bevy_ecs::{prelude::Entity, world::World};
use bevy_platform::collections::HashMap;
#[cfg(feature = "trace")]
use tracing::info_span;

use alloc::{borrow::Cow, collections::VecDeque};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

use crate::{
    frame_graph::FrameGraph,
    render_graph::{InternedRenderLabel, InternedRenderSubGraph},
};

use super::{Edge, NodeState, SetupGraph, SetupGraphContext, SetupRunError, SlotValue};

pub struct SetupGraphRunner;

#[derive(Error, Debug)]
pub enum SetupGraphRunnerError {
    #[error(transparent)]
    SetupRunError(#[from] SetupRunError),
    #[error("node output slot not set (index {slot_index}, name {slot_name})")]
    EmptyNodeOutputSlot {
        type_name: &'static str,
        slot_index: usize,
        slot_name: Cow<'static, str>,
    },
    #[error("graph '{sub_graph:?}' could not be run because slot '{slot_name}' at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        sub_graph: Option<InternedRenderSubGraph>,
    },
    #[error(
        "node (name: '{node_name:?}') has {slot_count} input slots, but was provided {value_count} values"
    )]
    MismatchedInputCount {
        node_name: InternedRenderLabel,
        slot_count: usize,
        value_count: usize,
    },
}

impl SetupGraphRunner {
    pub fn run(
        graph: &SetupGraph,
        frame_graph: &mut FrameGraph,
        world: &World,
    ) -> Result<(), SetupGraphRunnerError> {
        Self::run_graph(graph, None, frame_graph, world, &[], None)?;

        Ok(())
    }

    fn run_graph<'w>(
        graph: &SetupGraph,
        sub_graph: Option<InternedRenderSubGraph>,
        frame_graph: &mut FrameGraph,
        world: &'w World,
        inputs: &[SlotValue],
        view_entity: Option<Entity>,
    ) -> Result<(), SetupGraphRunnerError> {
        let mut node_outputs: HashMap<InternedRenderLabel, SmallVec<[SlotValue; 4]>> =
            HashMap::default();
        #[cfg(feature = "trace")]
        let span = if let Some(label) = &sub_graph {
            info_span!("run_graph", name = format!("{label:?}"))
        } else {
            info_span!("run_graph", name = "main_graph")
        };
        #[cfg(feature = "trace")]
        let _guard = span.enter();

        // Queue up nodes without inputs, which can be run immediately
        let mut node_queue: VecDeque<&NodeState> = graph
            .iter_nodes()
            .filter(|node| node.input_slots.is_empty())
            .collect();

        // pass inputs into the graph
        if let Some(input_node) = graph.get_input_node() {
            let mut input_values: SmallVec<[SlotValue; 4]> = SmallVec::new();
            for (i, input_slot) in input_node.input_slots.iter().enumerate() {
                if let Some(input_value) = inputs.get(i) {
                    input_values.push(input_value.clone());
                } else {
                    return Err(SetupGraphRunnerError::MissingInput {
                        slot_index: i,
                        slot_name: input_slot.name.clone(),
                        sub_graph,
                    });
                }
            }

            node_outputs.insert(input_node.label, input_values);

            for (_, node_state) in graph
                .iter_node_outputs(input_node.label)
                .expect("node exists")
            {
                node_queue.push_front(node_state);
            }
        }

        'handle_node: while let Some(node_state) = node_queue.pop_back() {
            // skip nodes that are already processed
            if node_outputs.contains_key(&node_state.label) {
                continue;
            }

            let mut slot_indices_and_inputs: SmallVec<[(usize, SlotValue); 4]> = SmallVec::new();
            // check if all dependencies have finished running
            for (edge, input_node) in graph
                .iter_node_inputs(node_state.label)
                .expect("node is in graph")
            {
                match edge {
                    Edge::SlotEdge {
                        output_index,
                        input_index,
                        ..
                    } => {
                        if let Some(outputs) = node_outputs.get(&input_node.label) {
                            slot_indices_and_inputs
                                .push((*input_index, outputs[*output_index].clone()));
                        } else {
                            node_queue.push_front(node_state);
                            continue 'handle_node;
                        }
                    }
                    Edge::NodeEdge { .. } => {
                        if !node_outputs.contains_key(&input_node.label) {
                            node_queue.push_front(node_state);
                            continue 'handle_node;
                        }
                    }
                }
            }

            // construct final sorted input list
            slot_indices_and_inputs.sort_by_key(|(index, _)| *index);
            let inputs: SmallVec<[SlotValue; 4]> = slot_indices_and_inputs
                .into_iter()
                .map(|(_, value)| value)
                .collect();

            if inputs.len() != node_state.input_slots.len() {
                return Err(SetupGraphRunnerError::MismatchedInputCount {
                    node_name: node_state.label,
                    slot_count: node_state.input_slots.len(),
                    value_count: inputs.len(),
                });
            }

            let mut outputs: SmallVec<[Option<SlotValue>; 4]> =
                smallvec![None; node_state.output_slots.len()];
            {
                let mut context = SetupGraphContext::new(graph, node_state, &inputs, &mut outputs);
                if let Some(view_entity) = view_entity {
                    context.set_view_entity(view_entity);
                }

                {
                    #[cfg(feature = "trace")]
                    let _span = info_span!("node", name = node_state.type_name).entered();

                    node_state.node.run(&mut context, frame_graph, world)?;
                }

                for run_sub_graph in context.finish() {
                    let sub_graph = graph
                        .get_sub_graph(run_sub_graph.sub_graph)
                        .expect("sub graph exists because it was validated when queued.");
                    Self::run_graph(
                        sub_graph,
                        Some(run_sub_graph.sub_graph),
                        frame_graph,
                        world,
                        &run_sub_graph.inputs,
                        run_sub_graph.view_entity,
                    )?;
                }
            }

            let mut values: SmallVec<[SlotValue; 4]> = SmallVec::new();
            for (i, output) in outputs.into_iter().enumerate() {
                if let Some(value) = output {
                    values.push(value);
                } else {
                    let empty_slot = node_state.output_slots.get_slot(i).unwrap();
                    return Err(SetupGraphRunnerError::EmptyNodeOutputSlot {
                        type_name: node_state.type_name,
                        slot_index: i,
                        slot_name: empty_slot.name.clone(),
                    });
                }
            }
            node_outputs.insert(node_state.label, values);

            for (_, node_state) in graph
                .iter_node_outputs(node_state.label)
                .expect("node exists")
            {
                node_queue.push_front(node_state);
            }
        }

        Ok(())
    }
}
