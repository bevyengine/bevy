use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{StorageBuffer, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    Extract,
};
use bevy_utils::{Entry, HashMap};

use super::pipeline::AutoExposureSettingsUniform;
use super::AutoExposureSettings;

#[derive(Resource, Default)]
pub(super) struct AutoExposureBuffers {
    pub(super) buffers: HashMap<Entity, AutoExposureBuffer>,
}

pub(super) struct AutoExposureBuffer {
    pub(super) state: StorageBuffer<f32>,
    pub(super) settings: UniformBuffer<AutoExposureSettingsUniform>,
}

#[derive(Resource)]
pub(super) struct ExtractedStateBuffers {
    changed: Vec<(Entity, AutoExposureSettings)>,
    removed: Vec<Entity>,
}

pub(super) fn extract_buffers(
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

pub(super) fn prepare_buffers(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut extracted: ResMut<ExtractedStateBuffers>,
    mut buffers: ResMut<AutoExposureBuffers>,
) {
    for (entity, settings) in extracted.changed.drain(..) {
        let (min_log_lum, max_log_lum) = settings.range.into_inner();
        let (low_percent, high_percent) = settings.filter.into_inner();
        let initial_state = 0.0f32.clamp(min_log_lum, max_log_lum);

        let settings = AutoExposureSettingsUniform {
            min_log_lum,
            inv_log_lum_range: 1.0 / (max_log_lum - min_log_lum),
            log_lum_range: max_log_lum - min_log_lum,
            low_percent,
            high_percent,
            speed_up: settings.speed_brighten,
            speed_down: settings.speed_darken,
            exponential_transition_distance: settings.exponential_transition_distance,
        };

        match buffers.buffers.entry(entity) {
            Entry::Occupied(mut entry) => {
                // Update the settings buffer, but skip updating the state buffer.
                // The state buffer is skipped so that the animation stays continuous.
                let value = entry.get_mut();
                value.settings.set(settings);
                value.settings.write_buffer(&device, &queue);
            }
            Entry::Vacant(entry) => {
                let value = entry.insert(AutoExposureBuffer {
                    state: StorageBuffer::from(initial_state),
                    settings: UniformBuffer::from(settings),
                });

                value.state.write_buffer(&device, &queue);
                value.settings.write_buffer(&device, &queue);
            }
        }
    }

    for entity in extracted.removed.drain(..) {
        buffers.buffers.remove(&entity);
    }
}
