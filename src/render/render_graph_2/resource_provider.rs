use crate::render::{
    render_graph_2::{resource_name, Renderer},
    ActiveCamera, Camera,
};
use bevy_transform::prelude::LocalToWorld;
use legion::prelude::*;
use zerocopy::AsBytes;

pub trait ResourceProvider {
    fn initialize(&self, renderer: &mut dyn Renderer, world: &mut World);
    fn update(&self, renderer: &mut dyn Renderer, world: &mut World);
    fn resize(&self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32);
}

pub struct CameraResourceProvider;

impl ResourceProvider for CameraResourceProvider {
    fn initialize(&self, renderer: &mut dyn Renderer, world: &mut World) {
        // TODO: create real buffer here
    }

    fn update(&self, _renderer: &mut dyn Renderer, _world: &mut World) {}
    fn resize(&self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32) {
        for (mut camera, local_to_world, _) in
            <(Write<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter_mut(world)
        {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] =
                (camera.view_matrix * local_to_world.0).to_cols_array_2d();
            renderer.create_buffer_with_data(
                resource_name::uniform::CAMERA,
                camera_matrix.as_bytes(),
                wgpu::BufferUsage::UNIFORM,
            );
        }
    }
}
