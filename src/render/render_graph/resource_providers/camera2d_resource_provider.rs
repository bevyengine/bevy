use crate::render::{
    render_graph::{resource_name, RenderResource, Renderer, ResourceProvider},
    ActiveCamera2d, Camera,
};
use legion::prelude::*;
use zerocopy::AsBytes;

#[derive(Default)]
pub struct Camera2dResourceProvider {
    pub camera_buffer: Option<RenderResource>,
    pub tmp_buffer: Option<RenderResource>,
}

impl ResourceProvider for Camera2dResourceProvider {
    fn initialize(
        &mut self,
        renderer: &mut dyn Renderer,
        _world: &mut World,
        _resources: &Resources,
    ) {
        let buffer = renderer.create_buffer(
            std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        );

        renderer
            .get_render_resources_mut()
            .set_named_resource(resource_name::uniform::CAMERA2D, buffer);
        self.camera_buffer = Some(buffer);
    }

    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World, _resources: &Resources) {
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
        for (mut camera, _) in <(Write<Camera>, Read<ActiveCamera2d>)>::query().iter_mut(world) {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] = camera.view_matrix.to_cols_array_2d();

            if let Some(old_tmp_buffer) = self.tmp_buffer {
                renderer.remove_buffer(old_tmp_buffer);
            }

            self.tmp_buffer = Some(renderer.create_buffer_mapped(
                matrix_size,
                wgpu::BufferUsage::COPY_SRC,
                &mut |data| {
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
