use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{encase::UniformBuffer, Buffer, BufferInitDescriptor, BufferUsages},
    renderer::RenderDevice,
    Extract,
};
use bevy_utils::HashMap;

use crate::auto_exposure::AutoExposureSettings;

use super::pipeline::AutoExposureUniform;

#[derive(Resource, Default)]
pub(super) struct AutoExposureStateBuffers {
    pub(super) buffers: HashMap<Entity, AutoExposureStateBuffer>,
}

pub(super) struct AutoExposureStateBuffer {
    pub(super) state: Buffer,
    pub(super) settings: Buffer,
}

#[derive(Resource)]
pub(super) struct ExtractedStateBuffers {
    changed: Vec<(Entity, AutoExposureSettings)>,
    removed: Vec<Entity>,
}

pub(super) fn extract_state_buffers(
    mut commands: Commands,
    changed: Extract<Query<(Entity, &AutoExposureSettings), Changed<AutoExposureSettings>>>,
    mut removed: Extract<RemovedComponents<AutoExposureSettings>>,
) {
    commands.insert_resource(ExtractedStateBuffers {
        changed: changed
            .iter()
            .map(|(entity, settings)| (entity, settings.clone()))
            .collect(),
        removed: removed.read().collect(),
    });
}

pub(super) fn prepare_state_buffers(
    device: Res<RenderDevice>,
    mut extracted: ResMut<ExtractedStateBuffers>,
    mut buffers: ResMut<AutoExposureStateBuffers>,
) {
    for (entity, settings) in extracted.changed.drain(..) {
        let (min_log_lum, max_log_lum) = settings.range.into_inner();
        let (low_percent, high_percent) = settings.filter.into_inner();
        let initial_state = 0.0f32.clamp(min_log_lum, max_log_lum);

        let state_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("auto exposure state buffer"),
            contents: &initial_state.to_ne_bytes(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let mut settings_buffer = UniformBuffer::new(Vec::new());
        settings_buffer
            .write(&AutoExposureUniform {
                min_log_lum,
                inv_log_lum_range: 1.0 / (max_log_lum - min_log_lum),
                log_lum_range: max_log_lum - min_log_lum,
                low_percent,
                high_percent,
                speed_up: settings.speed_brighten,
                speed_down: settings.speed_darken,
                exponential_transition_distance: settings.exponential_transition_distance,
            })
            .unwrap();
        let settings_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: settings_buffer.as_ref(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        buffers.buffers.insert(
            entity,
            AutoExposureStateBuffer {
                state: state_buffer,
                settings: settings_buffer,
            },
        );
    }

    for entity in extracted.removed.drain(..) {
        buffers.buffers.remove(&entity);
    }
}
