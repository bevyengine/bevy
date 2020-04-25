use super::{WgpuRenderContext, WgpuRenderResourceContext};
use bevy_render::{
    render_graph::{Edge, NodeId, ResourceSlots, StageBorrow},
    renderer::GlobalRenderResourceContext,
};
use legion::prelude::{Resources, World};
use std::{collections::HashMap, sync::Arc};

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
        let mut global_context = resources.get_mut::<GlobalRenderResourceContext>().unwrap();
        let render_resource_context = global_context
            .context
            .downcast_mut::<WgpuRenderResourceContext>()
            .unwrap();
        let mut node_outputs: HashMap<NodeId, ResourceSlots> = HashMap::new();
        for stage in stages.iter_mut() {
            // TODO: sort jobs and slice by "amount of work" / weights
            // stage.jobs.sort_by_key(|j| j.node_states.len());

            let (sender, receiver) = crossbeam_channel::bounded(self.max_thread_count);
            let chunk_size = (stage.jobs.len() + self.max_thread_count - 1) / self.max_thread_count; // divide ints rounding remainder up
            let mut actual_thread_count = 0;
            crossbeam_utils::thread::scope(|s| {
                let node_outputs = &node_outputs;
                for jobs_chunk in stage.jobs.chunks_mut(chunk_size) {
                    let sender = sender.clone();
                    let world = &*world;
                    actual_thread_count += 1;
                    let device = device.clone();
                    let render_resource_context = render_resource_context.clone();
                    s.spawn(move |_| {
                        let mut render_context =
                            WgpuRenderContext::new(device, render_resource_context);
                        let mut local_node_outputs = HashMap::new();
                        for job in jobs_chunk.iter_mut() {
                            for node_state in job.node_states.iter_mut() {
                                // bind inputs from connected node outputs
                                for (i, mut input_slot) in node_state.input_slots.iter_mut().enumerate()
                                {
                                    if let Edge::SlotEdge {
                                        output_node,
                                        output_index,
                                        ..
                                    } = node_state.edges.get_input_slot_edge(i).unwrap()
                                    {
                                        let outputs =
                                            if let Some(outputs) = node_outputs.get(output_node) {
                                                outputs
                                            } else if let Some(outputs) =
                                                local_node_outputs.get(output_node)
                                            {
                                                outputs
                                            } else {
                                                panic!("node inputs not set")
                                            };

                                        let output_resource = outputs
                                            .get(*output_index)
                                            .expect("output should be set");
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

                                local_node_outputs
                                    .insert(node_state.id, node_state.output_slots.clone());
                            }
                        }
                        sender
                            .send((render_context.finish(), local_node_outputs))
                            .unwrap();
                    });
                }
            })
            .unwrap();

            let mut command_buffers = Vec::new();
            for _i in 0..actual_thread_count {
                let (command_buffer, mut local_node_outputs) = receiver.recv().unwrap();
                if let Some(command_buffer) = command_buffer {
                    command_buffers.push(command_buffer);
                }

                node_outputs.extend(local_node_outputs.drain());
            }

            queue.submit(&command_buffers);
        }
    }
}
