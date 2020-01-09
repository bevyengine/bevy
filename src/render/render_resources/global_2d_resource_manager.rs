use crate::{render::*, math};

use legion::prelude::*;
use std::mem;
use zerocopy::{AsBytes, FromBytes};

pub const GLOBAL_2D_UNIFORM_BUFFER_NAME: &str = "global_2d";

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct Global2dUniforms {
    pub projection_matrix: [[f32; 4]; 4],
}

pub struct Global2dResourceManager;

impl RenderResourceManager for Global2dResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, _: &mut World) {
        let uniform_size = mem::size_of::<Global2dUniforms>() as wgpu::BufferAddress;
        let ui_uniforms = Global2dUniforms {
            projection_matrix: math::Mat4::identity().to_cols_array_2d(),
        };

        let buffer = render_graph.device.create_buffer_with_data(
            ui_uniforms.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let uniform_buffer = UniformBuffer {
            buffer: buffer,
            size: uniform_size,
        };
        render_graph.set_uniform_buffer(GLOBAL_2D_UNIFORM_BUFFER_NAME, uniform_buffer);
    }

    fn update<'a>(&mut self, _render_graph: &mut RenderGraphData, _encoder: &'a mut wgpu::CommandEncoder, _world: &mut World) {

    }

    fn resize<'a>(&self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) {
        for (mut camera, _) in <(Write<Camera>, Read<ActiveCamera2d>)>::query().iter(world) {
            camera.update(render_graph.swap_chain_descriptor.width, render_graph.swap_chain_descriptor.height);
            let camera_matrix: [[f32; 4]; 4] = camera.view_matrix.to_cols_array_2d();
            let matrix_size = mem::size_of::<[[f32; 4]; 4]>() as u64;
            let temp_camera_buffer =
                render_graph.device.create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::COPY_SRC);
            let global_2d_uniform_buffer = render_graph.get_uniform_buffer(GLOBAL_2D_UNIFORM_BUFFER_NAME).unwrap();
            encoder.copy_buffer_to_buffer(&temp_camera_buffer, 0, &global_2d_uniform_buffer.buffer, 0, matrix_size);
        }
    }
}