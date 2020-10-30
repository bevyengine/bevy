use super::{WgpuRenderContext, WgpuRenderResourceContext};
use bevy_ecs::{Resources, World};
use bevy_render::{
    render_graph::{Edge, NodeId, ResourceSlots, StageBorrow},
    renderer::RenderResourceContext,
};
use bevy_utils::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug)]
pub struct WgpuRenderGraphExecutor {
    pub max_thread_count: usize,
}

impl WgpuRenderGraphExecutor {
    pub fn execute(
        &self,
        world: &World,
        resources: &Resources,
        device: Arc<wgpu::Device>,
        queue: &mut wgpu::Queue,
        stages: &mut [StageBorrow],
    ) {
        let mut render_resource_context = resources
            .get_mut::<Box<dyn RenderResourceContext>>()
            .unwrap();
        let render_resource_context = render_resource_context
            .downcast_mut::<WgpuRenderResourceContext>()
            .unwrap();
        let node_outputs: Arc<RwLock<HashMap<NodeId, ResourceSlots>>> = Default::default();
        for stage in stages.iter_mut() {
            // TODO: sort jobs and slice by "amount of work" / weights
            // stage.jobs.sort_by_key(|j| j.node_states.len());

            let (sender, receiver) = crossbeam_channel::bounded(self.max_thread_count);
            let chunk_size = (stage.jobs.len() + self.max_thread_count - 1) / self.max_thread_count; // divide ints rounding remainder up
            let mut actual_thread_count = 0;
            // crossbeam_utils::thread::scope(|s| {
            for jobs_chunk in stage.jobs.chunks_mut(chunk_size) {
                let sender = sender.clone();
                let world = &*world;
                actual_thread_count += 1;
                let device = device.clone();
                let render_resource_context = render_resource_context.clone();
                let node_outputs = node_outputs.clone();
                // s.spawn(move |_| {
                let mut render_context = WgpuRenderContext::new(device, render_resource_context);
                for job in jobs_chunk.iter_mut() {
                    for node_state in job.node_states.iter_mut() {
                        // bind inputs from connected node outputs
                        for (i, mut input_slot) in node_state.input_slots.iter_mut().enumerate() {
                            if let Edge::SlotEdge {
                                output_node,
                                output_index,
                                ..
                            } = node_state.edges.get_input_slot_edge(i).unwrap()
                            {
                                let node_outputs = node_outputs.read();
                                let outputs = if let Some(outputs) = node_outputs.get(output_node) {
                                    outputs
                                } else {
                                    panic!("node inputs not set")
                                };

                                let output_resource =
                                    outputs.get(*output_index).expect("output should be set");
                                input_slot.resource = Some(output_resource);
                            } else {
                                panic!("no edge connected to input")
                            }
                        }
                        node_state.node.update(
                            world,
                            resources,
                            &mut render_context,
                            &node_state.input_slots,
                            &mut node_state.output_slots,
                        );

                        node_outputs
                            .write()
                            .insert(node_state.id, node_state.output_slots.clone());
                    }
                }
                sender.send(render_context.finish()).unwrap();
                // });
            }
            // })
            // .unwrap();

            let mut command_buffers = Vec::new();
            for _i in 0..actual_thread_count {
                let command_buffer = receiver.recv().unwrap();
                if let Some(command_buffer) = command_buffer {
                    command_buffers.push(command_buffer);
                }
            }

            queue.submit(command_buffers.drain(..));
        }
    }
}
