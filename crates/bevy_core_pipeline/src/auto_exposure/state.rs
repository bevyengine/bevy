use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{Buffer, BufferInitDescriptor, BufferUsages},
    renderer::RenderDevice,
    Extract,
};
use bevy_utils::HashMap;

use crate::auto_exposure::AutoExposureSettings;

#[derive(Resource, Default)]
pub(super) struct AutoExposureStateBuffers {
    pub(super) buffers: HashMap<Entity, AutoExposureStateBuffer>,
}

pub(super) struct AutoExposureStateBuffer {
    pub(super) state: Buffer,
}

#[derive(Resource)]
pub(super) struct ExtractedStateBuffers {
    changed: Vec<(Entity, f32)>,
    removed: Vec<Entity>,
}

pub(super) fn extract_state_buffers(
    mut commands: Commands,
    changed: Extract<Query<(Entity, &AutoExposureSettings), Added<AutoExposureSettings>>>,
    mut removed: Extract<RemovedComponents<AutoExposureSettings>>,
) {
    commands.insert_resource(ExtractedStateBuffers {
        changed: changed
            .iter()
            .map(|(entity, settings)| {
                let (min, max) = settings.range.clone().into_inner();
                (entity, 0.0f32.clamp(min, max))
            })
            .collect(),
        removed: removed.read().collect(),
    });
}

pub(super) fn prepare_state_buffers(
    device: Res<RenderDevice>,
    mut extracted: ResMut<ExtractedStateBuffers>,
    mut buffers: ResMut<AutoExposureStateBuffers>,
) {
    for (entity, initial_state) in extracted.changed.drain(..) {
        buffers.buffers.insert(
            entity,
            AutoExposureStateBuffer {
                state: device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("auto exposure state buffer"),
                    contents: &initial_state.to_ne_bytes(),
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                }),
            },
        );
    }

    for entity in extracted.removed.drain(..) {
        buffers.buffers.remove(&entity);
    }
}
