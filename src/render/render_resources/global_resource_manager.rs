use crate::{math, prelude::LocalToWorld, render::passes::ForwardUniforms, render::*};

use legion::prelude::*;
use std::mem;
use zerocopy::AsBytes;

pub const FORWARD_UNIFORM_BUFFER_NAME: &str = "forward";

pub struct GlobalResourceManager;

impl RenderResourceManager for GlobalResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, world: &mut World) {
        let light_count = <Read<Light>>::query().iter(world).count();
        let forward_uniforms = ForwardUniforms {
            proj: math::Mat4::identity().to_cols_array_2d(),
            num_lights: [light_count as u32, 0, 0, 0],
        };

        let uniform_size = mem::size_of::<ForwardUniforms>() as wgpu::BufferAddress;
        let buffer = render_graph.device.create_buffer_with_data(
            forward_uniforms.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let uniform_buffer = UniformBuffer {
            buffer: buffer,
            size: uniform_size,
        };
        render_graph.set_uniform_buffer(FORWARD_UNIFORM_BUFFER_NAME, uniform_buffer);
    }
    fn update<'a>(
        &mut self,
        _render_graph: &mut RenderGraphData,
        _encoder: &'a mut wgpu::CommandEncoder,
        _world: &mut World,
    ) {
    }
    fn resize<'a>(
        &self,
        render_graph: &mut RenderGraphData,
        encoder: &'a mut wgpu::CommandEncoder,
        world: &mut World,
    ) {
        for (mut camera, local_to_world, _) in
            <(Write<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter_mut(world)
        {
            camera.update(
                render_graph.swap_chain_descriptor.width,
                render_graph.swap_chain_descriptor.height,
            );
            let camera_matrix: [[f32; 4]; 4] =
                (camera.view_matrix * local_to_world.0).to_cols_array_2d();
            let matrix_size = mem::size_of::<[[f32; 4]; 4]>() as u64;
            let temp_camera_buffer = render_graph
                .device
                .create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::COPY_SRC);
            let forward_uniform_buffer = render_graph
                .get_uniform_buffer(FORWARD_UNIFORM_BUFFER_NAME)
                .unwrap();
            encoder.copy_buffer_to_buffer(
                &temp_camera_buffer,
                0,
                &forward_uniform_buffer.buffer,
                0,
                matrix_size,
            );
        }
    }
}
