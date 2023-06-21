use bevy_ecs::{prelude::Entity, world::World};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::HashMap;
use smallvec::{smallvec, SmallVec};
#[cfg(feature = "trace")]
use std::ops::Deref;
use std::{borrow::Cow, collections::VecDeque};
use thiserror::Error;

use crate::{
    render_graph::{
        Edge, NodeId, NodeRunError, NodeState, RenderGraph, RenderGraphContext, SlotLabel,
        SlotType, SlotValue,
    },
    renderer::{RenderContext, RenderDevice},
};

pub(crate) struct RenderGraphRunner;

#[derive(Error, Debug)]
pub enum RenderGraphRunnerError {
    #[error(transparent)]
    NodeRunError(#[from] NodeRunError),
    #[error("node output slot not set (index {slot_index}, name {slot_name})")]
    EmptyNodeOutputSlot {
        type_name: &'static str,
        slot_index: usize,
        slot_name: Cow<'static, str>,
    },
    #[error("graph (name: '{graph_name:?}') could not be run because slot '{slot_name}' at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        graph_name: Option<Cow<'static, str>>,
    },
    #[error("attempted to use the wrong type for input slot")]
    MismatchedInputSlotType {
        slot_index: usize,
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
    #[error(
        "node (name: '{node_name:?}') has {slot_count} input slots, but was provided {value_count} values"
    )]
    MismatchedInputCount {
        node_name: Option<Cow<'static, str>>,
        slot_count: usize,
        value_count: usize,
    },
}

impl RenderGraphRunner {
    pub fn run(
        graph: &RenderGraph,
        render_device: RenderDevice,
        queue: &wgpu::Queue,
        world: &World,
        finalizer: impl FnOnce(&mut wgpu::CommandEncoder),
    ) -> Result<(), RenderGraphRunnerError> {
        let mut render_context = RenderContext::new(render_device);
        Self::run_graph(graph, None, &mut render_context, world, &[], None)?;
        finalizer(render_context.command_encoder());

        {
            #[cfg(feature = "trace")]
            let _span = info_span!("submit_graph_commands").entered();
            queue.submit(render_context.finish());
        }
        Ok(())
    }

    fn run_graph(
        graph: &RenderGraph,
        graph_name: Option<Cow<'static, str>>,
        render_context: &mut RenderContext,
        world: &World,
        inputs: &[SlotValue],
        view_entity: Option<Entity>,
    ) -> Result<(), RenderGraphRunnerError> {
        let mut node_outputs: HashMap<NodeId, SmallVec<[SlotValue; 4]>> = HashMap::default();
        #[cfg(feature = "trace")]
        let span = if let Some(name) = &graph_name {
            info_span!("run_graph", name = name.deref())
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
                    if input_slot.slot_type != input_value.slot_type() {
                        return Err(RenderGraphRunnerError::MismatchedInputSlotType {
                            slot_index: i,
                            actual: input_value.slot_type(),
                            expected: input_slot.slot_type,
                            label: input_slot.name.clone().into(),
                        });
                    }
                    input_values.push(input_value.clone());
                } else {
                    return Err(RenderGraphRunnerError::MissingInput {
                        slot_index: i,
                        slot_name: input_slot.name.clone(),
                        graph_name,
                    });
                }
            }

            node_outputs.insert(input_node.id, input_values);

            for (_, node_state) in graph.iter_node_outputs(input_node.id).expect("node exists") {
                node_queue.push_front(node_state);
            }
        }

        'handle_node: while let Some(node_state) = node_queue.pop_back() {
            // skip nodes that are already processed
            if node_outputs.contains_key(&node_state.id) {
                continue;
            }

            let mut slot_indices_and_inputs: SmallVec<[(usize, SlotValue); 4]> = SmallVec::new();
            // check if all dependencies have finished running
            for (edge, input_node) in graph
                .iter_node_inputs(node_state.id)
                .expect("node is in graph")
            {
                match edge {
                    Edge::SlotEdge {
                        output_index,
                        input_index,
                        ..
                    } => {
                        if let Some(outputs) = node_outputs.get(&input_node.id) {
                            slot_indices_and_inputs
                                .push((*input_index, outputs[*output_index].clone()));
                        } else {
                            node_queue.push_front(node_state);
                            continue 'handle_node;
                        }
                    }
                    Edge::NodeEdge { .. } => {
                        if !node_outputs.contains_key(&input_node.id) {
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
                return Err(RenderGraphRunnerError::MismatchedInputCount {
                    node_name: node_state.name.clone(),
                    slot_count: node_state.input_slots.len(),
                    value_count: inputs.len(),
                });
            }

            let mut outputs: SmallVec<[Option<SlotValue>; 4]> =
                smallvec![None; node_state.output_slots.len()];
            {
                let mut context = RenderGraphContext::new(graph, node_state, &inputs, &mut outputs);
                if let Some(view_entity) = view_entity {
                    context.set_view_entity(view_entity);
                }

                {
                    #[cfg(feature = "trace")]
                    let _span = info_span!("node", name = node_state.type_name).entered();

                    node_state.node.run(&mut context, render_context, world)?;
                }

                for run_sub_graph in context.finish() {
                    let sub_graph = graph
                        .get_sub_graph(&run_sub_graph.name)
                        .expect("sub graph exists because it was validated when queued.");
                    Self::run_graph(
                        sub_graph,
                        Some(run_sub_graph.name),
                        render_context,
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
                    return Err(RenderGraphRunnerError::EmptyNodeOutputSlot {
                        type_name: node_state.type_name,
                        slot_index: i,
                        slot_name: empty_slot.name.clone(),
                    });
                }
            }
            node_outputs.insert(node_state.id, values);

            for (_, node_state) in graph.iter_node_outputs(node_state.id).expect("node exists") {
                node_queue.push_front(node_state);
            }
        }

        Ok(())
    }
}
