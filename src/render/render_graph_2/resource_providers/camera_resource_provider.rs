use crate::{render::{
    render_graph_2::{resource_name, Renderer},
    ActiveCamera, Camera,
}};
use bevy_transform::prelude::LocalToWorld;
use legion::prelude::*;
use zerocopy::AsBytes;
use crate::render::render_graph_2::ResourceProvider;

pub struct CameraResourceProvider;

impl ResourceProvider for CameraResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, _world: &mut World) {
        renderer.create_buffer(
            resource_name::uniform::CAMERA,
            std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        );
    }

    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World) {}
    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32) {
        let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
        for (mut camera, local_to_world, _) in
            <(Write<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter_mut(world)
        {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] =
                (camera.view_matrix * local_to_world.0).to_cols_array_2d();

            renderer.create_buffer_mapped(
                "camera_tmp",
                matrix_size,
                wgpu::BufferUsage::COPY_SRC,
                &mut |data| {
                    data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                },
            );

            renderer.copy_buffer_to_buffer(
                "camera_tmp",
                0,
                resource_name::uniform::CAMERA,
                0,
                matrix_size as u64,
            );
        }
    }
}