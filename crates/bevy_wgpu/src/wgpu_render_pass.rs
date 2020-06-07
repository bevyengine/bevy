use crate::{renderer::WgpuRenderContext, WgpuResourceRefs};
use bevy_asset::Handle;
use bevy_render::{
    pass::RenderPass,
    pipeline::PipelineDescriptor,
    render_resource::{
        RenderResourceId, RenderResourceAssignments, RenderResourceSetId, ResourceInfo,
    },
    renderer::RenderContext,
};
use std::{collections::HashMap, ops::Range};

pub struct WgpuRenderPass<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub render_context: &'a WgpuRenderContext,
    pub render_resources: WgpuResourceRefs<'a>,
    pub bound_bind_groups: HashMap<u32, RenderResourceSetId>,
}

impl<'a> RenderPass for WgpuRenderPass<'a> {
    fn get_render_context(&self) -> &dyn RenderContext {
        self.render_context
    }

    fn set_vertex_buffer(&mut self, start_slot: u32, resource: RenderResourceId, offset: u64) {
        let buffer = self.render_resources.buffers.get(&resource).unwrap();
        self.render_pass
            .set_vertex_buffer(start_slot, buffer.slice(offset..));
    }

    fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
        self.render_pass
            .set_viewport(x, y, w, h, min_depth, max_depth);
    }

    fn set_stencil_reference(&mut self, reference: u32) {
        self.render_pass.set_stencil_reference(reference);
    }

    fn set_index_buffer(&mut self, resource: RenderResourceId, offset: u64) {
        let buffer = self.render_resources.buffers.get(&resource).unwrap();
        self.render_pass.set_index_buffer(buffer.slice(offset..));
    }

    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances);
    }

    fn set_render_resources(
        &mut self,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>> {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
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
                    self.render_context.resources().get_resource_info(
                        index_buffer,
                        &mut |resource_info| match resource_info {
                            Some(ResourceInfo::Buffer(buffer_info)) => {
                                indices = Some(0..(buffer_info.size / 2) as u32)
                            }
                            _ => panic!("expected a buffer type"),
                        },
                    );
                }
            }
        }

        for bind_group in pipeline_layout.bind_groups.iter() {
            if let Some(resource_set) =
                render_resource_assignments.get_render_resource_set(bind_group.id)
            {
                if let Some(bind_group_info) = self.render_resources.bind_groups.get(&bind_group.id)
                {
                    if let Some(wgpu_bind_group) = bind_group_info.bind_groups.get(&resource_set.id)
                    {
                        const EMPTY: &'static [u32] = &[];
                        let dynamic_uniform_indices = if let Some(ref dynamic_uniform_indices) =
                            resource_set.dynamic_uniform_indices
                        {
                            dynamic_uniform_indices.as_slice()
                        } else {
                            EMPTY
                        };

                        // don't bind bind groups if they are already set
                        // TODO: these checks come at a performance cost. make sure it's worth it!
                        if let Some(bound_render_resource_set) =
                            self.bound_bind_groups.get(&bind_group.index)
                        {
                            if *bound_render_resource_set == resource_set.id
                                && dynamic_uniform_indices.len() == 0
                            {
                                continue;
                            }
                        }

                        if dynamic_uniform_indices.len() == 0 {
                            self.bound_bind_groups
                                .insert(bind_group.index, resource_set.id);
                        } else {
                            self.bound_bind_groups.remove(&bind_group.index);
                        }

                        log::trace!(
                            "set bind group {} {:?}: {:?}",
                            bind_group.index,
                            dynamic_uniform_indices,
                            resource_set.id
                        );
                        self.render_pass.set_bind_group(
                            bind_group.index,
                            wgpu_bind_group,
                            dynamic_uniform_indices,
                        );
                    }
                };
            }
        }

        indices
    }
    fn set_pipeline(&mut self, pipeline_handle: Handle<PipelineDescriptor>) {
        let pipeline = self.render_resources.render_pipelines.get(&pipeline_handle).expect(
            "Attempted to use a pipeline that does not exist in this RenderPass's RenderContext",
        );
        self.render_pass.set_pipeline(pipeline);
    }
}
