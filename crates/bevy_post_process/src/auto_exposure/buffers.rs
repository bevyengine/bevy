use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_platform::{collections::hash_map::Entry, time::Instant};
use bevy_render::{
    render_resource::{StorageBuffer, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    Extract,
};
use bevy_time::{Real, Time};
use bevy_utils::once;
use tracing::warn;

use super::{
    pipeline::{AutoExposureState, AutoExposureUniform},
    AutoExposure,
};

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
    pub(super) state: StorageBuffer<AutoExposureState>,
    pub(super) settings: UniformBuffer<AutoExposureUniform>,
    /// When auto exposure started on the view.
    pub(super) started: Instant,
}

#[derive(Resource)]
pub(super) struct ExtractedStateBuffers {
    changed: Vec<(Entity, AutoExposure)>,
    removed: Vec<Entity>,
    /// Real time since the previous frame, in seconds.
    delta_time: f32,
}

pub(super) fn extract_buffers(
    mut commands: Commands,
    changed: Extract<Query<(RenderEntity, &AutoExposure), Changed<AutoExposure>>>,
    mut removed: Extract<RemovedComponents<AutoExposure>>,
    time: Extract<Res<Time<Real>>>,
) {
    commands.insert_resource(ExtractedStateBuffers {
        changed: changed
            .iter()
            .map(|(entity, settings)| (entity, settings.clone()))
            .collect(),
        removed: removed.read().collect(),
        delta_time: time.delta_secs(),
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
        let (min_ev100, max_ev100) = settings.range.into_inner();
        let (low_percent, high_percent) = settings.filter.into_inner();

        let settings = AutoExposureUniform {
            min_ev100,
            inv_ev100_range: 1.0 / (max_ev100 - min_ev100),
            ev100_range: max_ev100 - min_ev100,
            low_percent,
            high_percent,
            speed_brighten: settings.speed_brighten,
            speed_darken: settings.speed_darken,
            exponential_transition_distance: settings.exponential_transition_distance,
            correction_min,
            correction_max,
            // Set for every view below.
            delta_time: 0.0,
            // Set by the auto exposure pass.
            pipelines_ready: 0,
        };

        match buffers.buffers.entry(entity) {
            Entry::Occupied(mut entry) => {
                // Update the settings buffer, but skip updating the state buffer.
                // The state buffer is skipped so that the animation stays continuous.
                entry.get_mut().settings.set(settings);
            }
            Entry::Vacant(entry) => {
                // The shader sets the state on the first fully drawn frame.
                let value = entry.insert(AutoExposureBuffer {
                    state: StorageBuffer::from(AutoExposureState::default()),
                    settings: UniformBuffer::from(settings),
                    started: Instant::now(),
                });
                value.state.write_buffer(&device, &queue);
            }
        }
    }

    for entity in extracted.removed.drain(..) {
        buffers.buffers.remove(&entity);
    }

    // The settings carry the frame's delta time, so they upload every frame.
    for value in buffers.buffers.values_mut() {
        value.settings.get_mut().delta_time = extracted.delta_time;
        value.settings.write_buffer(&device, &queue);
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
