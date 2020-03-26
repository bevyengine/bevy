use crate::{
    asset::{Asset, Handle},
    legion::prelude::*,
    math,
    prelude::MeshType,
    render::{
        draw_target::DrawTarget,
        mesh::Mesh,
        pipeline::PipelineDescriptor,
        render_resource::{resource_name, BufferInfo, BufferUsage, RenderResource, ResourceInfo},
        renderer::{RenderPass, Renderer},
    },
};

use zerocopy::AsBytes;

#[derive(Default)]
pub struct UiDrawTarget {
    pub mesh_vertex_buffer: Option<RenderResource>,
    pub mesh_index_buffer: Option<RenderResource>,
    pub mesh_index_length: usize,
    pub mesh: Option<Handle<Mesh>>,
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
            if let Some(ResourceInfo::Buffer(BufferInfo {
                array_info: Some(array_info),
                ..
            })) = renderer.get_resource_info(ui_instances_buffer)
            {
                Some(array_info.item_count)
            } else {
                None
            }
        };

        render_pass.set_render_resource_assignments(None);
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
        _resources: &Resources,
        renderer: &mut dyn Renderer,
        _pipeline_handle: Handle<PipelineDescriptor>,
    ) {
        // don't create meshes if they have already been created
        if let Some(_) = self.mesh_vertex_buffer {
            return;
        }

        let quad = Mesh::load(MeshType::Quad {
            north_west: math::vec2(-0.5, 0.5),
            north_east: math::vec2(0.5, 0.5),
            south_west: math::vec2(-0.5, -0.5),
            south_east: math::vec2(0.5, -0.5),
        });
        self.mesh_vertex_buffer = Some(renderer.create_buffer_with_data(
            BufferInfo {
                buffer_usage: BufferUsage::VERTEX,
                ..Default::default()
            },
            quad.vertices.as_bytes(),
        ));
        self.mesh_index_buffer = Some(renderer.create_buffer_with_data(
            BufferInfo {
                buffer_usage: BufferUsage::INDEX,
                ..Default::default()
            },
            quad.indices.as_bytes(),
        ));
        self.mesh_index_length = quad.indices.len();
    }
    fn get_name(&self) -> String {
        resource_name::draw_target::UI.to_string()
    }
}
