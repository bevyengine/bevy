//! A camera controller that snaps camera to different axes.
//!
//! This controller is intended as addition to Free Camera controller and adds Blender-like hotkeys to snap camera orientation to global axes. 
//! It is useful for detailed scene look.
//!
//! To use this controller, add [`SnapToViewPlugin`] to your app,
//! and attach the [`SnapToViewCamera`] component to your camera entity.
//! The required [`SnapToViewCameraState`] component will be added automatically.
//!
//! To configure the settings of this controller, modify the fields of the [`SnapToViewCamera`] component.
//! 
//! TODO: Add switching camera to orthographic mode.

use core::{f32, fmt};

use crate::free_camera::FreeCameraState;
use bevy_app::{App, Plugin, RunFixedMainLoop, RunFixedMainLoopSystems};
use bevy_camera::Camera;
use bevy_ecs::{
    component::Component,
    query::With,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
    world::Mut,
};
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_log::info;
use bevy_math::{Dir3, EulerRot};
use bevy_transform::components::Transform;

/// A camera controller plugin for snapping camera to axes on hotkey presses.
///
/// Use the [`SnapToViewCamera`] struct to add and customize the controller for a camera entity.
/// The camera's dynamic state is managed by the [`SnapToViewCameraState`] struct.
pub struct SnapToViewPlugin;

impl Plugin for SnapToViewPlugin {
    fn build(&self, app: &mut App) {
        // This ordering is required so that both fixed update and update systems can see the results correctly
        app.add_systems(
            RunFixedMainLoop,
            (run_snap_to_view_controller, rotate_camera_to)
                .chain()
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
    }
}

/// Stores the settings for the [`SnapToViewCamera`] controller.
///
/// This component defines static configuration for camera controls,
/// including movement speed, sensitivity, and input bindings.
///
/// From the controller’s perspective, this data is treated as immutable,
/// but it may be modified externally (e.g., by a settings UI) at runtime.
///
/// Add this component to a [`Camera`] entity to enable `SnapToView` controls.
/// The associated dynamic state is automatically handled by [`SnapToViewCameraState`],
/// which is added to the entity as a required component.
#[derive(Component)]
#[require(SnapToViewCameraState)]
pub struct SnapToViewCamera {
    /// Modifier [`KeyCode`] for making pressed axis alignment buttons go in opposite direction
    pub mod_key: KeyCode,
    /// [`KeyCode`] for snapping camera to top/bottom (+Y/-Y).
    pub axis_top: KeyCode,
    /// [`KeyCode`] for snapping camera to right/left (+X/-X).
    pub axis_right: KeyCode,
    /// [`KeyCode`] for snapping camera to front/back (-Z/+Z).
    pub axis_front: KeyCode,
    /// Speed of camera rotation to snapped axis in radians.
    pub rotation_speed: f32,
}

impl Default for SnapToViewCamera {
    fn default() -> Self {
        Self {
            mod_key: KeyCode::ControlLeft,
            axis_top: KeyCode::Numpad7,
            axis_right: KeyCode::Numpad3,
            axis_front: KeyCode::Numpad1,
            rotation_speed: f32::consts::PI / 16.0,
        }
    }
}

impl fmt::Display for SnapToViewCamera {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Snap camera to axis controls:
    [{:?} + ]{:?}\t- Snap to Up (+Y)/Down (-Y)
    [{:?} + ]{:?}\t- Snap to Right (+X)/Left (-X)
    [{:?} + ]{:?}\t- Snap to Front (-Z)/Back (+Z)
",
            self.mod_key,
            self.axis_top,
            self.mod_key,
            self.axis_right,
            self.mod_key,
            self.axis_front,
        )
    }
}

/// Tracks the runtime state of a [`SnapToViewCamera`] controller.
///
/// This component holds dynamic data that changes during camera operation,
/// such as direction for camera rotation and whether the controller is currently enabled.
///
/// It is automatically added to any entity that has a [`SnapToViewCamera`] component,
/// and is updated by the [`SnapToViewPlugin`] systems in response to user input.
#[derive(Component)]
pub struct SnapToViewCameraState {
    /// Enables [`FreeCamera`] controls when `true`.
    pub enabled: bool,
    /// Internal flag indicating if this controller has been initialized by the [`SnapToViewPlugin`].
    initialized: bool,
    /// Direction to which camera will snap at speed, specified in [`SnapToViewCamera`] by [`SnapToViewCamera::rotation_speed`] field.
    /// Consist of forward direction vector and up vector.
    pub rotate_to: Option<(Dir3, Dir3)>,
}

impl Default for SnapToViewCameraState {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            rotate_to: None,
        }
    }
}

/// Updates the internal state of camera based on user input.
/// Change of camera orientation is performed by [`rotate_camera_to`] system.
///
/// - [`SnapToViewCamera`] contains static configuration such as key bindings and rotation speed.
/// - [`SnapToViewCameraState`] stores the dynamic runtime state, including direction for camera rotation and enable flags.
///
/// This system is typically added via the [`SnapToViewPlugin`].

pub fn run_snap_to_view_controller(
    key_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut SnapToViewCameraState, &SnapToViewCamera), With<Camera>>,
) {
    let Ok((mut state, config)) = query.single_mut() else {
        return;
    };
    if !state.initialized {
        state.initialized = true;
        info!("{}", *config);
    }
    if !state.enabled {
        return;
    }
    let mod_key_pressed = key_input.pressed(config.mod_key);
    if key_input.pressed(config.axis_front) {
        if mod_key_pressed {
            state.rotate_to = Some((Dir3::Z, Dir3::Y));
        } else {
            state.rotate_to = Some((Dir3::NEG_Z, Dir3::Y));
        }
    }
    if key_input.pressed(config.axis_right) {
        if mod_key_pressed {
            state.rotate_to = Some((Dir3::NEG_X, Dir3::Y));
        } else {
            state.rotate_to = Some((Dir3::X, Dir3::Y));
        }
    }
    if key_input.pressed(config.axis_top) {
        if mod_key_pressed {
            state.rotate_to = Some((Dir3::NEG_Y, Dir3::NEG_Z));
        } else {
            state.rotate_to = Some((Dir3::Y, Dir3::Z));
        }
    }
}

/// Smoothly changes orientation of camera according to target orientation in [`SnapToViewCameraState`].
/// If [`FreeCameraState`] is also attached to camera, fixes internal state of camera rotation in it to avoid unexpected snaps.
///
/// - [`SnapToViewCamera`] contains static configuration such as key bindings and rotation speed.
/// - [`SnapToViewCameraState`] stores the dynamic runtime state, including direction for camera rotation and enable flags.
///
/// This system is typically added via the [`SnapToViewPlugin`].

pub fn rotate_camera_to(
    mut query: Query<
        (
            &mut Transform,
            &mut SnapToViewCameraState,
            &SnapToViewCamera,
            Option<Mut<FreeCameraState>>,
        ),
        With<Camera>,
    >,
) {
    let Ok((mut transform, mut state, config, freecam_state)) = query.single_mut() else {
        return;
    };
    let Some((to, up)) = state.rotate_to else {
        return;
    };
    let target = Transform::default().looking_to(to, up).rotation;
    transform.rotation = transform
        .rotation
        .rotate_towards(target, config.rotation_speed);
    if let Some(mut freecam_state) = freecam_state {
        let (_z, yaw, pitch) = transform.rotation.to_euler(EulerRot::ZYX);
        freecam_state.pitch = pitch;
        freecam_state.yaw = yaw;
    }
    if transform.rotation == target {
        state.rotate_to = None
    }
}
