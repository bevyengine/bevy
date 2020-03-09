use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::render_graph::{
        resource_name, DrawTarget, PipelineDescriptor, RenderPass, RenderResource, ResourceInfo,
    },
};

use zerocopy::AsBytes;

#[derive(Default)]
pub struct UiDrawTarget {
    pub mesh_vertex_buffer: Option<RenderResource>,
    pub mesh_index_buffer: Option<RenderResource>,
    pub mesh_index_length: usize,
}

impl DrawTarget for UiDrawTarget {
    fn draw(
        &self,
        _world: &World,
        _resources: &Resources,
        render_pass: &mut dyn RenderPass,
        _pipeline_handle: Handle<PipelineDescriptor>,
    ) {
        let ui_instances_buffer = {
            let renderer = render_pass.get_renderer();
            match renderer
                .get_render_resources()
                .get_named_resource(resource_name::buffer::UI_INSTANCES)
            {
                Some(buffer) => buffer,
                None => return,
            }
        };

        let index_count = {
            let renderer = render_pass.get_renderer();
            if let Some(ResourceInfo::InstanceBuffer { count, .. }) =
                renderer.get_resource_info(ui_instances_buffer)
            {
                Some(*count)
            } else {
                None
            }
        };

        render_pass.set_bind_groups(None);
        render_pass.set_index_buffer(self.mesh_index_buffer.unwrap(), 0);
        render_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.unwrap(), 0);
        render_pass.set_vertex_buffer(1, ui_instances_buffer, 0);
        render_pass.draw_indexed(
            0..self.mesh_index_length as u32,
            0,
            0..(index_count.unwrap() as u32),
        );
    }
    fn setup(
        &mut self,
        _world: &World,
        resources: &Resources,
        renderer: &mut dyn crate::render::render_graph::Renderer,
        _pipeline_handle: Handle<PipelineDescriptor>,
    ) {
        // don't create meshes if they have already been created
        if let Some(_) = self.mesh_vertex_buffer {
            return;
        }

        let ui_instances_buffer = {
            match renderer
                .get_render_resources()
                .get_named_resource(resource_name::buffer::UI_INSTANCES)
            {
                Some(buffer) => buffer,
                None => return,
            }
        };

        if let ResourceInfo::InstanceBuffer { mesh_id, .. } =
            renderer.get_resource_info(ui_instances_buffer).unwrap()
        {
            let mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
            if let Some(mesh_asset) = mesh_storage.get_id(*mesh_id) {
                self.mesh_vertex_buffer = Some(renderer.create_buffer_with_data(
                    mesh_asset.vertices.as_bytes(),
                    wgpu::BufferUsage::VERTEX,
                ));
                self.mesh_index_buffer = Some(renderer.create_buffer_with_data(
                    mesh_asset.indices.as_bytes(),
                    wgpu::BufferUsage::INDEX,
                ));
                self.mesh_index_length = mesh_asset.indices.len();
            };
        }
    }
    fn get_name(&self) -> String {
        resource_name::draw_target::UI.to_string()
    }
}
