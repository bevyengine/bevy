//! A camera controller that allows the user to move freely around the scene.
//!
//! Free cameras are helpful for exploring large scenes, level editors and for debugging.
//! They are rarely useful as-is for gameplay,
//! as they allow the user to move freely in all directions,
//! which can be disorienting, and they can clip through objects and terrain.
//!
//! You may have heard of a "fly camera" — a type of free camera designed for fluid "flying" movement and quickly surveying large areas.
//! By contrast, the default settings of this particular free camera are optimized for precise control.
//!
//! To use this controller, add [`FreeCameraPlugin`] to your app,
//! and attach the [`FreeCamera`] component to your camera entity.
//! The required [`FreeCameraState`] component will be added automatically.
//!
//! To configure the settings of this controller, modify the fields of the [`FreeCamera`] component.

use bevy_app::{App, Plugin, RunFixedMainLoop, RunFixedMainLoopSystems};
use bevy_camera::Camera;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::{
    AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseScrollUnit,
};
use bevy_input::ButtonInput;
use bevy_log::info;
use bevy_math::{EulerRot, Quat, StableInterpolate, Vec2, Vec3};
use bevy_time::{Real, Time};
use bevy_transform::prelude::Transform;
use bevy_window::{CursorGrabMode, CursorOptions, Window};

use core::{cmp::PartialEq, f32::consts::*, fmt};

/// A freecam-style camera controller plugin.
///
/// Use the [`FreeCamera`] struct to add and customize the controller for a camera entity.
/// The camera's dynamic state is managed by the [`FreeCameraState`] struct.
pub struct FreeCameraPlugin;

impl Plugin for FreeCameraPlugin {
    fn build(&self, app: &mut App) {
        // This ordering is required so that both fixed update and update systems can see the results correctly
        app.add_systems(
            RunFixedMainLoop,
            run_freecamera_controller.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
    }
}

/// Scales mouse motion into yaw/pitch movement.
///
/// Based on Valorant's default sensitivity, not entirely sure why it is exactly 1.0 / 180.0,
/// but we're guessing it is a misunderstanding between degrees/radians and then sticking with
/// it because it felt nice.
const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

/// Defines the coordinate system used for camera movement.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum RotationSystem {
    /// Movement is relative to the camera's full orientation (Yaw, Pitch, and Roll).
    /// For example, moving "forward" while looking up will move the camera upwards.
    YawPitchRoll,
    /// Movement is constrained to the horizontal plane (Yaw only).
    /// Moving "forward" will keep the camera at the same elevation regardless of pitch.
    Yaw,
}

/// Stores the settings for the [`FreeCamera`] controller.
///
/// This component defines static configuration for camera controls,
/// including movement speed, sensitivity, and input bindings.
///
/// From the controller’s perspective, this data is treated as immutable,
/// but it may be modified externally (e.g., by a settings UI) at runtime.
///
/// Add this component to a [`Camera`] entity to enable `FreeCamera` controls.
/// The associated dynamic state is automatically handled by [`FreeCameraState`],
/// which is added to the entity as a required component.
///
/// To activate the controller, add the [`FreeCameraPlugin`] to your [`App`].
#[derive(Component)]
#[require(FreeCameraState)]
pub struct FreeCamera {
    /// Multiplier for pitch and yaw rotation speed.
    pub sensitivity: f32,
    /// [`KeyCode`] for forward translation.
    pub key_forward: KeyCode,
    /// [`KeyCode`] for backward translation.
    pub key_back: KeyCode,
    /// [`KeyCode`] for left translation.
    pub key_left: KeyCode,
    /// [`KeyCode`] for right translation.
    pub key_right: KeyCode,
    /// [`KeyCode`] for up translation.
    pub key_up: KeyCode,
    /// [`KeyCode`] for down translation.
    pub key_down: KeyCode,
    /// [`KeyCode`] to use [`run_speed`](FreeCamera::run_speed) instead of
    /// [`walk_speed`](FreeCamera::walk_speed) for translation.
    pub key_run: KeyCode,
    /// [`MouseButton`] for grabbing the mouse focus.
    pub mouse_key_cursor_grab: MouseButton,
    /// [`KeyCode`] for grabbing the keyboard focus.
    pub keyboard_key_toggle_cursor_grab: KeyCode,
    /// Base multiplier for unmodified translation speed.
    pub walk_speed: f32,
    /// Base multiplier for running translation speed.
    pub run_speed: f32,
    /// Multiplier for how the mouse scroll wheel modifies [`walk_speed`](FreeCamera::walk_speed)
    /// and [`run_speed`](FreeCamera::run_speed).
    pub scroll_factor: f32,
    /// Friction factor used to exponentially decay [`velocity`](FreeCameraState::velocity) over time.
    pub friction: f32,
    /// The strategy used to calculate the movement direction relative to the camera's orientation.
    ///
    /// Defaults to [`RotationSystem::YawPitchRoll`].
    pub rotation_system: RotationSystem,
}

impl Default for FreeCamera {
    fn default() -> Self {
        Self {
            sensitivity: 0.2,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyE,
            key_down: KeyCode::KeyQ,
            key_run: KeyCode::ShiftLeft,
            mouse_key_cursor_grab: MouseButton::Left,
            keyboard_key_toggle_cursor_grab: KeyCode::KeyM,
            walk_speed: 5.0,
            run_speed: 15.0,
            scroll_factor: 0.5,
            friction: 40.0,
            rotation_system: RotationSystem::YawPitchRoll,
        }
    }
}

impl fmt::Display for FreeCamera {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Freecamera Controls:
    Mouse\t- Move camera orientation
    Scroll\t- Adjust movement speed
    {:?}\t- Hold to grab cursor
    {:?}\t- Toggle cursor grab
    {:?} & {:?}\t- Fly forward & backwards
    {:?} & {:?}\t- Fly sideways left & right
    {:?} & {:?}\t- Fly up & down
    {:?}\t- Fly faster while held",
            self.mouse_key_cursor_grab,
            self.keyboard_key_toggle_cursor_grab,
            self.key_forward,
            self.key_back,
            self.key_left,
            self.key_right,
            self.key_up,
            self.key_down,
            self.key_run,
        )
    }
}

/// Tracks the runtime state of a [`FreeCamera`] controller.
///
/// This component holds dynamic data that changes during camera operation,
/// such as pitch, yaw, velocity, and whether the controller is currently enabled.
///
/// It is automatically added to any entity that has a [`FreeCamera`] component,
/// and is updated by the [`FreeCameraPlugin`] systems in response to user input.
#[derive(Component)]
pub struct FreeCameraState {
    /// Enables [`FreeCamera`] controls when `true`.
    pub enabled: bool,
    /// Internal flag indicating if this controller has been initialized by the [`FreeCameraPlugin`].
    initialized: bool,
    /// This [`FreeCamera`]'s pitch rotation.
    pub pitch: f32,
    /// This [`FreeCamera`]'s yaw rotation.
    pub yaw: f32,
    /// Multiplier applied to movement speed.
    pub speed_multiplier: f32,
    /// This [`FreeCamera`]'s translation velocity.
    pub velocity: Vec3,
}

impl Default for FreeCameraState {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            pitch: 0.0,
            yaw: 0.0,
            speed_multiplier: 1.0,
            velocity: Vec3::ZERO,
        }
    }
}

/// Updates the camera's position and orientation based on user input.
///
/// - [`FreeCamera`] contains static configuration such as key bindings, movement speed, and sensitivity.
/// - [`FreeCameraState`] stores the dynamic runtime state, including pitch, yaw, velocity, and enable flags.
///
/// This system is typically added via the [`FreeCameraPlugin`].
pub fn run_freecamera_controller(
    time: Res<Time<Real>>,
    mut windows: Query<(&Window, &mut CursorOptions)>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut toggle_cursor_grab: Local<bool>,
    mut mouse_cursor_grab: Local<bool>,
    mut query: Query<(&mut Transform, &mut FreeCameraState, &FreeCamera), With<Camera>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, mut state, config)) = query.single_mut() else {
        return;
    };

    if !state.initialized {
        let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
        state.yaw = yaw;
        state.pitch = pitch;
        state.initialized = true;
        info!("{}", *config);
    }

    if !state.enabled {
        // don't keep the cursor grabbed if the camera controller was disabled.
        if *toggle_cursor_grab || *mouse_cursor_grab {
            *toggle_cursor_grab = false;
            *mouse_cursor_grab = false;

            for (_, mut cursor_options) in &mut windows {
                cursor_options.grab_mode = CursorGrabMode::None;
                cursor_options.visible = true;
            }
        }
        return;
    }

    let mut scroll = 0.0;

    let amount = match accumulated_mouse_scroll.unit {
        MouseScrollUnit::Line => accumulated_mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => {
            accumulated_mouse_scroll.delta.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR
        }
    };
    scroll += amount;
    state.speed_multiplier += scroll * config.scroll_factor;
    // Clamp the speed multiplier for safety
    state.speed_multiplier = state.speed_multiplier.clamp(0.0, f32::MAX);

    // Handle key input
    let mut axis_input = Vec3::ZERO;
    if key_input.pressed(config.key_forward) {
        axis_input.z -= 1.0;
    }
    if key_input.pressed(config.key_back) {
        axis_input.z += 1.0;
    }
    if key_input.pressed(config.key_right) {
        axis_input.x += 1.0;
    }
    if key_input.pressed(config.key_left) {
        axis_input.x -= 1.0;
    }
    if key_input.pressed(config.key_up) {
        axis_input.y += 1.0;
    }
    if key_input.pressed(config.key_down) {
        axis_input.y -= 1.0;
    }

    let mut cursor_grab_change = false;
    if key_input.just_pressed(config.keyboard_key_toggle_cursor_grab) {
        *toggle_cursor_grab = !*toggle_cursor_grab;
        cursor_grab_change = true;
    }
    if mouse_button_input.just_pressed(config.mouse_key_cursor_grab) {
        *mouse_cursor_grab = true;
        cursor_grab_change = true;
    }
    if mouse_button_input.just_released(config.mouse_key_cursor_grab) {
        *mouse_cursor_grab = false;
        cursor_grab_change = true;
    }
    let cursor_grab = *mouse_cursor_grab || *toggle_cursor_grab;

    // Update velocity
    if axis_input != Vec3::ZERO {
        let max_speed = if key_input.pressed(config.key_run) {
            config.run_speed * state.speed_multiplier
        } else {
            config.walk_speed * state.speed_multiplier
        };
        state.velocity = axis_input.normalize() * max_speed;
    } else {
        let friction = config.friction.clamp(0.0, f32::MAX);
        state.velocity.smooth_nudge(&Vec3::ZERO, friction, dt);
        if state.velocity.length_squared() < 1e-6 {
            state.velocity = Vec3::ZERO;
        }
    }

    // Apply movement update
    if state.velocity != Vec3::ZERO {
        let rotation = match config.rotation_system {
            RotationSystem::YawPitchRoll => transform.rotation,
            RotationSystem::Yaw => {
                let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                Quat::from_rotation_y(yaw)
            }
        };

        transform.translation += rotation * state.velocity * dt;
    }

    // Handle cursor grab
    if cursor_grab_change {
        if cursor_grab {
            for (window, mut cursor_options) in &mut windows {
                if !window.focused {
                    continue;
                }

                cursor_options.grab_mode = CursorGrabMode::Locked;
                cursor_options.visible = false;
            }
        } else {
            for (_, mut cursor_options) in &mut windows {
                cursor_options.grab_mode = CursorGrabMode::None;
                cursor_options.visible = true;
            }
        }
    }

    // Handle mouse input
    if accumulated_mouse_motion.delta != Vec2::ZERO && cursor_grab {
        // Apply look update
        state.pitch = (state.pitch
            - accumulated_mouse_motion.delta.y * RADIANS_PER_DOT * config.sensitivity)
            .clamp(-PI / 2., PI / 2.);
        state.yaw -= accumulated_mouse_motion.delta.x * RADIANS_PER_DOT * config.sensitivity;
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, state.yaw, state.pitch);
    }
}
