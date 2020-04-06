use super::{WgpuRenderer, WgpuResources};
use bevy_render::{
    pipeline::PipelineDescriptor,
    render_resource::{
        RenderResource, RenderResourceAssignments, RenderResourceSetId, ResourceInfo,
    },
    renderer::{RenderPass, Renderer},
};
use std::{collections::HashMap, ops::Range};

pub struct WgpuRenderPass<'a, 'b, 'c, 'd> {
    pub render_pass: &'b mut wgpu::RenderPass<'a>,
    pub pipeline_descriptor: &'c PipelineDescriptor,
    pub wgpu_resources: &'a WgpuResources,
    pub renderer: &'d WgpuRenderer,
    pub bound_bind_groups: HashMap<u32, RenderResourceSetId>,
}

impl<'a, 'b, 'c, 'd> RenderPass for WgpuRenderPass<'a, 'b, 'c, 'd> {
    fn get_renderer(&self) -> &dyn Renderer {
        self.renderer
    }

    fn get_pipeline_descriptor(&self) -> &PipelineDescriptor {
        self.pipeline_descriptor
    }

    fn set_vertex_buffer(&mut self, start_slot: u32, resource: RenderResource, offset: u64) {
        let buffer = self.wgpu_resources.buffers.get(&resource).unwrap();
        self.render_pass
            .set_vertex_buffer(start_slot, &buffer, offset, 0);
    }

    fn set_index_buffer(&mut self, resource: RenderResource, offset: u64) {
        let buffer = self.wgpu_resources.buffers.get(&resource).unwrap();
        self.render_pass.set_index_buffer(&buffer, offset, 0);
    }

    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    fn set_render_resources(
        &mut self,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>> {
        let pipeline_layout = self.pipeline_descriptor.get_layout().unwrap();
        // PERF: vertex buffer lookup comes at a cost when vertex buffers aren't in render_resource_assignments. iterating over render_resource_assignment vertex buffers
        // would likely be faster
        let mut indices = None;
        for (i, vertex_buffer_descriptor) in
            pipeline_layout.vertex_buffer_descriptors.iter().enumerate()
        {
            if let Some((vertex_buffer, index_buffer)) =
                render_resource_assignments.get_vertex_buffer(&vertex_buffer_descriptor.name)
            {
                log::trace!(
                    "set vertex buffer {}: {} ({:?})",
                    i,
                    vertex_buffer_descriptor.name,
                    vertex_buffer
                );
                self.set_vertex_buffer(i as u32, vertex_buffer, 0);
                if let Some(index_buffer) = index_buffer {
                    log::trace!(
                        "set index buffer: {} ({:?})",
                        vertex_buffer_descriptor.name,
                        index_buffer
                    );
                    self.set_index_buffer(index_buffer, 0);
                    match self.renderer.get_resource_info(index_buffer).unwrap() {
                        ResourceInfo::Buffer(buffer_info) => {
                            indices = Some(0..(buffer_info.size / 2) as u32)
                        }
                        _ => panic!("expected a buffer type"),
                    }
                }
            }
        }

        for bind_group in pipeline_layout.bind_groups.iter() {
            if let Some((render_resource_set_id, dynamic_uniform_indices)) =
                render_resource_assignments.get_render_resource_set_id(bind_group.id)
            {
                if let Some(wgpu_bind_group) = self
                    .wgpu_resources
                    .get_bind_group(bind_group.id, *render_resource_set_id)
                {
                    const EMPTY: &'static [u32] = &[];
                    let dynamic_uniform_indices =
                        if let Some(dynamic_uniform_indices) = dynamic_uniform_indices {
                            dynamic_uniform_indices.as_slice()
                        } else {
                            EMPTY
                        };

                    // don't bind bind groups if they are already set
                    // TODO: these checks come at a performance cost. make sure its worth it!
                    if let Some(bound_render_resource_set) =
                        self.bound_bind_groups.get(&bind_group.index)
                    {
                        if *bound_render_resource_set == *render_resource_set_id
                            && dynamic_uniform_indices.len() == 0
                        {
                            continue;
                        }
                    }

                    if dynamic_uniform_indices.len() == 0 {
                        self.bound_bind_groups
                            .insert(bind_group.index, *render_resource_set_id);
                    } else {
                        self.bound_bind_groups.remove(&bind_group.index);
                    }

                    log::trace!(
                        "set bind group {} {:?}: {:?}",
                        bind_group.index,
                        dynamic_uniform_indices,
                        render_resource_set_id
                    );
                    self.render_pass.set_bind_group(
                        bind_group.index,
                        &wgpu_bind_group,
                        dynamic_uniform_indices,
                    );
                };
            }
        }

        indices
    }
}
