use bevy_window::WindowResized;

use crate::{
    render_resource::{
        resource_name, BufferInfo, BufferUsage, RenderResource, RenderResourceAssignments,
        ResourceProvider,
    },
    renderer_2::RenderContext,
    ActiveCamera, Camera,
};

use bevy_app::{EventReader, Events};
use bevy_transform::prelude::*;
use legion::prelude::*;
use zerocopy::AsBytes;

pub struct CameraResourceProvider {
    pub camera_buffer: Option<RenderResource>,
    pub tmp_buffer: Option<RenderResource>,
    pub window_resized_event_reader: EventReader<WindowResized>,
}

impl CameraResourceProvider {
    pub fn new(window_resized_event_reader: EventReader<WindowResized>) -> Self {
        CameraResourceProvider {
            camera_buffer: None,
            tmp_buffer: None,
            window_resized_event_reader,
        }
    }
}

impl ResourceProvider for CameraResourceProvider {
    fn initialize(
        &mut self,
        render_context: &mut dyn RenderContext,
        _world: &mut World,
        resources: &Resources,
    ) {
        let buffer = render_context.resources_mut().create_buffer(BufferInfo {
            size: std::mem::size_of::<[[f32; 4]; 4]>(),
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });

        let mut render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();
        render_resource_assignments.set(resource_name::uniform::CAMERA, buffer);
        self.camera_buffer = Some(buffer);
    }

    fn update(
        &mut self,
        render_context: &mut dyn RenderContext,
        world: &World,
        resources: &Resources,
    ) {
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let primary_window_resized_event = window_resized_events
            .iter(&mut self.window_resized_event_reader)
            .rev()
            .filter(|event| event.is_primary)
            .next();
        if let Some(_) = primary_window_resized_event {
            let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
            for (camera, local_to_world, _) in
                <(Read<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter(world)
            {
                let camera_matrix: [[f32; 4]; 4] =
                    (camera.view_matrix * local_to_world.0).to_cols_array_2d();

                if let Some(old_tmp_buffer) = self.tmp_buffer {
                    render_context.resources_mut().remove_buffer(old_tmp_buffer);
                }

                self.tmp_buffer = Some(render_context.resources_mut().create_buffer_mapped(
                    BufferInfo {
                        size: matrix_size,
                        buffer_usage: BufferUsage::COPY_SRC,
                        ..Default::default()
                    },
                    &mut |data, _renderer| {
                        data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                    },
                ));

                render_context.copy_buffer_to_buffer(
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
