use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_platform::collections::hash_map::Entry;
use bevy_render::{
    render_resource::{StorageBuffer, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    Extract,
};
use bevy_utils::once;
use tracing::warn;

use super::{pipeline::AutoExposureUniform, AutoExposure};

/// Returns the correction limits, or the defaults if either value is invalid.
fn correction_bounds(settings: &AutoExposure) -> (f32, f32) {
    let (min, max) = settings.correction_range.clone().into_inner();
    if min <= max && min.is_finite() && max.is_finite() {
        (min, max)
    } else {
        once!(warn!(
            "AutoExposure::correction_range must not contain NaN or infinity, and the minimum must not be greater than the maximum; using the default range"
        ));
        (f32::MIN, f32::MAX)
    }
}

#[derive(Resource, Default)]
pub(super) struct AutoExposureBuffers {
    pub(super) buffers: EntityHashMap<AutoExposureBuffer>,
}

pub(super) struct AutoExposureBuffer {
    pub(super) state: StorageBuffer<f32>,
    pub(super) settings: UniformBuffer<AutoExposureUniform>,
}

#[derive(Resource)]
pub(super) struct ExtractedStateBuffers {
    changed: Vec<(Entity, AutoExposure)>,
    removed: Vec<Entity>,
}

pub(super) fn extract_buffers(
    mut commands: Commands,
    changed: Extract<Query<(RenderEntity, &AutoExposure), Changed<AutoExposure>>>,
    mut removed: Extract<RemovedComponents<AutoExposure>>,
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
        let (correction_min, correction_max) = correction_bounds(&settings);
        let (min_log_lum, max_log_lum) = settings.range.into_inner();
        let (low_percent, high_percent) = settings.filter.into_inner();
        // Keep the initial correction within the configured range.
        let initial_state = 0.0f32
            .clamp(min_log_lum, max_log_lum)
            .clamp(correction_min, correction_max);

        let settings = AutoExposureUniform {
            min_log_lum,
            inv_log_lum_range: 1.0 / (max_log_lum - min_log_lum),
            log_lum_range: max_log_lum - min_log_lum,
            low_percent,
            high_percent,
            speed_up: settings.speed_brighten,
            speed_down: settings.speed_darken,
            exponential_transition_distance: settings.exponential_transition_distance,
            correction_min,
            correction_max,
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

#[cfg(test)]
mod tests {
    use super::{correction_bounds, AutoExposure};

    #[test]
    fn valid_correction_ranges_are_preserved() {
        for (min, max) in [
            (f32::MIN, f32::MAX),
            (-2.0, 2.0),
            (0.0, 0.0),
            (10.0, 20.0),
            (-20.0, -10.0),
        ] {
            let settings = AutoExposure {
                correction_range: min..=max,
                ..Default::default()
            };
            assert_eq!(correction_bounds(&settings), (min, max));
        }
    }

    #[test]
    fn invalid_correction_ranges_use_default_bounds() {
        let default_bounds = correction_bounds(&AutoExposure::default());
        for (min, max) in [
            (2.0, -2.0),
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::NEG_INFINITY, 0.0),
            (0.0, f32::INFINITY),
            (f32::INFINITY, f32::INFINITY),
            (f32::NEG_INFINITY, f32::NEG_INFINITY),
        ] {
            let settings = AutoExposure {
                correction_range: min..=max,
                ..Default::default()
            };
            assert_eq!(correction_bounds(&settings), default_bounds);
        }
    }
}
