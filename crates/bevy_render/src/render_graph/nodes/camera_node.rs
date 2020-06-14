use crate::{
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    render_resource::{BufferInfo, BufferUsage, RenderResourceBinding, RenderResourceBindings},
    renderer::{RenderResourceContext, RenderContext},
    Camera,
};
use bevy_core::bytes::AsBytes;

use bevy_transform::prelude::*;
use legion::prelude::*;
use std::borrow::Cow;

pub struct CameraNode {
    command_queue: CommandQueue,
    uniform_name: Cow<'static, str>,
}

impl CameraNode {
    pub fn new<T>(uniform_name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        CameraNode {
            command_queue: Default::default(),
            uniform_name: uniform_name.into(),
        }
    }
}

impl Node for CameraNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

impl SystemNode for CameraNode {
    fn get_system(&self) -> Box<dyn Schedulable> {
        let mut camera_buffer = None;
        let mut command_queue = self.command_queue.clone();
        let uniform_name = self.uniform_name.clone();
        (move |world: &mut SubWorld,
               render_resource_context: Res<Box<dyn RenderResourceContext>>,
               // PERF: this write on RenderResourceAssignments will prevent this system from running in parallel
               // with other systems that do the same
               mut render_resource_bindings: ResMut<RenderResourceBindings>,
               query: &mut Query<(Read<Camera>, Read<Transform>)>| {
            let render_resource_context = &**render_resource_context;
            if camera_buffer.is_none() {
                let size = std::mem::size_of::<[[f32; 4]; 4]>();
                let buffer = render_resource_context.create_buffer(BufferInfo {
                    size,
                    buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                    ..Default::default()
                });
                render_resource_bindings.set(
                    &uniform_name,
                    RenderResourceBinding::Buffer {
                        buffer,
                        range: 0..size as u64,
                        dynamic_index: None,
                    },
                );
                camera_buffer = Some(buffer);
            }
            let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
            if let Some((camera, transform)) = query
                .iter(world)
                .find(|(camera, _)| camera.name.as_ref().map(|n| n.as_str()) == Some(&uniform_name))
            {
                let camera_matrix: [f32; 16] =
                    (camera.view_matrix * transform.value).to_cols_array();

                let tmp_buffer = render_resource_context.create_buffer_mapped(
                    BufferInfo {
                        size: matrix_size,
                        buffer_usage: BufferUsage::COPY_SRC,
                        ..Default::default()
                    },
                    &mut |data, _renderer| {
                        data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                    },
                );

                command_queue.copy_buffer_to_buffer(
                    tmp_buffer,
                    0,
                    camera_buffer.unwrap(),
                    0,
                    matrix_size as u64,
                );
                command_queue.free_buffer(tmp_buffer);
            }
        })
        .system()
    }
}
