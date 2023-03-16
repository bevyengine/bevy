use bevy_ecs::{prelude::Entity, world::World};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::HashSet;
#[cfg(feature = "trace")]
use std::ops::Deref;
use std::{borrow::Cow, collections::VecDeque};
use thiserror::Error;

use crate::{
    render_graph::{NodeId, NodeRunError, NodeState, RenderGraph, RenderGraphContext},
    renderer::{RenderContext, RenderDevice},
};

pub(crate) struct RenderGraphRunner;

#[derive(Error, Debug)]
pub enum RenderGraphRunnerError {
    #[error(transparent)]
    NodeRunError(#[from] NodeRunError),
}

impl RenderGraphRunner {
    pub fn run(
        graph: &RenderGraph,
        render_device: RenderDevice,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<(), RenderGraphRunnerError> {
        let mut render_context = RenderContext::new(render_device);
        Self::run_graph(graph, None, &mut render_context, world, None)?;
        {
            #[cfg(feature = "trace")]
            let _span = info_span!("submit_graph_commands").entered();
            queue.submit(render_context.finish());
        }
        Ok(())
    }

    fn run_graph(
        graph: &RenderGraph,
        #[allow(unused)] // This is only used in when trace is enabled
        graph_name: Option<Cow<'static, str>>,
        render_context: &mut RenderContext,
        world: &World,
        view_entity: Option<Entity>,
    ) -> Result<(), RenderGraphRunnerError> {
        let mut node_completed: HashSet<NodeId> = HashSet::default();
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
            if node_completed.contains(&node_state.id) {
                continue;
            }

            // check if all dependencies have finished running
            for (_edge, input_node) in graph
                .iter_node_inputs(node_state.id)
                .expect("node is in graph")
            {
                if !node_completed.contains(&input_node.id) {
                    node_queue.push_front(node_state);
                    continue 'handle_node;
                }
            }

            {
                let mut context = RenderGraphContext::new();
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
                        Some(run_sub_graph.view_entity),
                    )?;
                }
            }

            node_completed.insert(node_state.id);

            for (_, node_state) in graph.iter_node_outputs(node_state.id).expect("node exists") {
                node_queue.push_front(node_state);
            }
        }

        Ok(())
    }
}
