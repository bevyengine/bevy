use super::{WgpuRenderer, WgpuResources};
use crate::render::{
    pipeline::{BindType, PipelineDescriptor},
    render_resource::{BufferInfo, RenderResource, RenderResourceAssignments, ResourceInfo},
    renderer::{RenderPass, Renderer},
};
use std::ops::Range;

pub struct WgpuRenderPass<'a, 'b, 'c, 'd> {
    pub render_pass: &'b mut wgpu::RenderPass<'a>,
    pub pipeline_descriptor: &'c PipelineDescriptor,
    pub wgpu_resources: &'a WgpuResources,
    pub renderer: &'d WgpuRenderer,
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
            .set_vertex_buffers(start_slot, &[(&buffer, offset)]);
    }

    fn set_index_buffer(&mut self, resource: RenderResource, offset: u64) {
        let buffer = self.wgpu_resources.buffers.get(&resource).unwrap();
        self.render_pass.set_index_buffer(&buffer, offset);
    }

    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    fn set_render_resource_assignments(
        &mut self,
        render_resource_assignments: Option<&RenderResourceAssignments>,
    ) -> Option<Range<u32>> {
        let pipeline_layout = self.pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            let bind_group_id = bind_group.get_id().unwrap();
            let wgpu_bind_group = match self.wgpu_resources.bind_groups.get(&bind_group_id) {
                // if there is a "global" bind group, use that
                Some(wgpu_bind_group) => wgpu_bind_group,
                // otherwise try to get an entity-specific bind group
                None => {
                    if let Some(assignments) = render_resource_assignments {
                        self.wgpu_resources
                            .get_assignments_bind_group(assignments.get_id(), bind_group_id)
                            .unwrap()
                    } else {
                        panic!("No bind group exists that matches: {:?}");
                    }
                }
            };

            // setup dynamic uniform instances
            // TODO: these indices could be stored in RenderResourceAssignments so they dont need to be collected on each draw
            let mut dynamic_uniform_indices = Vec::new();
            for binding in bind_group.bindings.iter() {
                if let BindType::Uniform { dynamic, .. } = binding.bind_type {
                    if !dynamic {
                        continue;
                    }

                    // PERF: This hashmap get is pretty expensive (10 fps for 10000 entities)
                    if let Some(resource) = self
                        .wgpu_resources
                        .render_resources
                        .get_named_resource(&binding.name)
                    {
                        if let Some(ResourceInfo::Buffer(BufferInfo {
                            array_info: Some(array_info),
                            is_dynamic: true,
                            ..
                        })) = self.wgpu_resources.resource_info.get(&resource)
                        {
                            let index = array_info
                                .indices
                                .get(&render_resource_assignments.unwrap().get_id())
                                .unwrap();

                            dynamic_uniform_indices.push((*index * array_info.item_size) as u32);
                        }
                    }
                }
            }

            // TODO: check to see if bind group is already set
            self.render_pass.set_bind_group(
                bind_group.index,
                &wgpu_bind_group,
                dynamic_uniform_indices.as_slice(),
            );
        }

        None
    }
}
