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

    fn set_render_resources(
        &mut self,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>> {
        let pipeline_layout = self.pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            if let Some((render_resource_set_id, dynamic_uniform_indices)) =
                render_resource_assignments.get_render_resource_set_id(bind_group.id)
            {
                if let Some(wgpu_bind_group) = self
                    .wgpu_resources
                    .get_bind_group(bind_group.id, *render_resource_set_id)
                {
                    // TODO: check to see if bind group is already set
                    let empty = &[];
                    let dynamic_uniform_indices = if let Some(dynamic_uniform_indices) = dynamic_uniform_indices {
                        dynamic_uniform_indices.as_slice()
                    } else {
                        empty
                    };
                    self.render_pass.set_bind_group(
                        bind_group.index,
                        &wgpu_bind_group,
                        dynamic_uniform_indices,
                    );
                };
            }
        }

        None
    }
}
