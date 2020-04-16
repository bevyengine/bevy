use bevy_window::WindowResized;

use crate::{
    render_resource::{
        resource_name, BufferInfo, BufferUsage, RenderResource, RenderResourceAssignments,
        ResourceProvider,
    },
    renderer_2::{GlobalRenderResourceContext, RenderContext},
    ActiveCamera, Camera,
};

use bevy_app::{EventReader, Events, GetEventReader};
use bevy_transform::prelude::*;
use legion::prelude::*;
use zerocopy::AsBytes;

pub fn camera_resource_provider_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut camera_buffer = None;
    let mut tmp_buffer = None;
    let mut window_resized_event_reader = resources.get_event_reader::<WindowResized>();
    SystemBuilder::new("mesh_resource_provider")
        .read_resource::<GlobalRenderResourceContext>()
        // TODO: this write on RenderResourceAssignments will prevent this system from running in parallel with other systems that do the same
        .write_resource::<RenderResourceAssignments>()
        .read_resource::<Events<WindowResized>>()
        .with_query(<(Read<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query())
        .build(
            move |_,
                  world,
                  (
                render_resource_context,
                ref mut render_resource_assignments,
                window_resized_events,
            ),
                  query| {
                let render_resources = &render_resource_context.context;
                if camera_buffer.is_none() {
                    let buffer = render_resources.create_buffer(BufferInfo {
                        size: std::mem::size_of::<[[f32; 4]; 4]>(),
                        buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                        ..Default::default()
                    });
                    render_resource_assignments.set(resource_name::uniform::CAMERA, buffer);
                    camera_buffer = Some(buffer);
                }

                let primary_window_resized_event = window_resized_events
                    .find_latest(&mut window_resized_event_reader, |event| event.is_primary);
                if let Some(_) = primary_window_resized_event {
                    let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
                    for (camera, local_to_world, _) in query.iter(world) {
                        let camera_matrix: [[f32; 4]; 4] =
                            (camera.view_matrix * local_to_world.0).to_cols_array_2d();

                        if let Some(old_tmp_buffer) = tmp_buffer {
                            render_resources.remove_buffer(old_tmp_buffer);
                        }

                        tmp_buffer = Some(render_resources.create_buffer_mapped(
                            BufferInfo {
                                size: matrix_size,
                                buffer_usage: BufferUsage::COPY_SRC,
                                ..Default::default()
                            },
                            &mut |data, _renderer| {
                                data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                            },
                        ));

                        // render_resources.copy_buffer_to_buffer(
                        //     tmp_buffer.unwrap(),
                        //     0,
                        //     camera_buffer.unwrap(),
                        //     0,
                        //     matrix_size as u64,
                        // );
                    }
                }
            },
        )
}

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
            .find_latest(&mut self.window_resized_event_reader, |event| {
                event.is_primary
            });
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
