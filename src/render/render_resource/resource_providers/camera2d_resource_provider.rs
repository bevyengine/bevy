use crate::{
    core::WindowResized,
    prelude::*,
    render::{
        render_resource::{
            BufferInfo, BufferUsage, RenderResource, RenderResourceAssignments, ResourceProvider,
        },
        renderer::Renderer,
    },
};
use zerocopy::AsBytes;

pub struct Camera2dResourceProvider {
    pub camera_buffer: Option<RenderResource>,
    pub tmp_buffer: Option<RenderResource>,
    pub window_resized_event_reader: EventReader<WindowResized>,
}

impl Camera2dResourceProvider {
    pub fn new(window_resized_event_reader: EventReader<WindowResized>) -> Self {
        Camera2dResourceProvider {
            camera_buffer: None,
            tmp_buffer: None,
            window_resized_event_reader,
        }
    }
}

impl ResourceProvider for Camera2dResourceProvider {
    fn initialize(
        &mut self,
        renderer: &mut dyn Renderer,
        _world: &mut World,
        resources: &Resources,
    ) {
        let buffer = renderer.create_buffer(BufferInfo {
            size: std::mem::size_of::<[[f32; 4]; 4]>(),
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });

        let mut render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();
        render_resource_assignments.set(resource_name::uniform::CAMERA2D, buffer);
        self.camera_buffer = Some(buffer);
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources) {
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let primary_window_resized_event = window_resized_events
            .iter(&mut self.window_resized_event_reader)
            .rev()
            .filter(|event| event.is_primary)
            .next();

        if let Some(primary_window_resized_event) = primary_window_resized_event {
            let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
            for (mut camera, _) in <(Write<Camera>, Read<ActiveCamera2d>)>::query().iter_mut(world)
            {
                camera.update(
                    primary_window_resized_event.width,
                    primary_window_resized_event.height,
                );
                let camera_matrix: [[f32; 4]; 4] = camera.view_matrix.to_cols_array_2d();

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
}
