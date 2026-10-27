use bevy_camera::Exposure;
use bevy_core_pipeline::tonemapping::ViewExposureTexture;
use bevy_ecs::{entity::EntityHashMap, prelude::*, query::QueryState};
use bevy_platform::time::Instant;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{
        BufferUsages, Extent3d, StorageBuffer, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, TextureView, TextureViewDescriptor, UniformBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    MainWorld,
};
use bevy_time::{Real, Time};
use bevy_utils::once;
use tracing::warn;

use super::{
    pipeline::{AutoExposureUniform, GpuAutoExposureState},
    readback::StateReadback,
    AutoExposure, AutoExposureState,
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
    pub(super) state: StorageBuffer<GpuAutoExposureState>,
    pub(super) settings: UniformBuffer<AutoExposureUniform>,
    pub(super) readback: StateReadback,
    /// The view's [`ViewExposureTexture`], which the shader writes.
    pub(super) exposure: TextureView,
    /// When auto exposure started on the view, or was last reset.
    pub(super) started: Instant,
}

/// The views with auto exposure this frame, extracted from every camera with [`AutoExposure`].
#[derive(Resource, Default)]
pub(super) struct ExtractedAutoExposure {
    views: Vec<ExtractedAutoExposureView>,
}

pub(super) struct ExtractedAutoExposureView {
    /// The render entity.
    entity: Entity,
    settings: AutoExposureUniform,
    /// The exposure the view renders with, in EV100, if pre-exposure sets it this frame.
    pre_exposure_ev100: Option<f32>,
    /// Whether the adapted exposure restarts at this frame's target.
    reset: bool,
}

type AutoExposureCameras = QueryState<(
    RenderEntity,
    &'static mut AutoExposure,
    Option<&'static Exposure>,
    &'static mut AutoExposureState,
)>;

/// Extracts every camera with auto exposure, clears its [`AutoExposure::reset`], and sets its
/// [`AutoExposureState`] from the readback.
pub(super) fn extract_auto_exposure(
    mut main_world: ResMut<MainWorld>,
    mut extracted: ResMut<ExtractedAutoExposure>,
    buffers: Res<AutoExposureBuffers>,
    mut cameras: Local<Option<AutoExposureCameras>>,
) {
    let delta_time = main_world
        .get_resource::<Time<Real>>()
        .map_or(0.0, Time::delta_secs);
    let cameras = cameras.get_or_insert_with(|| main_world.query());

    extracted.views.clear();
    for (entity, mut auto_exposure, exposure, mut state) in cameras.iter_mut(&mut main_world) {
        let reset = auto_exposure.reset;
        if reset {
            auto_exposure.bypass_change_detection().reset = false;
        }

        let readback = buffers.buffers.get(&entity).map(|buffer| &buffer.readback);
        if reset && let Some(readback) = readback {
            readback.reset();
        }
        let ev100 = readback.and_then(StateReadback::ev100);
        state.set_if_neq(AutoExposureState { ev100 });

        let camera_ev100 = exposure.copied().unwrap_or_default().ev100;
        extracted.views.push(ExtractedAutoExposureView {
            entity,
            settings: uniform(&auto_exposure, camera_ev100, delta_time),
            pre_exposure_ev100: ev100.filter(|_| auto_exposure.pre_exposure),
            reset,
        });
    }
}

fn uniform(settings: &AutoExposure, camera_ev100: f32, delta_time: f32) -> AutoExposureUniform {
    let (correction_min, correction_max) = correction_bounds(settings);
    let (min_ev100, max_ev100) = (*settings.range.start(), *settings.range.end());
    AutoExposureUniform {
        min_ev100,
        inv_ev100_range: 1.0 / (max_ev100 - min_ev100),
        ev100_range: max_ev100 - min_ev100,
        low_percent: *settings.filter.start(),
        high_percent: *settings.filter.end(),
        speed_brighten: settings.speed_brighten,
        speed_darken: settings.speed_darken,
        exponential_transition_distance: settings.exponential_transition_distance,
        correction_min,
        correction_max,
        camera_ev100,
        delta_time,
        // Set by the auto exposure pass.
        pipelines_ready: 0,
    }
}

/// Sets the [`ExtractedCamera`] exposure of each pre-exposed view to its read-back adapted
/// exposure, before the view uniform reads it.
pub(super) fn apply_pre_exposure(
    extracted: Res<ExtractedAutoExposure>,
    mut cameras: Query<&mut ExtractedCamera>,
) {
    for view in &extracted.views {
        if let (Some(ev100), Ok(mut camera)) =
            (view.pre_exposure_ev100, cameras.get_mut(view.entity))
        {
            camera.exposure = Exposure { ev100 }.exposure();
        }
    }
}

pub(super) fn prepare_buffers(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    extracted: Res<ExtractedAutoExposure>,
    mut buffers: ResMut<AutoExposureBuffers>,
) {
    buffers.buffers.retain(|entity, _| {
        let kept = extracted.views.iter().any(|view| view.entity == *entity);
        if !kept && let Ok(mut view) = commands.get_entity(*entity) {
            view.try_remove::<ViewExposureTexture>();
        }
        kept
    });

    for view in &extracted.views {
        let buffer = buffers.buffers.entry(view.entity).or_insert_with(|| {
            let mut state = StorageBuffer::from(GpuAutoExposureState::default());
            state.add_usages(BufferUsages::COPY_SRC);
            state.write_buffer(&device, &queue);
            AutoExposureBuffer {
                state,
                settings: UniformBuffer::from(view.settings),
                readback: StateReadback::new(&device),
                exposure: exposure_texture(&device),
                started: Instant::now(),
            }
        });
        // The settings carry the frame's delta time, so they upload every frame.
        buffer.settings.set(view.settings);
        buffer.settings.write_buffer(&device, &queue);
        if view.reset {
            // The shader sets an uninitialized state on the next fully drawn frame.
            buffer.state.set(GpuAutoExposureState::default());
            buffer.state.write_buffer(&device, &queue);
            buffer.started = Instant::now();
        }
        // A readback is copied after the pass that writes the exposure texture, so the texture
        // holds an exposure once one arrives.
        if buffer.readback.ev100().is_some() {
            commands
                .entity(view.entity)
                .insert(ViewExposureTexture(buffer.exposure.clone()));
        }
    }
}

fn exposure_texture(device: &RenderDevice) -> TextureView {
    device
        .create_texture(&TextureDescriptor {
            label: Some("auto_exposure_exposure"),
            size: Extent3d::default(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Float,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
}

#[cfg(test)]
mod tests {
    use super::*;

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
