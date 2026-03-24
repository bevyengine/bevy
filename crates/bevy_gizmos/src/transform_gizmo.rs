//! Interactive transform gizmo for translating, rotating, and scaling entities.
//!
//! This module provides an opt-in transform gizmo that renders visual handles on a
//! focused entity, allowing the user to click-and-drag to translate, rotate, or scale
//! it. The plugin does **not** handle keyboard input — users set [`TransformGizmoMode`]
//! as a resource however they like (keyboard shortcuts, UI buttons, gamepad, etc.).
//!
//! # Quick start
//!
//! ```no_run
//! # use bevy_app::App;
//! # use bevy_gizmos::transform_gizmo::{TransformGizmoPlugin, TransformGizmoFocus};
//! // 1. Add the plugin
//! // app.add_plugins(TransformGizmoPlugin);
//!
//! // 2. Mark the entity you want to manipulate
//! // commands.spawn((Mesh3d(mesh), TransformGizmoFocus));
//! ```

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
    system::{Query, Res, ResMut, Single},
};
use bevy_input::{mouse::MouseButton, ButtonInput};
use bevy_math::{Isometry3d, Quat, Vec2, Vec3};
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

const TRANSLATE_SENSITIVITY: f32 = 0.003;
const ROTATE_SENSITIVITY: f32 = 0.01;
const SCALE_SENSITIVITY: f32 = 0.005;
const MIN_SCALE: f32 = 0.01;
const AXIS_HIT_DISTANCE: f32 = 35.0;
const INACTIVE_ALPHA: f32 = 0.15;

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
/// If no camera has this component, the gizmo systems will silently do nothing.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub struct TransformGizmoCamera;

/// Which manipulation mode the gizmo is in.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug, Reflect)]
#[reflect(Resource, Default)]
pub enum TransformGizmoMode {
    /// Move the entity along an axis.
    #[default]
    Translate,
    /// Rotate the entity around an axis.
    Rotate,
    /// Scale the entity along an axis.
    Scale,
}

/// Whether the gizmo operates in world or local space.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug, Reflect)]
#[reflect(Resource, Default)]
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

/// Exposes the current drag state so external systems (undo, UI) can observe it.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct TransformGizmoDragState {
    /// `true` while the user is actively dragging.
    pub active: bool,
    /// The axis being dragged, if any.
    pub axis: Option<TransformGizmoAxis>,
    /// Screen position where the drag started.
    pub drag_start_screen: Vec2,
    /// The transform snapshot taken when the drag started.
    pub start_transform: Transform,
    /// The entity being dragged, if any.
    pub entity: Option<Entity>,
}

/// Exposes which axis the cursor is currently hovering over.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct TransformGizmoHoverState {
    /// The axis under the cursor, if any.
    pub hovered_axis: Option<TransformGizmoAxis>,
}

/// Optional configuration to customize [`TransformGizmo`](TransformGizmoPlugin) parameters.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct TransformGizmoConfig {
    /// Length of the axis handles.
    pub axis_length: f32,
    /// Radius of the rotation rings.
    pub rotate_ring_radius: f32,
    /// Translation sensitivity (world-units per pixel per unit-distance).
    pub translate_sensitivity: f32,
    /// Rotation sensitivity (radians per pixel).
    pub rotate_sensitivity: f32,
    /// Scale sensitivity (scale-units per pixel).
    pub scale_sensitivity: f32,
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

impl Default for TransformGizmoConfig {
    fn default() -> Self {
        Self {
            axis_length: AXIS_LENGTH,
            rotate_ring_radius: ROTATE_RING_RADIUS,
            translate_sensitivity: TRANSLATE_SENSITIVITY,
            rotate_sensitivity: ROTATE_SENSITIVITY,
            scale_sensitivity: SCALE_SENSITIVITY,
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

/// System set for the transform gizmo. All transform gizmo systems run in [`Update`] within this set.
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
        app.init_resource::<TransformGizmoMode>()
            .init_resource::<TransformGizmoSpace>()
            .init_resource::<TransformGizmoDragState>()
            .init_resource::<TransformGizmoHoverState>()
            .init_resource::<TransformGizmoConfig>()
            .init_gizmo_group::<TransformGizmoGroup>()
            .register_type::<TransformGizmoFocus>()
            .register_type::<TransformGizmoMode>()
            .register_type::<TransformGizmoSpace>()
            .register_type::<TransformGizmoDragState>()
            .register_type::<TransformGizmoHoverState>()
            .register_type::<TransformGizmoConfig>()
            .register_type::<TransformGizmoCamera>()
            .add_systems(Startup, configure_transform_gizmo_group)
            .add_systems(
                Update,
                (
                    sync_transform_gizmo_config,
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
    config: Res<TransformGizmoConfig>,
) {
    let (gizmo_config, _) = config_store.config_mut::<TransformGizmoGroup>();
    gizmo_config.depth_bias = -1.0;
    gizmo_config.line.width = config.line_width;
}

fn sync_transform_gizmo_config(
    mut config_store: ResMut<GizmoConfigStore>,
    config: Res<TransformGizmoConfig>,
) {
    if config.is_changed() {
        let (gizmo_config, _) = config_store.config_mut::<TransformGizmoGroup>();
        gizmo_config.line.width = config.line_width;
    }
}

fn transform_gizmo_hover(
    focus: Option<Single<&GlobalTransform, With<TransformGizmoFocus>>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mode: Res<TransformGizmoMode>,
    space: Res<TransformGizmoSpace>,
    config: Res<TransformGizmoConfig>,
    mut hover: ResMut<TransformGizmoHoverState>,
    drag_state: Res<TransformGizmoDragState>,
) {
    hover.hovered_axis = None;

    if drag_state.active {
        return;
    }

    let Some(global_tf) = focus else {
        return;
    };
    let Some(camera) = camera else {
        return;
    };
    let (camera, cam_tf) = *camera;
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let gizmo_pos = global_tf.translation();
    let effective_space = if *mode == TransformGizmoMode::Scale {
        &TransformGizmoSpace::Local
    } else {
        &space
    };
    let rotation = gizmo_rotation(*global_tf, effective_space);

    let scale = if config.screen_scale_factor > 0.0 {
        (cam_tf.translation() - gizmo_pos).length() * config.screen_scale_factor
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
    let threshold = config.axis_hit_distance;

    for (axis, dir) in &axes {
        let dist = match *mode {
            TransformGizmoMode::Translate | TransformGizmoMode::Scale => {
                let start = gizmo_pos + *dir * (AXIS_START_OFFSET * scale);
                let endpoint = gizmo_pos + *dir * (config.axis_length * scale);
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
                config.rotate_ring_radius * scale,
            ),
        };
        if dist < threshold && dist < best_dist {
            best_dist = dist;
            best_axis = Some(*axis);
        }
    }

    hover.hovered_axis = best_axis;
}

fn transform_gizmo_drag(
    mut focus_query: Query<(Entity, &GlobalTransform, &mut Transform), With<TransformGizmoFocus>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>>,
    primary_window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mode: Res<TransformGizmoMode>,
    space: Res<TransformGizmoSpace>,
    config: Res<TransformGizmoConfig>,
    hover: Res<TransformGizmoHoverState>,
    mut drag_state: ResMut<TransformGizmoDragState>,
) {
    let Some(camera) = camera else {
        return;
    };
    let (camera, cam_tf) = *camera;
    let (window, mut cursor_opts) = primary_window.into_inner();
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Start drag
    if mouse.just_pressed(MouseButton::Left) && !drag_state.active {
        if let Some(axis) = hover.hovered_axis
            && let Some((entity, _, transform)) = focus_query.iter().next()
        {
            drag_state.active = true;
            drag_state.axis = Some(axis);
            drag_state.drag_start_screen = cursor_pos;
            drag_state.start_transform = *transform;
            drag_state.entity = Some(entity);
            if config.confine_cursor {
                cursor_opts.grab_mode = CursorGrabMode::Confined;
            }
        }
        return;
    }

    // Continue drag
    if drag_state.active && mouse.pressed(MouseButton::Left) {
        let Some(drag_entity) = drag_state.entity else {
            return;
        };
        let Some(axis) = drag_state.axis else {
            return;
        };
        let Ok((_, global_tf, mut transform)) = focus_query.get_mut(drag_entity) else {
            return;
        };

        let effective_space = if *mode == TransformGizmoMode::Scale {
            &TransformGizmoSpace::Local
        } else {
            &space
        };
        let rotation = gizmo_rotation(global_tf, effective_space);
        let axis_dir = match axis {
            TransformGizmoAxis::X => rotation * Vec3::X,
            TransformGizmoAxis::Y => rotation * Vec3::Y,
            TransformGizmoAxis::Z => rotation * Vec3::Z,
        };

        let gizmo_pos = global_tf.translation();

        match *mode {
            TransformGizmoMode::Translate => {
                let Some(origin_screen) = camera.world_to_viewport(cam_tf, gizmo_pos).ok() else {
                    return;
                };
                let Some(axis_screen) = camera.world_to_viewport(cam_tf, gizmo_pos + axis_dir).ok()
                else {
                    return;
                };
                let screen_axis = (axis_screen - origin_screen).normalize_or_zero();
                let mouse_delta = cursor_pos - drag_state.drag_start_screen;
                let projected = mouse_delta.dot(screen_axis);

                let cam_dist = (cam_tf.translation() - gizmo_pos).length();
                let scale = cam_dist * config.translate_sensitivity;

                let raw_delta = axis_dir * projected * scale;
                let snapped_delta = match config.snap_translate {
                    Some(inc) => snap_vec3(raw_delta, inc),
                    None => raw_delta,
                };
                transform.translation = drag_state.start_transform.translation + snapped_delta;
            }
            TransformGizmoMode::Rotate => {
                let mouse_delta = cursor_pos - drag_state.drag_start_screen;
                let screen_axis = match axis {
                    TransformGizmoAxis::X => Vec2::Y,
                    TransformGizmoAxis::Y => Vec2::X,
                    TransformGizmoAxis::Z => -Vec2::X,
                };
                let raw_angle = mouse_delta.dot(screen_axis) * config.rotate_sensitivity;
                let angle = match config.snap_rotate {
                    Some(inc) => snap_value(raw_angle, inc),
                    None => raw_angle,
                };
                let rotation_delta = Quat::from_axis_angle(axis_dir, angle);
                transform.rotation = rotation_delta * drag_state.start_transform.rotation;
            }
            TransformGizmoMode::Scale => {
                let Some(origin_screen) = camera.world_to_viewport(cam_tf, gizmo_pos).ok() else {
                    return;
                };
                let Some(axis_screen) = camera.world_to_viewport(cam_tf, gizmo_pos + axis_dir).ok()
                else {
                    return;
                };
                let screen_axis = (axis_screen - origin_screen).normalize_or_zero();
                let mouse_delta = cursor_pos - drag_state.drag_start_screen;
                let projected = mouse_delta.dot(screen_axis) * config.scale_sensitivity;

                let mut new_scale = drag_state.start_transform.scale;
                match axis {
                    TransformGizmoAxis::X => {
                        new_scale.x = (new_scale.x + projected).max(MIN_SCALE);
                    }
                    TransformGizmoAxis::Y => {
                        new_scale.y = (new_scale.y + projected).max(MIN_SCALE);
                    }
                    TransformGizmoAxis::Z => {
                        new_scale.z = (new_scale.z + projected).max(MIN_SCALE);
                    }
                }
                transform.scale = match config.snap_scale {
                    Some(inc) => snap_vec3(new_scale, inc),
                    None => new_scale,
                };
            }
        }
        return;
    }

    // End drag
    if drag_state.active && mouse.just_released(MouseButton::Left) {
        drag_state.active = false;
        drag_state.axis = None;
        drag_state.entity = None;
        if config.confine_cursor {
            cursor_opts.grab_mode = CursorGrabMode::None;
        }
    }
}

fn transform_gizmo_draw(
    mut gizmos: Gizmos<TransformGizmoGroup>,
    focus: Option<Single<&GlobalTransform, With<TransformGizmoFocus>>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>>,
    mode: Res<TransformGizmoMode>,
    space: Res<TransformGizmoSpace>,
    config: Res<TransformGizmoConfig>,
    hover: Res<TransformGizmoHoverState>,
    drag_state: Res<TransformGizmoDragState>,
) {
    let Some(global_tf) = focus else {
        return;
    };
    let Some(camera) = camera else {
        return;
    };

    let pos = global_tf.translation();
    let effective_space = if *mode == TransformGizmoMode::Scale {
        &TransformGizmoSpace::Local
    } else {
        &space
    };
    let rotation = gizmo_rotation(*global_tf, effective_space);

    let scale = if config.screen_scale_factor > 0.0 {
        let (_, cam_tf) = *camera;
        (cam_tf.translation() - pos).length() * config.screen_scale_factor
    } else {
        1.0
    };

    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let forward = rotation * Vec3::Z;

    let active_axis = if drag_state.active {
        drag_state.axis
    } else {
        hover.hovered_axis
    };
    let dragging = drag_state.active;

    let x_color = axis_color(TransformGizmoAxis::X, active_axis, dragging);
    let y_color = axis_color(TransformGizmoAxis::Y, active_axis, dragging);
    let z_color = axis_color(TransformGizmoAxis::Z, active_axis, dragging);

    let length = config.axis_length * scale;

    match *mode {
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
            let radius = config.rotate_ring_radius * scale;
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
    const RING_SAMPLES: usize = 16;
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

fn snap_vec3(v: Vec3, increment: f32) -> Vec3 {
    Vec3::new(
        snap_value(v.x, increment),
        snap_value(v.y, increment),
        snap_value(v.z, increment),
    )
}
