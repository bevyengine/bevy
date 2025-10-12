//! A camera controller for 2D scenes that supports panning and zooming.
//!
//! To use this controller, add [`PanCamPlugin`] to your app,
//! and insert a [`PanCam`] component into your camera entity.
//!
//! To configure the settings of this controller, modify the fields of the [`PanCam`] component.

use bevy_app::{App, Plugin, RunFixedMainLoop, RunFixedMainLoopSystems};
use bevy_camera::Camera;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyCode;
use bevy_input::ButtonInput;
use bevy_math::{Vec2, Vec3};
use bevy_time::{Real, Time};
use bevy_transform::prelude::Transform;

use core::{f32::consts::*, fmt};

/// A pancam-style camera controller plugin.
///
/// Use [`PanCam`] to add a pancam controller to a camera entity,
/// and change its values to customize the controls and change its behavior.
pub struct PanCamPlugin;

impl Plugin for PanCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            RunFixedMainLoop,
            run_pancam_controller.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
    }
}

/// Pancam controller settings and state.
///
/// Add this component to a [`Camera`] entity and add [`PanCamPlugin`]
/// to your [`App`] to enable freecam controls.
#[derive(Component)]
pub struct PanCam {
    /// Enables this [`PanCam`] when `true`.
    pub enable: bool,
    /// Multiplier for how much each zoom input affects the camera.
    pub zoom_factor: f32,
    /// Minimum allowed zoom level.
    pub min_zoom: f32,
    /// Maximum allowed zoom level.
    pub max_zoom: f32,
    /// [`KeyCode`] to zoom in.
    pub key_zoom_in: KeyCode,
    /// [`KeyCode`] to zoom out.
    pub key_zoom_out: KeyCode,
    /// This [`PanCam`]'s translation speed.
    pub pan_speed: f32,
    /// [`KeyCode`] for upward translation.
    pub key_up: KeyCode,
    /// [`KeyCode`] for backward translation.
    pub key_down: KeyCode,
    /// [`KeyCode`] for leftward translation.
    pub key_left: KeyCode,
    /// [`KeyCode`] for rightward translation.
    pub key_right: KeyCode,
    /// Rotation speed multiplier (in radians per second).
    pub rotation_speed: f32,
    /// [`KeyCode`] for counter-clockwise rotation.
    pub key_rotate_ccw: KeyCode,
    /// [`KeyCode`] for clockwise rotation.
    pub key_rotate_cw: KeyCode,
}

impl Default for PanCam {
    fn default() -> Self {
        Self {
            enable: true,
            zoom_factor: 0.1,
            min_zoom: 0.2,
            max_zoom: 5.0,
            key_zoom_in: KeyCode::Equal,
            key_zoom_out: KeyCode::Minus,
            pan_speed: 500.0,
            key_up: KeyCode::KeyW,
            key_down: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            rotation_speed: PI,
            key_rotate_ccw: KeyCode::KeyQ,
            key_rotate_cw: KeyCode::KeyE,
        }
    }
}

impl fmt::Display for PanCam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
PanCam Controls:
    {:?} & {:?}\t- Move up & down
    {:?} & {:?}\t- Move left & right
    {:?}\t- Rotate counter-clockwise
    {:?}\t- Rotate clockwise
    Mouse Scroll\t- Zoom in & out
    {:?} & {:?}\t- Zoom in & out keys",
            self.key_up,
            self.key_down,
            self.key_left,
            self.key_right,
            self.key_rotate_ccw,
            self.key_rotate_cw,
            self.key_zoom_in,
            self.key_zoom_out,
        )
    }
}

/// This system is typically added via the [`PanCamPlugin`].
///
/// Reads inputs and then moves the camera entity according
/// to the settings given in [`PanCam`].
fn run_pancam_controller(
    time: Res<Time<Real>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &PanCam), With<Camera>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, controller)) = query.single_mut() else {
        return;
    };

    if !controller.enable {
        return;
    }

    // === Movement
    let mut movement = Vec2::ZERO;
    if key_input.pressed(controller.key_left) {
        movement.x -= 1.0;
    }
    if key_input.pressed(controller.key_right) {
        movement.x += 1.0;
    }
    if key_input.pressed(controller.key_down) {
        movement.y -= 1.0;
    }
    if key_input.pressed(controller.key_up) {
        movement.y += 1.0;
    }

    // NOTE: Movement is world-axis aligned, not relative to camera rotation
    if movement != Vec2::ZERO {
        let delta = movement.normalize() * controller.pan_speed * dt;
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }

    // === Rotation
    if key_input.pressed(controller.key_rotate_ccw) {
        transform.rotate_z(controller.rotation_speed * dt);
    }
    if key_input.pressed(controller.key_rotate_cw) {
        transform.rotate_z(-controller.rotation_speed * dt);
    }

    // === Zoom
    // TODO: Implement zooming (e.g., adjusting camera scale or projection)
}
