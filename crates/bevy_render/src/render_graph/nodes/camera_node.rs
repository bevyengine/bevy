use crate::{
    camera::{ActiveCameras, Camera},
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext,
    },
};
use bevy_core::{AsBytes, Byteable, Bytes};
use bevy_ecs::{
    system::{BoxedSystem, IntoSystem, Local, Query, Res, ResMut},
    world::World,
};

use bevy_transform::prelude::*;
use bevy_utils::HashMap;
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
                buffers: HashMap::default(),
            });
        });
        Box::new(system)
    }
}

#[derive(Debug, Default)]
pub struct CameraNodeState {
    command_queue: CommandQueue,
    camera_name: Cow<'static, str>,
    buffers: HashMap<String, (BufferId, BufferId)>,
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
    let camera_name = state.camera_name.clone();

    let (camera, global_transform) = if let Some(entity) = active_cameras.get(&camera_name) {
        query.get(entity).unwrap()
    } else {
        return;
    };

    let view_matrix = global_transform.compute_matrix().inverse();
    let view_proj_matrix = camera.projection_matrix * view_matrix;

    make_binding(
        &mut *state,
        &**render_resource_context,
        &mut *render_resource_bindings,
        &format!("{}ViewProj", camera_name),
        view_proj_matrix.to_cols_array(),
    );

    make_binding(
        &mut *state,
        &**render_resource_context,
        &mut *render_resource_bindings,
        &format!("{}View", camera_name),
        view_matrix.to_cols_array(),
    );
}

fn make_binding<B>(
    state: &mut CameraNodeState,
    render_resource_context: &dyn RenderResourceContext,
    render_resource_bindings: &mut RenderResourceBindings,
    binding_name: &str,
    bytes: B,
) where
    B: Bytes + Byteable,
{
    let buffer_size = bytes.byte_len();

    let buffers_entry = state.buffers.entry(binding_name.to_owned());

    let (staging_buffer, buffer) = buffers_entry
        .and_modify(|(staging_buffer, _)| {
            render_resource_context.map_buffer(*staging_buffer, BufferMapMode::Write);
        })
        .or_insert_with(|| {
            let buffer = render_resource_context.create_buffer(BufferInfo {
                size: buffer_size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                ..Default::default()
            });

            render_resource_bindings.set(
                &binding_name,
                RenderResourceBinding::Buffer {
                    buffer,
                    range: 0..buffer_size as u64,
                    dynamic_index: None,
                },
            );

            let staging_buffer = render_resource_context.create_buffer(BufferInfo {
                size: buffer_size,
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                mapped_at_creation: true,
            });
            (staging_buffer, buffer)
        });

    render_resource_context.write_mapped_buffer(
        *staging_buffer,
        0..buffer_size as u64,
        &mut |data, _renderer| {
            data[0..buffer_size].copy_from_slice(bytes.as_bytes());
        },
    );
    render_resource_context.unmap_buffer(*staging_buffer);

    state
        .command_queue
        .copy_buffer_to_buffer(*staging_buffer, 0, *buffer, 0, buffer_size as u64);
}
