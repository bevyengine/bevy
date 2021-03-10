use crate::{
    camera::{ActiveCameras, Camera},
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext,
    },
};
use bevy_core::{AsBytes, Bytes};
use bevy_ecs::{
    system::{BoxedSystem, IntoSystem, Local, Query, Res, ResMut},
    world::World,
};

use bevy_transform::prelude::*;
use std::borrow::Cow;

#[derive(Debug)]
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
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

impl SystemNode for CameraNode {
    fn get_system(&self) -> BoxedSystem {
        let system = camera_node_system.system().config(|config| {
            config.0 = Some(CameraNodeState {
                camera_name: self.camera_name.clone(),
                command_queue: self.command_queue.clone(),
                camera_buffer: None,
                staging_buffer: None,
            })
        });
        Box::new(system)
    }
}

#[derive(Debug, Default)]
pub struct CameraNodeState {
    command_queue: CommandQueue,
    camera_name: Cow<'static, str>,
    camera_buffer: Option<BufferId>,
    staging_buffer: Option<BufferId>,
}

pub fn camera_node_system(
    mut state: Local<CameraNodeState>,
    active_cameras: Res<ActiveCameras>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    // PERF: this write on RenderResourceAssignments will prevent this system from running in parallel
    // with other systems that do the same
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    query: Query<(&Camera, &GlobalTransform)>,
) {
    let render_resource_context = &**render_resource_context;

    let (camera, global_transform) = if let Some(entity) = active_cameras.get(&state.camera_name) {
        query.get(entity).unwrap()
    } else {
        return;
    };

    let view_matrix = global_transform.compute_matrix().inverse();
    let camera_matrix = [
        (camera.projection_matrix * view_matrix).to_cols_array(),
        view_matrix.to_cols_array(),
    ];
    let buffer_size = camera_matrix.byte_len();

    let staging_buffer = if let Some(staging_buffer) = state.staging_buffer {
        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);
        staging_buffer
    } else {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: buffer_size,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });
        render_resource_bindings.set(
            &format!("{}ViewProj", &state.camera_name),
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..view_matrix.byte_len() as u64,
                dynamic_index: None,
            },
        );
        render_resource_bindings.set(
            &format!("{}View", &state.camera_name),
            RenderResourceBinding::Buffer {
                buffer,
                range: view_matrix.byte_len() as u64..buffer_size as u64,
                dynamic_index: None,
            },
        );
        state.camera_buffer = Some(buffer);

        let staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size: buffer_size,
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: true,
        });

        state.staging_buffer = Some(staging_buffer);
        staging_buffer
    };

    render_resource_context.write_mapped_buffer(
        staging_buffer,
        0..buffer_size as u64,
        &mut |data, _renderer| {
            data[0..buffer_size].copy_from_slice(camera_matrix.as_bytes());
        },
    );
    render_resource_context.unmap_buffer(staging_buffer);

    let camera_buffer = state.camera_buffer.unwrap();
    state.command_queue.copy_buffer_to_buffer(
        staging_buffer,
        0,
        camera_buffer,
        0,
        buffer_size as u64,
    );
}
