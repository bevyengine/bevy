use super::{WgpuRenderer, WgpuResources};
use crate::render::{
    pipeline::{BindType, PipelineDescriptor},
    render_resource::RenderResource,
    renderer::{RenderPass, Renderer},
};
use legion::prelude::Entity;

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

    fn draw_indexed(
        &mut self,
        indices: core::ops::Range<u32>,
        base_vertex: i32,
        instances: core::ops::Range<u32>,
    ) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    fn set_bind_groups(&mut self, entity: Option<&Entity>) {
        let pipeline_layout = self.pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            let bind_group_id = bind_group.get_hash().unwrap();
            let bind_group_info = match self.wgpu_resources.bind_groups.get(&bind_group_id) {
                // if there is a "global" bind group, use that
                Some(bind_group_info) => bind_group_info,
                // otherwise try to get an entity-specific bind group
                None => {
                    if let Some(entity) = entity {
                        self.wgpu_resources
                            .get_entity_bind_group(*entity, bind_group_id)
                            .unwrap()
                    } else {
                        panic!("No bind group exists that matches: {:?}");
                    }
                }
            };

            // setup dynamic uniform instances
            let mut dynamic_uniform_indices = Vec::new();
            for binding in bind_group.bindings.iter() {
                if let BindType::Uniform { dynamic, .. } = binding.bind_type {
                    if !dynamic {
                        continue;
                    }

                    if let Some(resource) = self
                        .wgpu_resources
                        .render_resources
                        .get_named_resource(&binding.name)
                    {
                        // PERF: This hashmap get is pretty expensive (10 fps for 10000 entities)
                        if let Some(dynamic_uniform_buffer_info) = self
                            .wgpu_resources
                            .dynamic_uniform_buffer_info
                            .get(&resource)
                        {
                            let index = dynamic_uniform_buffer_info
                                .offsets
                                .get(entity.unwrap())
                                .unwrap();

                            dynamic_uniform_indices.push(*index);
                        }
                    }
                }
            }

            // TODO: check to see if bind group is already set
            self.render_pass.set_bind_group(
                bind_group.index,
                &bind_group_info.bind_group,
                dynamic_uniform_indices.as_slice(),
            );
        }
    }
}
