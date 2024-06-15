use bevy_ecs::{prelude::Entity, world::World};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::HashMap;

use smallvec::{smallvec, SmallVec};
use std::{borrow::Cow, collections::VecDeque};
use thiserror::Error;

use crate::{
    diagnostic::internal::{DiagnosticsRecorder, RenderDiagnosticsMutex},
    render_graph::{
        Edge, InternedRenderLabel, InternedRenderSubGraph, NodeRunError, NodeState, RenderGraph,
        RenderGraphContext, SlotLabel, SlotType, SlotValue,
    },
    renderer::{RenderContext, RenderDevice},
};

/// The [`RenderGraphRunner`] is responsible for executing a [`RenderGraph`].
///
/// It will run all nodes in the graph sequentially in the correct order (defined by the edges).
/// Each [`Node`](crate::render_graph::node::Node) can run any arbitrary code, but will generally
/// either send directly a [`CommandBuffer`] or a task that will asynchronously generate a [`CommandBuffer`]
///
/// After running the graph, the [`RenderGraphRunner`] will execute in parallel all the tasks to get
/// an ordered list of [`CommandBuffer`]s to execute. These [`CommandBuffer`] will be submitted to the GPU
/// sequentially in the order that the tasks were submitted. (which is the order of the [`RenderGraph`])
///
/// [`CommandBuffer`]: wgpu::CommandBuffer
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
    #[error("graph '{sub_graph:?}' could not be run because slot '{slot_name}' at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        sub_graph: Option<InternedRenderSubGraph>,
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
        node_name: InternedRenderLabel,
        slot_count: usize,
        value_count: usize,
    },
}

impl RenderGraphRunner {
    pub fn run(
        graph: &RenderGraph,
        render_device: RenderDevice,
        mut diagnostics_recorder: Option<DiagnosticsRecorder>,
        queue: &wgpu::Queue,
        adapter: &wgpu::Adapter,
        world: &World,
        finalizer: impl FnOnce(&mut wgpu::CommandEncoder),
    ) -> Result<Option<DiagnosticsRecorder>, RenderGraphRunnerError> {
        if let Some(recorder) = &mut diagnostics_recorder {
            recorder.begin_frame();
        }

        let mut render_context =
            RenderContext::new(render_device, adapter.get_info(), diagnostics_recorder);
        Self::run_graph(graph, None, &mut render_context, world, &[], None)?;
        finalizer(render_context.command_encoder());

        let (render_device, mut diagnostics_recorder) = {
            #[cfg(feature = "trace")]
            let _span = info_span!("submit_graph_commands").entered();

            let (commands, render_device, diagnostics_recorder) = render_context.finish();
            queue.submit(commands);

            (render_device, diagnostics_recorder)
        };

        if let Some(recorder) = &mut diagnostics_recorder {
            let render_diagnostics_mutex = world.resource::<RenderDiagnosticsMutex>().0.clone();
            recorder.finish_frame(&render_device, move |diagnostics| {
                *render_diagnostics_mutex.lock().expect("lock poisoned") = Some(diagnostics);
            });
        }

        Ok(diagnostics_recorder)
    }

    /// Runs the [`RenderGraph`] and all its sub-graphs sequentially, making sure that all nodes are
    /// run in the correct order. (a node only runs when all its dependencies have finished running)
    fn run_graph<'w>(
        graph: &RenderGraph,
        sub_graph: Option<InternedRenderSubGraph>,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
        inputs: &[SlotValue],
        view_entity: Option<Entity>,
    ) -> Result<(), RenderGraphRunnerError> {
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
                return Err(RenderGraphRunnerError::MismatchedInputCount {
                    node_name: node_state.label,
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
                        .get_sub_graph(run_sub_graph.sub_graph)
                        .expect("sub graph exists because it was validated when queued.");
                    Self::run_graph(
                        sub_graph,
                        Some(run_sub_graph.sub_graph),
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
