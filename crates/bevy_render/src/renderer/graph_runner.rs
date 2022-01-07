use bevy_ecs::world::World;
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{HashMap, HashSet};
use smallvec::{smallvec, SmallVec};
#[cfg(feature = "trace")]
use std::ops::Deref;
use std::{borrow::Cow, collections::VecDeque, rc::Rc};
use thiserror::Error;

use crate::{
    render_graph::{
        Edge, Node, NodeId, NodeRunError, NodeState, RenderGraph, RenderGraphContext,
        RenderGraphId, RenderGraphs, SlotLabel, SlotType, SlotValue, SlotValues,
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
            &SlotValues::default(),
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
        inputs: &SlotValues,
    ) -> Result<(), RenderGraphRunnerError> {
        let mut nodes_ran: HashSet<NodeId> = HashSet::default();
        let graph = graphs.get_graph(graph_id).expect("graph exists");
        let context = RenderGraphContext::new(inputs, graphs);

        #[cfg(feature = "trace")]
        let span = if let Some(name) = &graph_name {
            info_span!("run_graph", name = name.deref())
        } else {
            info_span!("run_graph", name = "main_graph")
        };
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
                if !nodes_ran.contains(&id) {
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
                        &run_sub_graph.inputs,
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

// pub(crate) struct ParalellRenderGraphRunner {

// }

// impl ParalellRenderGraphRunner {
//     pub fn new() -> Self {
//         Self {}
//     }

//     pub fn run(
//         graph: &RenderGraph,
//         render_device: RenderDevice,
//         queue: &wgpu::Queue,
//         world: &World,
//     ) -> Result<(), RenderGraphRunnerError> {

//         Ok(())
//     }

//     pub fn flatten(
//         &self,
//         graph: &RenderGraph,
//         world: &World,
//         inputs: &SlotValues,
//     ) -> Result<Vec<Rc<dyn Node>>, RenderGraphRunnerError> {
//         let mut nodes_ordered = Vec::default();
//         let mut nodes_ran: HashSet<NodeId> = HashSet::default();

//         let context = RenderGraphContext::new(inputs);

//         #[cfg(feature = "trace")]
//         let span = if let Some(name) = &graph_name {
//             info_span!("run_graph", name = name.deref())
//         } else {
//             info_span!("run_graph", name = "main_graph")
//         };
//         #[cfg(feature = "trace")]
//         let _guard = span.enter();

//         // Queue up nodes
//         let mut node_queue: VecDeque<&NodeState> = graph
//             .iter_nodes()
//             .collect();

//         'handle_node: while let Some(node_state) = node_queue.pop_back() {
//             // skip nodes that are already processed
//             if nodes_ran.contains(&node_state.id) {
//                 continue;
//             }

//             // check if all dependencies have finished running
//             for id in graph
//                 .iter_node_dependencies(node_state.id)
//                 .expect("node is in graph")
//             {
//                 if !nodes_ran.contains(&id) {
//                     node_queue.push_front(node_state);
//                     continue 'handle_node;
//                 }
//             }

//             {

//                 #[cfg(feature = "trace")]
//                 let span = info_span!("node", name = node_state.type_name);
//                 #[cfg(feature = "trace")]
//                 let guard = span.enter();

//                 let sub_graph_runs = node_state.node.queue_graphs(&context, world)?;
//                 // node_state.no

//                 #[cfg(feature = "trace")]
//                 drop(guard);

//             }

//             nodes_ran.insert(node_state.id);

//             for (_, node_state) in graph.iter_node_outputs(node_state.id).expect("node exists") {
//                 node_queue.push_front(node_state);
//             }
//         }
//         return Ok(nodes_ordered);
//     }
// }
