//! A `bevy_editor_cam` extension that adds the ability to smoothly transition between perspective
//! and orthographic projections using what's known as a "dolly zoom" in film. This is useful
//! because it ensures that the object the user is focusing on does not change size even as the
//! projection changes.

use std::time::Duration;

use bevy_app::prelude::*;
use bevy_camera::{prelude::*, ScalingMode};
use bevy_ecs::prelude::*;
use bevy_ecs::resource::IsResource;
use bevy_log::error_once;
use bevy_math::{prelude::*, DQuat, DVec3};
use bevy_platform::{collections::HashMap, time::Instant};
use bevy_window::RequestRedraw;

use crate::pan_orbit_camera::prelude::{
    motion::CurrentMotion, EnabledMotion, PanOrbitCamera, TransformAdapter,
};

/// See the [module](self) docs.
pub struct DollyZoomPlugin;

impl Plugin for DollyZoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DollyZoom>()
            .add_message::<DollyZoomTrigger>()
            .add_systems(
                PreUpdate,
                DollyZoom::update.before(PanOrbitCamera::update_camera_positions),
            )
            .add_systems(Last, DollyZoomTrigger::receive); // This mutates camera components, so we want to be sure it runs *after* rendering has happened. We place it in Last to ensure that we wake the next frame if needed. If we run this in PostUpdate, this can result in rendering artifacts because this will mutate projections right before rendering.
    }
}

/// Used when transitioning from ortho to perspective, this needs to be close to ortho (zero fov).
const ZERO_FOV: f64 = 1e-3;

/// Triggers a dolly zoom on the specified camera.
#[derive(Debug, Message)]
pub struct DollyZoomTrigger {
    /// The new projection.
    pub target_projection: Projection,
    /// The camera to update.
    pub camera: Entity,
}

impl DollyZoomTrigger {
    fn receive(
        mut events: MessageReader<Self>,
        mut state: ResMut<DollyZoom>,
        mut camera_set: ParamSet<(
            Query<(&Camera, Mut<Projection>, &mut PanOrbitCamera), Without<IsResource>>,
            Query<EntityMut, (With<PanOrbitCamera>, Without<IsResource>)>,
        )>,
        transform_adapter: Res<TransformAdapter>,
        mut redraw: MessageWriter<RequestRedraw>,
    ) {
        for event in events.read() {
            let mut cameras = camera_set.p0();
            let Ok((camera, mut proj, mut controller)) = cameras.get_mut(event.camera) else {
                continue;
            };
            let mut delta_translation = DVec3::ZERO;
            redraw.write(RequestRedraw);
            let (fov_start, triangle_base) = match &*proj {
                Projection::Perspective(perspective) => {
                    if let Projection::Perspective(PerspectiveProjection {
                        fov: target_fov, ..
                    }) = event.target_projection
                    {
                        // If the target and current fov are the same, there is nothing to do.
                        if (target_fov - perspective.fov).abs() <= f32::EPSILON {
                            continue;
                        }
                    }
                    (
                        perspective.fov,
                        (perspective.fov as f64 / 2.0).tan() * controller.last_anchor_depth.abs(),
                    )
                }
                Projection::Orthographic(ortho) => {
                    if matches!(event.target_projection, Projection::Orthographic(..)) {
                        // If the camera is in ortho, and wants to go to ortho, early exit.
                        continue;
                    }

                    let base = ortho.scale as f64 / ortho_tri_base_to_scale_factor(camera, ortho);
                    let new_anchor_dist = base / (ZERO_FOV / 2.0).tan();
                    let forward_dist = controller.last_anchor_depth.abs() - new_anchor_dist;
                    let cam_forward = DVec3::NEG_Z;
                    let next_translation = cam_forward * forward_dist;

                    delta_translation += next_translation;
                    controller.last_anchor_depth += forward_dist;

                    (ZERO_FOV as f32, base)
                }
                Projection::Custom(_) => {
                    error_once!("Custom projections are not supported.");
                    return;
                }
            };

            let perspective_start = PerspectiveProjection {
                fov: fov_start,
                ..Default::default()
            };
            *proj = Projection::Perspective(perspective_start.clone());

            state
                .map
                .entry(event.camera)
                .and_modify(|e| {
                    e.perspective_start = perspective_start.clone();
                    e.proj_end = event.target_projection.clone();
                    e.triangle_base = triangle_base;
                    e.start = Instant::now();
                    e.complete = false;
                })
                .or_insert(ZoomEntry {
                    perspective_start,
                    proj_end: event.target_projection.clone(),
                    triangle_base,
                    start: Instant::now(),
                    initial_enabled: controller.enabled_motion.clone(),
                    complete: false,
                });

            controller.end_move();
            controller.current_motion = CurrentMotion::Stationary;
            controller.enabled_motion = EnabledMotion {
                pan: false,
                orbit: false,
                zoom: false,
            };

            let mut camera_muts = camera_set.p1();
            let mut camera_mut = camera_muts.get_mut(event.camera).unwrap();
            transform_adapter.apply_delta(&mut camera_mut, delta_translation, DQuat::IDENTITY);
        }
    }
}

struct ZoomEntry {
    perspective_start: PerspectiveProjection,
    proj_end: Projection,
    triangle_base: f64,
    start: Instant,
    initial_enabled: EnabledMotion,
    complete: bool,
}

/// Stores settings and state for the dolly zoom plugin.
#[derive(Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct DollyZoom {
    /// The duration of the dolly zoom transition animation.
    pub animation_duration: Duration,
    /// The cubic curve used to animate the camera during a dolly zoom.
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub animation_curve: CubicSegment<Vec2>,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    map: HashMap<Entity, ZoomEntry>,
}

impl Default for DollyZoom {
    fn default() -> Self {
        Self {
            animation_duration: Duration::from_millis(300),
            animation_curve: CubicSegment::new_bezier_easing((0.65, 0.0), (0.35, 1.0)),
            map: Default::default(),
        }
    }
}

impl DollyZoom {
    fn update(
        mut state: ResMut<Self>,
        mut camera_set: ParamSet<(
            Query<(&Camera, Mut<Projection>, &mut PanOrbitCamera), Without<IsResource>>,
            Query<EntityMut, (With<PanOrbitCamera>, Without<IsResource>)>,
        )>,
        transform_adapter: Res<TransformAdapter>,
        mut redraw: MessageWriter<RequestRedraw>,
    ) {
        let animation_duration = state.animation_duration;
        let animation_curve = state.animation_curve;
        for (
            camera_entity,
            ZoomEntry {
                perspective_start,
                proj_end,
                triangle_base,
                start,
                initial_enabled,
                complete,
            },
        ) in state.map.iter_mut()
        {
            let mut cameras = camera_set.p0();
            let Ok((camera, mut projection, mut controller)) = cameras.get_mut(*camera_entity)
            else {
                *complete = true;
                continue;
            };
            let Projection::Perspective(last_perspective) = projection.clone() else {
                *projection = proj_end.clone();
                controller.enabled_motion = initial_enabled.clone();
                *complete = true;
                continue;
            };

            let mut delta_translation = DVec3::ZERO;

            let last_fov = last_perspective.fov as f64;
            let fov_start = perspective_start.fov as f64;

            let fov_end = match &*proj_end {
                Projection::Perspective(perspective) => perspective.fov as f64,
                Projection::Orthographic(_) => ZERO_FOV,
                Projection::Custom(_) => {
                    error_once!("Custom projections are not supported.");
                    return;
                }
            };
            let progress = start.elapsed().as_secs_f32() / animation_duration.as_secs_f32();
            let progress = animation_curve.ease(progress);
            let next_fov = (1.0 - progress as f64) * fov_start + progress as f64 * fov_end;

            let last_dist = *triangle_base / (last_fov / 2.0).tan();
            let next_dist = *triangle_base / (next_fov / 2.0).tan();
            let forward_dist = last_dist - next_dist;
            let cam_forward = DVec3::NEG_Z;
            let next_translation = cam_forward * forward_dist;

            delta_translation += next_translation;
            controller.last_anchor_depth += forward_dist;

            if progress < 1.0 {
                *projection = Projection::Perspective(PerspectiveProjection {
                    fov: next_fov as f32,
                    ..last_perspective
                })
            } else {
                *projection = proj_end.clone();
                if let Projection::Orthographic(ortho) = &mut *projection {
                    let multiplier = ortho_tri_base_to_scale_factor(camera, ortho);
                    ortho.scale = (*triangle_base * multiplier) as f32;
                }
                controller.enabled_motion = initial_enabled.clone();
                *complete = true;
            }
            redraw.write(RequestRedraw);

            let mut camera_muts = camera_set.p1();
            let mut camera_mut = camera_muts.get_mut(*camera_entity).unwrap();
            transform_adapter.apply_delta(&mut camera_mut, delta_translation, DQuat::IDENTITY);
        }
        state.map.retain(|_, v| !v.complete);
    }
}

fn ortho_tri_base_to_scale_factor(camera: &Camera, ortho: &OrthographicProjection) -> f64 {
    if let Some(size) = camera.logical_viewport_size() {
        let (width, height) = (size.x as f64, size.y as f64);
        2.0 / match ortho.scaling_mode {
            ScalingMode::WindowSize => height,
            ScalingMode::AutoMin {
                min_width,
                min_height,
            } => {
                if width * min_height as f64 > min_width as f64 * height {
                    min_height as f64
                } else {
                    height * min_width as f64 / width
                }
            }
            ScalingMode::AutoMax {
                max_width,
                max_height,
            } => {
                if (width * max_height as f64) < max_width as f64 * height {
                    max_height as f64
                } else {
                    height * max_width as f64 / width
                }
            }
            ScalingMode::FixedVertical { viewport_height } => viewport_height as f64,
            ScalingMode::FixedHorizontal { viewport_width } => {
                height * viewport_width as f64 / width
            }
            ScalingMode::Fixed { height, .. } => height as f64,
        }
    } else {
        0.00278
    }
}
