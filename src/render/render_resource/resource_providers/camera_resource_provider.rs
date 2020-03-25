use crate::render::{
    render_resource::{resource_name, BufferInfo, BufferUsage, RenderResource, ResourceProvider},
    renderer::Renderer,
    ActiveCamera, Camera,
};
use bevy_transform::prelude::LocalToWorld;
use legion::prelude::*;
use zerocopy::AsBytes;

#[derive(Default)]
pub struct CameraResourceProvider {
    pub camera_buffer: Option<RenderResource>,
    pub tmp_buffer: Option<RenderResource>,
}

impl ResourceProvider for CameraResourceProvider {
    fn initialize(
        &mut self,
        renderer: &mut dyn Renderer,
        _world: &mut World,
        _resources: &Resources,
    ) {
        let buffer = renderer.create_buffer(BufferInfo {
            size: std::mem::size_of::<[[f32; 4]; 4]>(),
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });

        renderer
            .get_render_resources_mut()
            .set_named_resource(resource_name::uniform::CAMERA, buffer);
        self.camera_buffer = Some(buffer);
    }

    fn resize(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &mut World,
        _resources: &Resources,
        width: u32,
        height: u32,
    ) {
        let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
        for (mut camera, local_to_world, _) in
            <(Write<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter_mut(world)
        {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] =
                (camera.view_matrix * local_to_world.0).to_cols_array_2d();

            if let Some(old_tmp_buffer) = self.tmp_buffer {
                renderer.remove_buffer(old_tmp_buffer);
            }

            self.tmp_buffer = Some(renderer.create_buffer_mapped(
                BufferInfo {
                    size: matrix_size,
                    buffer_usage: BufferUsage::COPY_SRC,
                    ..Default::default()
                },
                &mut |data, _renderer| {
                    data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                },
            ));

            renderer.copy_buffer_to_buffer(
                self.tmp_buffer.unwrap(),
                0,
                self.camera_buffer.unwrap(),
                0,
                matrix_size as u64,
            );
        }
    }
}
