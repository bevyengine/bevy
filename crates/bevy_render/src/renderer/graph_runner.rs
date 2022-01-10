use bevy_ecs::world::World;
use bevy_tasks::ComputeTaskPool;
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::HashSet;
use std::ops::Deref;
use std::{borrow::Cow, collections::VecDeque};
use thiserror::Error;

use crate::{
    render_graph::{
        NodeId, NodeRunError, NodeState, RenderGraphContext, RenderGraphId, RenderGraphs, SlotType,
        SlotValues,
    },
    renderer::{RenderContext, RenderDevice},
};

pub struct RenderGraphRunner;

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
        label: &'static str,
        expected: SlotType,
        actual: SlotType,
    },
}

impl RenderGraphRunner {
    pub fn run(
        main_id: &RenderGraphId,
        render_device: RenderDevice,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<(), RenderGraphRunnerError> {
        let command_encoder =
            render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut render_context = RenderContext {
            render_device,
            command_encoder,
        };
        let graphs = world.get_resource().expect("render graphs should exist");
        Self::run_graph(
            graphs,
            main_id,
            &mut render_context,
            world,
            SlotValues::default(),
        )?;
        {
            #[cfg(feature = "trace")]
            let span = info_span!("submit_graph_commands");
            #[cfg(feature = "trace")]
            let _guard = span.enter();
            queue.submit(vec![render_context.command_encoder.finish()]);
        }

        Ok(())
    }

    fn run_graph(
        graphs: &RenderGraphs,
        graph_id: &RenderGraphId,
        render_context: &mut RenderContext,
        world: &World,
        inputs: SlotValues,
    ) -> Result<(), RenderGraphRunnerError> {
        let mut nodes_ran: HashSet<NodeId> = HashSet::default();
        let graph = graphs.get_graph(graph_id).expect("graph exists");
        let context = RenderGraphContext::new(inputs, graphs);

        #[cfg(feature = "trace")]
        let span = info_span!("run_graph", name = "main_graph");
        #[cfg(feature = "trace")]
        let _guard = span.enter();

        // Queue up nodes
        let mut node_queue: VecDeque<&NodeState> = graph.iter_nodes().collect();

        'handle_node: while let Some(node_state) = node_queue.pop_back() {
            // skip nodes that are already processed
            if nodes_ran.contains(&node_state.id) {
                continue;
            }

            // check if all dependencies have finished running
            for id in graph
                .iter_node_dependencies(node_state.id)
                .expect("node is in graph")
            {
                if !nodes_ran.contains(id) {
                    node_queue.push_front(node_state);
                    continue 'handle_node;
                }
            }

            {
                #[cfg(feature = "trace")]
                let span = info_span!("node", name = node_state.type_name);
                #[cfg(feature = "trace")]
                let guard = span.enter();

                let sub_graph_runs = node_state.node.queue_graphs(&context, world)?;

                node_state.node.record(&context, render_context, world)?;

                #[cfg(feature = "trace")]
                drop(guard);

                for run_sub_graph in sub_graph_runs.drain() {
                    let sub_graph = graphs
                        .get_graph(run_sub_graph.id)
                        .expect("sub graph exists because it was validated when queued, the slot inputs are also valid");

                    Self::run_graph(
                        graphs,
                        sub_graph.id(),
                        render_context,
                        world,
                        run_sub_graph.inputs,
                    )?;
                }
            }

            nodes_ran.insert(node_state.id);

            for (_, node_state) in graph.iter_node_outputs(node_state.id).expect("node exists") {
                node_queue.push_front(node_state);
            }
        }

        Ok(())
    }
}

pub(crate) struct ParalellRenderGraphRunner {
    threads: usize,
}

impl ParalellRenderGraphRunner {
    pub fn new() -> Self {
        Self { threads: 4 }
    }

    pub fn run(
        &self,
        main_graph_id: &RenderGraphId,
        render_device: RenderDevice,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<(), RenderGraphRunnerError> {
        let graphs = world.get_resource::<RenderGraphs>().unwrap();

        let slot_values = SlotValues::empty();
        let context = RenderGraphContext::new(slot_values, graphs);

        let recording_nodes = self.flatten(main_graph_id, graphs, world, context)?;
        let compute_pool = world.get_resource::<ComputeTaskPool>().unwrap();

        let len = recording_nodes.len();

        // TODO: split into workgroups based on estimated time to complete nodes instead of just the number of nodes
        let workgroup_size = (len / self.threads) + (len % self.threads != 0) as usize; // Ceiled division
        let node_ranges = (0..self.threads).map(|s| {
            (s * workgroup_size)..(*vec![(s + 1) * workgroup_size, len].iter().min().unwrap())
        }); // Which ranges of nodes to be recorded on the same thread

        let finished_buffers = compute_pool.scope(|scope| {
            for range in node_ranges {
                let command_encoder = render_device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                let mut render_context = RenderContext {
                    render_device: render_device.clone(),
                    command_encoder,
                };
                scope.spawn(async {
                    for node in range {
                        let (state, ctx) = &recording_nodes[node];
                        state.node.record(ctx, &mut render_context, world).unwrap();
                    }
                    render_context.command_encoder.finish()
                });
            }
        });

        // for (node, ctx) in recording_nodes {
        //     node.node.record(&ctx, &mut render_context, world)?;
        // }

        {
            #[cfg(feature = "trace")]
            let span = info_span!("submit_graph_commands");
            #[cfg(feature = "trace")]
            let _guard = span.enter();

            queue.submit(finished_buffers);
        }

        Ok(())
    }
    /// Runs the `queue_graphs` methods of all of the nodes in the graph and returns a
    /// topologically sorted vector of the resulting nodes
    pub fn flatten<'g>(
        &'g self,
        graph_id: &RenderGraphId,
        graphs: &'g RenderGraphs,
        world: &World,
        context: RenderGraphContext<'g>,
    ) -> Result<Vec<(&'g NodeState, RenderGraphContext)>, RenderGraphRunnerError> {
        let mut nodes_ordered = Vec::default();
        let mut nodes_ran: HashSet<NodeId> = HashSet::default();

        let graph = graphs.get_graph(graph_id).unwrap();

        #[cfg(feature = "trace")]
        let span = info_span!("run_graph", name = graph.get_name().deref());
        #[cfg(feature = "trace")]
        let _guard = span.enter();

        // Queue up nodes
        let mut node_queue: VecDeque<&NodeState> = graph.iter_nodes().collect();

        'handle_node: while let Some(node_state) = node_queue.pop_back() {
            // skip nodes that are already processed
            if nodes_ran.contains(&node_state.id) {
                continue;
            }

            // check if all dependencies have finished running
            for id in graph
                .iter_node_dependencies(node_state.id)
                .expect("node is in graph")
            {
                if !nodes_ran.contains(id) {
                    node_queue.push_front(node_state);
                    continue 'handle_node;
                }
            }

            {
                #[cfg(feature = "trace")]
                let span = info_span!("node", name = node_state.type_name);
                #[cfg(feature = "trace")]
                let guard = span.enter();

                let sub_graph_runs = node_state.node.queue_graphs(&context, world)?;

                for graph in sub_graph_runs.drain() {
                    let sub_context = RenderGraphContext::new(graph.inputs, graphs);
                    nodes_ordered.extend(
                        self.flatten(&graph.id, graphs, world, sub_context)?
                            .into_iter(),
                    )
                }
                nodes_ordered.push((node_state, context.clone()));

                #[cfg(feature = "trace")]
                drop(guard);
            }

            nodes_ran.insert(node_state.id);

            for (_, node_state) in graph.iter_node_outputs(node_state.id).expect("node exists") {
                node_queue.push_front(node_state);
            }
        }

        Ok(nodes_ordered)
    }
}
