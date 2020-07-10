use crate::{
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    render_resource::{
        BufferId, BufferInfo, BufferUsage, RenderResourceBinding, RenderResourceBindings,
    },
    renderer::{RenderContext, RenderResourceContext},
    ActiveCameras, Camera,
};
use bevy_core::bytes::AsBytes;

use bevy_ecs::{Commands, IntoQuerySystem, Local, Query, Res, ResMut, Resources, System, World};
use bevy_transform::prelude::*;
use std::borrow::Cow;

pub struct CameraNode {
    command_queue: CommandQueue,
    camera_name: Cow<'static, str>,
}

impl CameraNode {
    pub fn new<T>(camera_name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        CameraNode {
            command_queue: Default::default(),
            camera_name: camera_name.into(),
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
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = camera_node_system.system();
        commands.insert_local_resource(
            system.id(),
            CameraNodeState {
                camera_name: self.camera_name.clone(),
                command_queue: self.command_queue.clone(),
                camera_buffer: None,
            },
        );
        system
    }
}

#[derive(Default)]
pub struct CameraNodeState {
    command_queue: CommandQueue,
    camera_name: Cow<'static, str>,
    camera_buffer: Option<BufferId>,
}

pub fn camera_node_system(
    mut state: Local<CameraNodeState>,
    active_cameras: Res<ActiveCameras>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    // PERF: this write on RenderResourceAssignments will prevent this system from running in parallel
    // with other systems that do the same
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    query: Query<(&Camera, &Transform)>,
) {
    let render_resource_context = &**render_resource_context;
    state.camera_buffer = Some(match state.camera_buffer {
        Some(buffer) => buffer,
        None => {
            let size = std::mem::size_of::<[[f32; 4]; 4]>();
            let buffer = render_resource_context.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                ..Default::default()
            });
            render_resource_bindings.set(
                &state.camera_name,
                RenderResourceBinding::Buffer {
                    buffer,
                    range: 0..size as u64,
                    dynamic_index: None,
                },
            );
            state.camera_buffer = Some(buffer);
            buffer
        }
    });
    let (camera, transform) = if let Some(camera_entity) = active_cameras.get(&state.camera_name) {
        (
            query.get::<Camera>(camera_entity).unwrap(),
            query.get::<Transform>(camera_entity).unwrap(),
        )
    } else {
        return;
    };

    let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
    let camera_matrix: [f32; 16] =
        (camera.projection_matrix * transform.value.inverse()).to_cols_array();

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

    let camera_buffer = state.camera_buffer.unwrap();
    state
        .command_queue
        .copy_buffer_to_buffer(tmp_buffer, 0, camera_buffer, 0, matrix_size as u64);
    state.command_queue.free_buffer(tmp_buffer);
}
