use crate::{
    camera::{ActiveCameras, Camera},
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        BufferId, BufferInfo, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext,
    },
};
use bevy_core::AsBytes;

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
                camera_buffer: None,
            },
        );
        system
    }
}

#[derive(Default)]
pub struct CameraNodeState {
    camera_name: Cow<'static, str>,
    camera_buffer: Option<BufferId>,
}

pub fn camera_node_system(
    mut state: Local<CameraNodeState>,
    active_cameras: Res<ActiveCameras>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    // PERF: this write on RenderResourceAssignments will prevent this system from running in parallel
    // with other systems that do the same
    //
    // If the `camera_buffer` could be created when creating this system, this
    // wouldn't need to be here.
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    // PERF: Once `Either` queries are merged (#218), this should be changed
    // to: Query<Either<Mutated<Camera>, Mutated<Transform>>>
    //
    // However, since `<gpu>.write_buffer` is much faster than `<gpu>.map_buffer` + `<gpu>.write_mapped_buffer`,
    // we're still better of than before.
    query: Query<(&Camera, &Transform)>,
) {
    let render_resource_context = &**render_resource_context;

    let (camera, transform) = if let Some(camera_entity) = active_cameras.get(&state.camera_name) {
        (
            query.get::<Camera>(camera_entity).unwrap(),
            query.get::<Transform>(camera_entity).unwrap(),
        )
    } else {
        return;
    };

    let state = &mut *state;
    let camera_name = &state.camera_name;
    let camera_buffer = &mut state.camera_buffer;

    let camera_buffer = *camera_buffer.get_or_insert_with(|| {
        let size = std::mem::size_of::<[[f32; 4]; 4]>();
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size,
            buffer_usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        render_resource_bindings.set(
            camera_name,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..size as u64,
                dynamic_index: None,
            },
        );
        buffer
    });

    let camera_matrix: [f32; 16] =
        (camera.projection_matrix * transform.value.inverse()).to_cols_array();

    render_resource_context.write_buffer(camera_buffer, 0, camera_matrix.as_bytes());
}
