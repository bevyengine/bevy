use super::{WgpuRenderContext, WgpuRenderResourceContext};
use bevy_render::{render_graph_2::StageBorrow, renderer_2::GlobalRenderResourceContext};
use legion::prelude::{Resources, World};
use std::sync::Arc;

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
        for stage in stages.iter_mut() {
            // TODO: sort jobs and slice by "amount of work" / weights
            // stage.jobs.sort_by_key(|j| j.node_states.len());

            let (sender, receiver) = crossbeam_channel::bounded(self.max_thread_count);
            let chunk_size = (stage.jobs.len() + self.max_thread_count - 1) / self.max_thread_count; // divide ints rounding remainder up
            let mut actual_thread_count = 0;
            crossbeam_utils::thread::scope(|s| {
                for jobs_chunk in stage.jobs.chunks_mut(chunk_size) {
                    let sender = sender.clone();
                    let world = &*world;
                    actual_thread_count += 1;
                    let device = device.clone();
                    let render_resource_context = render_resource_context.clone();
                    s.spawn(move |_| {
                        let mut render_context =
                            WgpuRenderContext::new(device, render_resource_context);
                        for job in jobs_chunk.iter_mut() {
                            for node_state in job.node_states.iter_mut() {
                                node_state.node.update(
                                    world,
                                    resources,
                                    &mut render_context,
                                    &node_state.input_slots,
                                    &mut node_state.output_slots,
                                );
                            }
                        }
                        sender.send(render_context.finish()).unwrap();
                    });
                }
            })
            .unwrap();

            let mut command_buffers = Vec::new();
            for _i in 0..actual_thread_count {
                let command_buffer = receiver.recv().unwrap();
                if let Some(command_buffer) = command_buffer {
                    command_buffers.push(command_buffer);
                }
            }

            queue.submit(&command_buffers);
        }
    }
}
