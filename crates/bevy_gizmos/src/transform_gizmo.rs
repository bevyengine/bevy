//! Interactive transform gizmo for translating, rotating, and scaling entities.
//!
//! This module provides an opt-in transform gizmo that renders visual handles on a
//! focused entity, allowing the user to click-and-drag to translate, rotate, or scale
//! it. The plugin does **not** handle keyboard input — users set
//! [`TransformGizmoSettings::mode`] however they like (keyboard shortcuts, UI buttons,
//! gamepad, etc.).
//!
//! # Quick start
//!
//! 1. Add [`TransformGizmoPlugin`] to your app.
//! 2. Mark the camera with [`TransformGizmoCamera`].
//! 3. Tag the entity you want to manipulate with [`TransformGizmoFocus`].
//!
//! If there is exactly one camera in the world, the [`TransformGizmoCamera`] marker
//! is optional — the gizmo will use that camera automatically. When multiple cameras
//! exist, the marker is required so the gizmo knows which one to use.

use bevy_app::{App, Plugin, Startup, Update};
use bevy_camera::Camera;
use bevy_color::{Alpha, Color};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    query::With,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Local, Query, Res, ResMut, Single},
};
use bevy_input::{mouse::MouseButton, ButtonInput};
use bevy_math::{Isometry3d, Quat, Ray3d, Vec2, Vec3};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window};

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

const AXIS_LENGTH: f32 = 1.0;
const AXIS_TIP_LENGTH: f32 = 0.25;
const AXIS_START_OFFSET: f32 = 0.2;
const ROTATE_RING_RADIUS: f32 = 1.0;
const SCALE_CUBE_SIZE: f32 = 0.07;

const COLOR_X: Color = Color::srgb(1.0, 0.2, 0.2);
const COLOR_Y: Color = Color::srgb(0.2, 1.0, 0.2);
const COLOR_Z: Color = Color::srgb(0.2, 0.4, 1.0);
const COLOR_X_BRIGHT: Color = Color::srgb(1.0, 0.5, 0.5);
const COLOR_Y_BRIGHT: Color = Color::srgb(0.5, 1.0, 0.5);
const COLOR_Z_BRIGHT: Color = Color::srgb(0.5, 0.7, 1.0);

const MIN_SCALE: f32 = 0.01;
const AXIS_HIT_DISTANCE: f32 = 35.0;
const INACTIVE_ALPHA: f32 = 0.15;
const SCALE_SENSITIVITY: f32 = 0.005;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct TransformGizmoGroup;

/// Component that marks the entity the transform gizmo operates on.
///
/// Only one entity should carry this at a time. If multiple entities have it,
/// the gizmo picks the first one returned by the query.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub struct TransformGizmoFocus;

/// Marker component for the camera the transform gizmo should use.
///
/// When exactly one camera exists, this marker is optional. When multiple cameras
/// exist, add this to the camera the gizmo should project through. If multiple
/// cameras carry this marker, the first one found is used and a warning is logged.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub struct TransformGizmoCamera;

/// Which manipulation mode the gizmo is in.
#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Reflect)]
pub enum TransformGizmoMode {
    /// Move the entity along an axis.
    #[default]
    Translate,
    /// Rotate the entity around an axis.
    Rotate,
    /// Scale the entity along an axis.
    Scale,
}

/// Whether the gizmo transforms the object using world or local space axes.
#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Reflect)]
pub enum TransformGizmoSpace {
    /// Axes are aligned to the world.
    #[default]
    World,
    /// Axes follow the entity's local rotation.
    Local,
}

/// Which axis the user is interacting with.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum TransformGizmoAxis {
    /// The X axis (red).
    X,
    /// The Y axis (green).
    Y,
    /// The Z axis (blue).
    Z,
}

/// Configuration and preferences for the transform gizmo.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct TransformGizmoSettings {
    /// Which manipulation mode the gizmo is in.
    pub mode: TransformGizmoMode,
    /// Whether the gizmo operates in world or local space.
    pub space: TransformGizmoSpace,
    /// Length of the axis handles.
    pub axis_length: f32,
    /// Radius of the rotation rings.
    pub rotate_ring_radius: f32,
    /// Screen-space pixel distance for hover detection.
    pub axis_hit_distance: f32,
    /// If set, translation snaps to this increment.
    pub snap_translate: Option<f32>,
    /// If set, rotation snaps to this increment (radians).
    pub snap_rotate: Option<f32>,
    /// If set, scale snaps to this increment.
    pub snap_scale: Option<f32>,
    /// Whether to confine the cursor during drag.
    pub confine_cursor: bool,
    /// Screen-space scale factor. Set to 0.0 to disable constant-size behavior.
    pub screen_scale_factor: f32,
    /// Line width for gizmo rendering.
    pub line_width: f32,
}

impl Default for TransformGizmoSettings {
    fn default() -> Self {
        Self {
            mode: TransformGizmoMode::default(),
            space: TransformGizmoSpace::default(),
            axis_length: AXIS_LENGTH,
            rotate_ring_radius: ROTATE_RING_RADIUS,
            axis_hit_distance: AXIS_HIT_DISTANCE,
            snap_translate: None,
            snap_rotate: None,
            snap_scale: None,
            confine_cursor: true,
            screen_scale_factor: 0.1,
            line_width: 3.0,
        }
    }
}

/// Runtime state of the transform gizmo (drag and hover).
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct TransformGizmoState {
    /// The axis under the cursor, if any.
    pub hovered_axis: Option<TransformGizmoAxis>,
    /// `true` while the user is actively dragging.
    pub active: bool,
    /// The axis being dragged, if any.
    pub axis: Option<TransformGizmoAxis>,
    /// The transform snapshot taken when the drag started.
    pub start_transform: Transform,
    /// The entity being dragged, if any.
    pub entity: Option<Entity>,
    /// World-space point (or normalized direction for rotation) where the drag started.
    pub drag_start_world: Vec3,
    /// World-space gizmo origin at drag start.
    pub gizmo_origin: Vec3,
}

/// System set for the transform gizmo. All transform gizmo systems run in [`Update`]
/// within this set.
///
/// Add a run condition to control when the gizmo is active:
/// ```ignore
/// app.configure_sets(Update, TransformGizmoSystems.run_if(in_state(AppState::Editor)));
/// ```
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TransformGizmoSystems;

/// Opt-in plugin that adds the interactive transform gizmo.
///
/// Add this alongside your camera and mark entities with [`TransformGizmoFocus`]
/// to enable manipulation.
pub struct TransformGizmoPlugin;

impl Plugin for TransformGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransformGizmoSettings>()
            .init_resource::<TransformGizmoState>()
            .init_gizmo_group::<TransformGizmoGroup>()
            .register_type::<TransformGizmoFocus>()
            .register_type::<TransformGizmoCamera>()
            .register_type::<TransformGizmoSettings>()
            .register_type::<TransformGizmoState>()
            .add_systems(Startup, configure_transform_gizmo_group)
            .add_systems(
                Update,
                (
                    sync_transform_gizmo_settings,
                    transform_gizmo_hover,
                    transform_gizmo_drag,
                    transform_gizmo_draw,
                )
                    .chain()
                    .in_set(TransformGizmoSystems),
            );
    }
}

fn configure_transform_gizmo_group(
    mut config_store: ResMut<GizmoConfigStore>,
    settings: Res<TransformGizmoSettings>,
) {
    let (gizmo_config, _) = config_store.config_mut::<TransformGizmoGroup>();
    gizmo_config.depth_bias = -1.0;
    gizmo_config.line.width = settings.line_width;
}

fn sync_transform_gizmo_settings(
    mut config_store: ResMut<GizmoConfigStore>,
    settings: Res<TransformGizmoSettings>,
) {
    if settings.is_changed() {
        let (gizmo_config, _) = config_store.config_mut::<TransformGizmoGroup>();
        gizmo_config.line.width = settings.line_width;
    }
}

/// Resolves which camera the gizmo should use.
///
/// Prefers cameras marked with [`TransformGizmoCamera`]. Falls back to the sole
/// camera in the world when no marker is present, and warns when ambiguous.
macro_rules! resolve_gizmo_camera {
    ($marked:expr, $all:expr) => {{
        let mut marked_iter = $marked.iter();
        if let Some(first) = marked_iter.next() {
            if marked_iter.next().is_some() {
                bevy_log::warn_once!(
                    "Multiple cameras have the TransformGizmoCamera component; \
                     using the first one found."
                );
            }
            Some(first)
        } else {
            let mut all_iter = $all.iter();
            match (all_iter.next(), all_iter.next()) {
                (Some(cam), None) => Some(cam),
                (Some(_), Some(_)) => {
                    bevy_log::warn_once!(
                        "Multiple cameras exist but none has the TransformGizmoCamera \
                         component. Add TransformGizmoCamera to the camera the gizmo \
                         should use."
                    );
                    None
                }
                _ => None,
            }
        }
    }};
}

fn transform_gizmo_hover(
    focus: Option<Single<&GlobalTransform, With<TransformGizmoFocus>>>,
    marked_cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    all_cameras: Query<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<TransformGizmoSettings>,
    mut state: ResMut<TransformGizmoState>,
) {
    state.hovered_axis = None;

    if state.active {
        return;
    }

    let Some(global_tf) = focus else {
        return;
    };
    let Some((camera, cam_tf)) = resolve_gizmo_camera!(marked_cameras, all_cameras) else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let gizmo_pos = global_tf.translation();
    let effective_space = if settings.mode == TransformGizmoMode::Scale {
        &TransformGizmoSpace::Local
    } else {
        &settings.space
    };
    let rotation = gizmo_rotation(*global_tf, effective_space);

    let scale = if settings.screen_scale_factor > 0.0 {
        (cam_tf.translation() - gizmo_pos).length() * settings.screen_scale_factor
    } else {
        1.0
    };

    let axes = [
        (TransformGizmoAxis::X, rotation * Vec3::X),
        (TransformGizmoAxis::Y, rotation * Vec3::Y),
        (TransformGizmoAxis::Z, rotation * Vec3::Z),
    ];

    let mut best_axis = None;
    let mut best_dist = f32::MAX;
    let threshold = settings.axis_hit_distance;

    for (axis, dir) in &axes {
        let dist = match settings.mode {
            TransformGizmoMode::Translate | TransformGizmoMode::Scale => {
                let start = gizmo_pos + *dir * (AXIS_START_OFFSET * scale);
                let endpoint = gizmo_pos + *dir * (settings.axis_length * scale);
                let Some(start_screen) = camera.world_to_viewport(cam_tf, start).ok() else {
                    continue;
                };
                let Some(end_screen) = camera.world_to_viewport(cam_tf, endpoint).ok() else {
                    continue;
                };
                point_to_segment_dist(cursor_pos, start_screen, end_screen)
            }
            TransformGizmoMode::Rotate => point_to_ring_screen_dist(
                cursor_pos,
                camera,
                cam_tf,
                gizmo_pos,
                *dir,
                settings.rotate_ring_radius * scale,
            ),
        };
        if dist < threshold && dist < best_dist {
            best_dist = dist;
            best_axis = Some(*axis);
        }
    }

    state.hovered_axis = best_axis;
}

fn transform_gizmo_drag(
    mut focus_query: Query<(Entity, &GlobalTransform, &mut Transform), With<TransformGizmoFocus>>,
    marked_cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    all_cameras: Query<(&Camera, &GlobalTransform)>,
    primary_window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<TransformGizmoSettings>,
    mut state: ResMut<TransformGizmoState>,
    mut saved_grab_mode: Local<CursorGrabMode>,
) {
    let Some((camera, cam_tf)) = resolve_gizmo_camera!(marked_cameras, all_cameras) else {
        return;
    };
    let (window, mut cursor_opts) = primary_window.into_inner();
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Start drag
    if mouse.just_pressed(MouseButton::Left) && !state.active {
        if let Some(axis) = state.hovered_axis
            && let Some((entity, global_tf, transform)) = focus_query.iter().next()
        {
            let effective_space = if settings.mode == TransformGizmoMode::Scale {
                &TransformGizmoSpace::Local
            } else {
                &settings.space
            };
            let rotation = gizmo_rotation(global_tf, effective_space);
            let axis_dir = axis_direction(axis, rotation);
            let gizmo_pos = global_tf.translation();

            // Compute initial ray-plane intersection
            let Ok(ray) = camera.viewport_to_world(cam_tf, cursor_pos) else {
                return;
            };

            let drag_start_world = match settings.mode {
                TransformGizmoMode::Translate | TransformGizmoMode::Scale => {
                    let plane_normal = translation_plane_normal(ray, axis_dir);
                    let Some(intersection) = intersect_plane(ray, plane_normal, gizmo_pos) else {
                        return;
                    };
                    // Project onto axis and store as a point on the axis
                    let cursor_vec = intersection - gizmo_pos;
                    cursor_vec.dot(axis_dir.normalize()) * axis_dir.normalize() + gizmo_pos
                }
                TransformGizmoMode::Rotate => {
                    let Some(intersection) = intersect_plane(ray, axis_dir.normalize(), gizmo_pos)
                    else {
                        return;
                    };
                    (intersection - gizmo_pos).normalize()
                }
            };

            state.active = true;
            state.axis = Some(axis);
            state.start_transform = *transform;
            state.entity = Some(entity);
            state.drag_start_world = drag_start_world;
            state.gizmo_origin = gizmo_pos;

            if settings.confine_cursor {
                *saved_grab_mode = cursor_opts.grab_mode;
                cursor_opts.grab_mode = CursorGrabMode::Confined;
            }
        }
        return;
    }

    // Continue drag
    if state.active && mouse.pressed(MouseButton::Left) {
        let Some(drag_entity) = state.entity else {
            return;
        };
        let Some(axis) = state.axis else {
            return;
        };
        let Ok((_, global_tf, mut transform)) = focus_query.get_mut(drag_entity) else {
            return;
        };

        let effective_space = if settings.mode == TransformGizmoMode::Scale {
            &TransformGizmoSpace::Local
        } else {
            &settings.space
        };
        let rotation = gizmo_rotation(global_tf, effective_space);
        let axis_dir = axis_direction(axis, rotation);
        let gizmo_origin = state.gizmo_origin;

        let Ok(ray) = camera.viewport_to_world(cam_tf, cursor_pos) else {
            return;
        };

        match settings.mode {
            TransformGizmoMode::Translate => {
                let plane_normal = translation_plane_normal(ray, axis_dir);
                let Some(intersection) = intersect_plane(ray, plane_normal, gizmo_origin) else {
                    return;
                };
                let cursor_vec = intersection - gizmo_origin;
                let axis_norm = axis_dir.normalize();
                let new_projected = cursor_vec.dot(axis_norm) * axis_norm + gizmo_origin;
                let delta = new_projected - state.drag_start_world;

                let new_pos = state.start_transform.translation + delta;
                transform.translation = match settings.snap_translate {
                    Some(inc) => snap_axis(new_pos, state.start_transform.translation, axis, inc),
                    None => new_pos,
                };
            }
            TransformGizmoMode::Rotate => {
                let Some(intersection) = intersect_plane(ray, axis_dir.normalize(), gizmo_origin)
                else {
                    return;
                };
                let cursor_vector = (intersection - gizmo_origin).normalize();
                let drag_start = state.drag_start_world; // normalized direction

                let dot = drag_start.dot(cursor_vector);
                let det = axis_dir.dot(drag_start.cross(cursor_vector));
                let raw_angle = bevy_math::ops::atan2(det, dot);
                let angle = match settings.snap_rotate {
                    Some(inc) => snap_value(raw_angle, inc),
                    None => raw_angle,
                };
                let rotation_delta = Quat::from_axis_angle(axis_dir, angle);
                transform.rotation = rotation_delta * state.start_transform.rotation;
            }
            TransformGizmoMode::Scale => {
                let plane_normal = translation_plane_normal(ray, axis_dir);
                let Some(intersection) = intersect_plane(ray, plane_normal, gizmo_origin) else {
                    return;
                };
                let axis_norm = axis_dir.normalize();
                let cursor_projected = (intersection - gizmo_origin).dot(axis_norm);
                let start_projected = (state.drag_start_world - gizmo_origin).dot(axis_norm);
                let scale_delta = (cursor_projected - start_projected) * SCALE_SENSITIVITY;

                let mut new_scale = state.start_transform.scale;
                match axis {
                    TransformGizmoAxis::X => {
                        new_scale.x = (new_scale.x + scale_delta).max(MIN_SCALE);
                    }
                    TransformGizmoAxis::Y => {
                        new_scale.y = (new_scale.y + scale_delta).max(MIN_SCALE);
                    }
                    TransformGizmoAxis::Z => {
                        new_scale.z = (new_scale.z + scale_delta).max(MIN_SCALE);
                    }
                }
                transform.scale = match settings.snap_scale {
                    Some(inc) => {
                        let mut snapped = state.start_transform.scale;
                        match axis {
                            TransformGizmoAxis::X => snapped.x = snap_value(new_scale.x, inc),
                            TransformGizmoAxis::Y => snapped.y = snap_value(new_scale.y, inc),
                            TransformGizmoAxis::Z => snapped.z = snap_value(new_scale.z, inc),
                        }
                        snapped
                    }
                    None => new_scale,
                };
            }
        }
        return;
    }

    // End drag — use !pressed instead of just_released for robustness (Alt-Tab, etc.)
    if state.active && !mouse.pressed(MouseButton::Left) {
        state.active = false;
        state.axis = None;
        state.entity = None;
        if settings.confine_cursor {
            cursor_opts.grab_mode = *saved_grab_mode;
        }
    }
}

fn transform_gizmo_draw(
    mut gizmos: Gizmos<TransformGizmoGroup>,
    focus: Option<Single<&GlobalTransform, With<TransformGizmoFocus>>>,
    marked_cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    all_cameras: Query<(&Camera, &GlobalTransform)>,
    settings: Res<TransformGizmoSettings>,
    state: Res<TransformGizmoState>,
) {
    let Some(global_tf) = focus else {
        return;
    };
    let Some((_, cam_tf)) = resolve_gizmo_camera!(marked_cameras, all_cameras) else {
        return;
    };

    let pos = global_tf.translation();
    let effective_space = if settings.mode == TransformGizmoMode::Scale {
        &TransformGizmoSpace::Local
    } else {
        &settings.space
    };
    let rotation = gizmo_rotation(*global_tf, effective_space);

    let scale = if settings.screen_scale_factor > 0.0 {
        (cam_tf.translation() - pos).length() * settings.screen_scale_factor
    } else {
        1.0
    };

    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let forward = rotation * Vec3::Z;

    let active_axis = if state.active {
        state.axis
    } else {
        state.hovered_axis
    };
    let dragging = state.active;

    let x_color = axis_color(TransformGizmoAxis::X, active_axis, dragging);
    let y_color = axis_color(TransformGizmoAxis::Y, active_axis, dragging);
    let z_color = axis_color(TransformGizmoAxis::Z, active_axis, dragging);

    let length = settings.axis_length * scale;

    match settings.mode {
        TransformGizmoMode::Translate => {
            let tip = AXIS_TIP_LENGTH * scale;
            let offset = AXIS_START_OFFSET * scale;
            gizmos
                .arrow(pos + right * offset, pos + right * length, x_color)
                .with_tip_length(tip);
            gizmos
                .arrow(pos + up * offset, pos + up * length, y_color)
                .with_tip_length(tip);
            gizmos
                .arrow(pos + forward * offset, pos + forward * length, z_color)
                .with_tip_length(tip);
        }
        TransformGizmoMode::Rotate => {
            let radius = settings.rotate_ring_radius * scale;
            gizmos.circle(
                Isometry3d::new(pos, Quat::from_rotation_arc(Vec3::Z, right)),
                radius,
                x_color,
            );
            gizmos.circle(
                Isometry3d::new(pos, Quat::from_rotation_arc(Vec3::Z, up)),
                radius,
                y_color,
            );
            gizmos.circle(
                Isometry3d::new(pos, Quat::from_rotation_arc(Vec3::Z, forward)),
                radius,
                z_color,
            );
        }
        TransformGizmoMode::Scale => {
            let cube_half = SCALE_CUBE_SIZE * scale;
            let offset = AXIS_START_OFFSET * scale;
            for (dir, color) in [(right, x_color), (up, y_color), (forward, z_color)] {
                let end = pos + dir * length;
                gizmos.line(pos + dir * offset, end, color);
                // Wireframe cube at endpoint
                let x = Vec3::X * cube_half;
                let y = Vec3::Y * cube_half;
                let z = Vec3::Z * cube_half;
                let corners = [
                    end - x - y - z,
                    end + x - y - z,
                    end + x + y - z,
                    end - x + y - z,
                    end - x - y + z,
                    end + x - y + z,
                    end + x + y + z,
                    end - x + y + z,
                ];
                // Bottom face
                gizmos.line(corners[0], corners[1], color);
                gizmos.line(corners[1], corners[2], color);
                gizmos.line(corners[2], corners[3], color);
                gizmos.line(corners[3], corners[0], color);
                // Top face
                gizmos.line(corners[4], corners[5], color);
                gizmos.line(corners[5], corners[6], color);
                gizmos.line(corners[6], corners[7], color);
                gizmos.line(corners[7], corners[4], color);
                // Verticals
                gizmos.line(corners[0], corners[4], color);
                gizmos.line(corners[1], corners[5], color);
                gizmos.line(corners[2], corners[6], color);
                gizmos.line(corners[3], corners[7], color);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn axis_direction(axis: TransformGizmoAxis, rotation: Quat) -> Vec3 {
    match axis {
        TransformGizmoAxis::X => rotation * Vec3::X,
        TransformGizmoAxis::Y => rotation * Vec3::Y,
        TransformGizmoAxis::Z => rotation * Vec3::Z,
    }
}

/// Construct the constraint plane normal for axis translation/scale.
///
/// The plane contains the drag axis and is oriented to face the camera as much
/// as possible, matching the approach from `bevy_transform_gizmo`.
fn translation_plane_normal(ray: Ray3d, axis: Vec3) -> Vec3 {
    let vertical = Vec3::from(ray.direction).cross(axis);
    if vertical.length_squared() < f32::EPSILON {
        // Ray is nearly parallel to the axis — pick an arbitrary perpendicular.
        return axis.any_orthonormal_vector();
    }
    axis.cross(vertical.normalize()).normalize()
}

/// Intersect a ray with a plane defined by a normal and a point on the plane.
fn intersect_plane(ray: Ray3d, plane_normal: Vec3, plane_origin: Vec3) -> Option<Vec3> {
    let denominator = Vec3::from(ray.direction).dot(plane_normal);
    if denominator.abs() > f32::EPSILON {
        let point_to_point = plane_origin - ray.origin;
        let intersect_dist = plane_normal.dot(point_to_point) / denominator;
        Some(Vec3::from(ray.direction) * intersect_dist + ray.origin)
    } else {
        None
    }
}

/// Distance from a point to a line segment in 2D.
fn point_to_segment_dist(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ap = point - a;
    let t = (ap.dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (point - closest).length()
}

fn point_to_ring_screen_dist(
    cursor: Vec2,
    camera: &Camera,
    cam_tf: &GlobalTransform,
    center: Vec3,
    normal: Vec3,
    radius: f32,
) -> f32 {
    const RING_SAMPLES: usize = 64;
    let rot = Quat::from_rotation_arc(Vec3::Z, normal);
    let mut min_dist = f32::MAX;
    let mut prev_screen = None;

    for i in 0..=RING_SAMPLES {
        let angle = (i % RING_SAMPLES) as f32 * core::f32::consts::TAU / RING_SAMPLES as f32;
        let local = Vec3::new(
            bevy_math::ops::cos(angle) * radius,
            bevy_math::ops::sin(angle) * radius,
            0.0,
        );
        let world = center + rot * local;
        let Some(screen) = camera.world_to_viewport(cam_tf, world).ok() else {
            prev_screen = None;
            continue;
        };
        if let Some(prev) = prev_screen {
            let dist = point_to_segment_dist(cursor, prev, screen);
            if dist < min_dist {
                min_dist = dist;
            }
        }
        prev_screen = Some(screen);
    }

    min_dist
}

fn gizmo_rotation(global_tf: &GlobalTransform, space: &TransformGizmoSpace) -> Quat {
    match space {
        TransformGizmoSpace::World => Quat::IDENTITY,
        TransformGizmoSpace::Local => {
            let (_, rotation, _) = global_tf.to_scale_rotation_translation();
            rotation
        }
    }
}

fn axis_color(
    axis: TransformGizmoAxis,
    active: Option<TransformGizmoAxis>,
    dragging: bool,
) -> Color {
    let is_active = active == Some(axis);
    let (normal, bright) = match axis {
        TransformGizmoAxis::X => (COLOR_X, COLOR_X_BRIGHT),
        TransformGizmoAxis::Y => (COLOR_Y, COLOR_Y_BRIGHT),
        TransformGizmoAxis::Z => (COLOR_Z, COLOR_Z_BRIGHT),
    };

    if is_active {
        bright
    } else if dragging {
        normal.with_alpha(INACTIVE_ALPHA)
    } else {
        normal
    }
}

fn snap_value(value: f32, increment: f32) -> f32 {
    (value / increment).round() * increment
}

/// Snap only the component along the dragged axis, leaving others unchanged.
fn snap_axis(position: Vec3, original: Vec3, axis: TransformGizmoAxis, increment: f32) -> Vec3 {
    match axis {
        TransformGizmoAxis::X => {
            Vec3::new(snap_value(position.x, increment), original.y, original.z)
        }
        TransformGizmoAxis::Y => {
            Vec3::new(original.x, snap_value(position.y, increment), original.z)
        }
        TransformGizmoAxis::Z => {
            Vec3::new(original.x, original.y, snap_value(position.z, increment))
        }
    }
}
