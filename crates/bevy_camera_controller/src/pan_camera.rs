//! A controller for 2D cameras that supports panning, zooming, and rotation.
//!
//! To use this controller, add [`PanCameraPlugin`] to your app,
//! and insert a [`PanCamera`] component into your camera entity.
//!
//! To configure the settings of this controller, modify the fields of the [`PanCamera`] component.

use bevy_app::{App, Plugin, RunFixedMainLoop, RunFixedMainLoopSystems};
use bevy_camera::Camera;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy_input::ButtonInput;
use bevy_math::{Vec2, Vec3};
use bevy_time::{Real, Time};
use bevy_transform::prelude::Transform;

use core::{f32::consts::*, fmt};

/// A plugin that enables 2D camera panning and zooming controls.
///
/// Add this plugin to your [`App`] to enable [`PanCamera`] behavior
/// on any camera entity that has the [`PanCamera`] component.
pub struct PanCameraPlugin;

impl Plugin for PanCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            RunFixedMainLoop,
            run_pancamera_controller.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
    }
}

/// Configuration and state for a 2D panning camera controller.
///
/// Add this component to a [`Camera`] entity to enable keyboard and mouse controls
/// for panning, zooming, and optional rotation. Requires the [`PanCameraPlugin`].
#[derive(Component)]
pub struct PanCamera {
    /// Enables this [`PanCamera`] when `true`.
    pub enabled: bool,
    /// Current zoom level (factor applied to camera scale).
    pub zoom_factor: f32,
    /// Minimum allowed zoom level.
    pub min_zoom: f32,
    /// Maximum allowed zoom level.
    pub max_zoom: f32,
    /// Translation speed for panning movement.
    pub zoom_speed: f32,
    /// [`KeyCode`] to zoom in.
    pub key_zoom_in: Option<KeyCode>,
    /// [`KeyCode`] to zoom out.
    pub key_zoom_out: Option<KeyCode>,
    /// This [`PanCamera`]'s translation speed.
    pub pan_speed: f32,
    /// [`KeyCode`] for upward translation.
    pub key_up: Option<KeyCode>,
    /// [`KeyCode`] for downward translation.
    pub key_down: Option<KeyCode>,
    /// [`KeyCode`] for leftward translation.
    pub key_left: Option<KeyCode>,
    /// [`KeyCode`] for rightward translation.
    pub key_right: Option<KeyCode>,
    /// Rotation speed multiplier (in radians per second).
    pub rotation_speed: f32,
    /// [`KeyCode`] for counter-clockwise rotation.
    pub key_rotate_ccw: Option<KeyCode>,
    /// [`KeyCode`] for clockwise rotation.
    pub key_rotate_cw: Option<KeyCode>,
}

/// Provides the default values for the `PanCamera` controller.
///
/// The default settings are:
/// - Zoom factor: 1.0
/// - Min zoom: 0.1
/// - Max zoom: 5.0
/// - Zoom speed: 0.1
/// - Zoom in/out key: +/-
/// - Pan speed: 500.0
/// - Move up/down: W/S
/// - Move left/right: A/D
/// - Rotation speed: PI (radians per second)
/// - Rotation ccw/cw: Q/E
impl Default for PanCamera {
    /// Provides the default values for the `PanCamera` controller.
    ///
    /// Users can override these values by manually creating a `PanCamera` instance
    /// or modifying the default instance.
    fn default() -> Self {
        Self {
            enabled: true,
            zoom_factor: 1.0,
            min_zoom: 0.1,
            max_zoom: 5.0,
            zoom_speed: 0.1,
            key_zoom_in: Some(KeyCode::Equal),
            key_zoom_out: Some(KeyCode::Minus),
            pan_speed: 500.0,
            key_up: Some(KeyCode::KeyW),
            key_down: Some(KeyCode::KeyS),
            key_left: Some(KeyCode::KeyA),
            key_right: Some(KeyCode::KeyD),
            rotation_speed: PI,
            key_rotate_ccw: Some(KeyCode::KeyQ),
            key_rotate_cw: Some(KeyCode::KeyE),
        }
    }
}

impl PanCamera {
    fn key_to_string(key: &Option<KeyCode>) -> String {
        key.map_or("None".to_string(), |k| format!("{:?}", k))
    }
}

impl fmt::Display for PanCamera {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
PanCamera Controls:
  Move Up / Down    - {} / {}
  Move Left / Right - {} / {}
  Rotate CCW / CW   - {} / {}
  Zoom              - Mouse Scroll + {} / {}
",
            Self::key_to_string(&self.key_up),
            Self::key_to_string(&self.key_down),
            Self::key_to_string(&self.key_left),
            Self::key_to_string(&self.key_right),
            Self::key_to_string(&self.key_rotate_ccw),
            Self::key_to_string(&self.key_rotate_cw),
            Self::key_to_string(&self.key_zoom_in),
            Self::key_to_string(&self.key_zoom_out),
        )
    }
}

/// This system is typically added via the [`PanCameraPlugin`].
///
/// Reads inputs and then moves the camera entity according
/// to the settings given in [`PanCamera`].
///
/// **Note**: The zoom applied in this controller is linear. The zoom factor is directly adjusted
/// based on the input (either from the mouse scroll or keyboard).
fn run_pancamera_controller(
    time: Res<Time<Real>>,
    key_input: Res<ButtonInput<KeyCode>>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut Transform, &mut PanCamera), With<Camera>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, mut controller)) = query.single_mut() else {
        return;
    };

    if !controller.enabled {
        return;
    }

    // === Movement
    let mut movement = Vec2::ZERO;
    if let Some(key) = controller.key_left {
        if key_input.pressed(key) {
            movement.x -= 1.0;
        }
    }
    if let Some(key) = controller.key_right {
        if key_input.pressed(key) {
            movement.x += 1.0;
        }
    }
    if let Some(key) = controller.key_down {
        if key_input.pressed(key) {
            movement.y -= 1.0;
        }
    }
    if let Some(key) = controller.key_up {
        if key_input.pressed(key) {
            movement.y += 1.0;
        }
    }

    if movement != Vec2::ZERO {
        let right = transform.right();
        let up = transform.up();

        let delta = (right * movement.x + up * movement.y).normalize() * controller.pan_speed * dt;

        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }

    // === Rotation
    if let Some(key) = controller.key_rotate_ccw {
        if key_input.pressed(key) {
            transform.rotate_z(controller.rotation_speed * dt);
        }
    }
    if let Some(key) = controller.key_rotate_cw {
        if key_input.pressed(key) {
            transform.rotate_z(-controller.rotation_speed * dt);
        }
    }

    // === Zoom
    let mut zoom_amount = 0.0;

    // (with keys)
    if let Some(key) = controller.key_zoom_in {
        if key_input.pressed(key) {
            zoom_amount -= controller.zoom_speed;
        }
    }
    if let Some(key) = controller.key_zoom_out {
        if key_input.pressed(key) {
            zoom_amount += controller.zoom_speed;
        }
    }

    // (with mouse wheel)
    let mouse_scroll = match accumulated_mouse_scroll.unit {
        MouseScrollUnit::Line => accumulated_mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => {
            accumulated_mouse_scroll.delta.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR
        }
    };
    zoom_amount += mouse_scroll * controller.zoom_speed;

    controller.zoom_factor =
        (controller.zoom_factor - zoom_amount).clamp(controller.min_zoom, controller.max_zoom);

    transform.scale = Vec3::splat(controller.zoom_factor);
}
