use crate::render::render_graph_2::ResourceProvider;
use crate::render::{
    render_graph_2::{resource_name, Renderer},
    ActiveCamera2d, Camera,
};
use legion::prelude::*;
use zerocopy::AsBytes;

pub struct Camera2dResourceProvider;

impl ResourceProvider for Camera2dResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, _world: &mut World) {
        renderer.create_buffer(
            resource_name::uniform::CAMERA2D,
            std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        );
    }

    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World) {}
    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32) {
        let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
        for (mut camera, _) in <(Write<Camera>, Read<ActiveCamera2d>)>::query().iter_mut(world) {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] = camera.view_matrix.to_cols_array_2d();

            renderer.create_buffer_mapped(
                "camera2d_tmp",
                matrix_size,
                wgpu::BufferUsage::COPY_SRC,
                &mut |data| {
                    data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                },
            );

            renderer.copy_buffer_to_buffer(
                "camera2d_tmp",
                0,
                resource_name::uniform::CAMERA2D,
                0,
                matrix_size as u64,
            );
        }
    }
}
