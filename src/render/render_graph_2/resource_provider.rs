use crate::render::{render_graph_2::Renderer, ActiveCamera, Camera};
use bevy_transform::prelude::LocalToWorld;
use legion::prelude::*;
use std::mem;
use zerocopy::AsBytes;

pub trait ResourceProvider {
    fn update(&self, renderer: &mut dyn Renderer, world: &mut World);
    fn resize(&self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32);
}

pub struct CameraResourceProvider;

impl ResourceProvider for CameraResourceProvider {
    fn update(&self, renderer: &mut dyn Renderer, world: &mut World) {}
    fn resize(&self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32) {
        for (mut camera, local_to_world, _) in
            <(Write<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter_mut(world)
        {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] =
                (camera.view_matrix * local_to_world.0).to_cols_array_2d();
            let matrix_size = mem::size_of::<[[f32; 4]; 4]>() as u64;
            // TODO: use staging buffer?
            let buffer = renderer
                .create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::UNIFORM);
            // let temp_camera_buffer = render_graph
            //     .device
            //     .create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::COPY_SRC);
            // let forward_uniform_buffer = render_graph
            //     .get_uniform_buffer(FORWARD_UNIFORM_BUFFER_NAME)
            //     .unwrap();
            // encoder.copy_buffer_to_buffer(
            //     &temp_camera_buffer,
            //     0,
            //     &forward_uniform_buffer.buffer,
            //     0,
            //     matrix_size,
            // );
        }
    }
}
