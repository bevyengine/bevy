use crate::{
    camera::{ActiveCameras, Camera},
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceContext,
    },
};
use bevy_core::bytes_of;
use bevy_ecs::{
    system::{BoxedSystem, ConfigurableSystem, Local, Query, Res, ResMut},
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
        let system = camera_node_system.config(|config| {
            config.0 = Some(CameraNodeState {
                camera_name: self.camera_name.clone(),
                command_queue: self.command_queue.clone(),
                staging_buffer: None,
            })
        });
        Box::new(system)
    }
}

const CAMERA_VIEW_PROJ: &str = "CameraViewProj";
const CAMERA_VIEW: &str = "CameraView";
const CAMERA_POSITION: &str = "CameraPosition";

#[derive(Debug, Default)]
pub struct CameraNodeState {
    command_queue: CommandQueue,
    camera_name: Cow<'static, str>,
    staging_buffer: Option<BufferId>,
}

const MATRIX_SIZE: usize = std::mem::size_of::<[[f32; 4]; 4]>();
const VEC4_SIZE: usize = std::mem::size_of::<[f32; 4]>();

pub fn camera_node_system(
    mut state: Local<CameraNodeState>,
    mut active_cameras: ResMut<ActiveCameras>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut query: Query<(&Camera, &GlobalTransform)>,
) {
    let render_resource_context = &**render_resource_context;

    let ((camera, global_transform), bindings) =
        if let Some(active_camera) = active_cameras.get_mut(&state.camera_name) {
            if let Some(entity) = active_camera.entity {
                (query.get_mut(entity).unwrap(), &mut active_camera.bindings)
            } else {
                return;
            }
        } else {
            return;
        };

    let staging_buffer = if let Some(staging_buffer) = state.staging_buffer {
        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);
        staging_buffer
    } else {
        let staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size:
                // ViewProj
                MATRIX_SIZE +
                // View
                MATRIX_SIZE +
                // Position
                VEC4_SIZE,
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: true,
        });

        state.staging_buffer = Some(staging_buffer);
        staging_buffer
    };

    if bindings.get(CAMERA_VIEW_PROJ).is_none() {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: MATRIX_SIZE,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });
        bindings.set(
            CAMERA_VIEW_PROJ,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..MATRIX_SIZE as u64,
                dynamic_index: None,
            },
        );
    }

    if bindings.get(CAMERA_VIEW).is_none() {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: MATRIX_SIZE,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });
        bindings.set(
            CAMERA_VIEW,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..MATRIX_SIZE as u64,
                dynamic_index: None,
            },
        );
    }

    if bindings.get(CAMERA_POSITION).is_none() {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: VEC4_SIZE,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });
        bindings.set(
            CAMERA_POSITION,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..VEC4_SIZE as u64,
                dynamic_index: None,
            },
        );
    }

    let view = global_transform.compute_matrix();
    let mut offset = 0;

    if let Some(RenderResourceBinding::Buffer { buffer, .. }) = bindings.get(CAMERA_VIEW) {
        render_resource_context.write_mapped_buffer(
            staging_buffer,
            0..MATRIX_SIZE as u64,
            &mut |data, _renderer| {
                data[0..MATRIX_SIZE].copy_from_slice(bytes_of(&view));
            },
        );
        state.command_queue.copy_buffer_to_buffer(
            staging_buffer,
            0,
            *buffer,
            0,
            MATRIX_SIZE as u64,
        );
        offset += MATRIX_SIZE as u64;
    }

    if let Some(RenderResourceBinding::Buffer { buffer, .. }) = bindings.get(CAMERA_VIEW_PROJ) {
        let view_proj = camera.projection_matrix * view.inverse();
        render_resource_context.write_mapped_buffer(
            staging_buffer,
            offset..(offset + MATRIX_SIZE as u64),
            &mut |data, _renderer| {
                data[0..MATRIX_SIZE].copy_from_slice(bytes_of(&view_proj));
            },
        );
        state.command_queue.copy_buffer_to_buffer(
            staging_buffer,
            offset,
            *buffer,
            0,
            MATRIX_SIZE as u64,
        );
        offset += MATRIX_SIZE as u64;
    }

    if let Some(RenderResourceBinding::Buffer { buffer, .. }) = bindings.get(CAMERA_POSITION) {
        let position: [f32; 3] = global_transform.translation.into();
        let position: [f32; 4] = [position[0], position[1], position[2], 0.0];
        render_resource_context.write_mapped_buffer(
            staging_buffer,
            offset..(offset + VEC4_SIZE as u64),
            &mut |data, _renderer| {
                data[0..VEC4_SIZE].copy_from_slice(bytes_of(&position));
            },
        );
        state.command_queue.copy_buffer_to_buffer(
            staging_buffer,
            offset,
            *buffer,
            0,
            VEC4_SIZE as u64,
        );
    }

    render_resource_context.unmap_buffer(staging_buffer);
}
