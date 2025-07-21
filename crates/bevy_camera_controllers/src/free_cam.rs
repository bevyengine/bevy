//! A camera controller that allows the user to move freely around the scene.
//!
//! Free cams are helpful for exploring large scenes, level editors and for debugging.
//! They are rarely useful as-is for gameplay,
//! as they allow the user to move freely in all directions,
//! which can be disorienting, and they can clip through objects and terrain.
//!
//! Fly cams are a kind of free cam, designed for fluid "flying" movement.
//! This particular free cam is optimized for precise control,
//! allowing the user to move around the scene with a keyboard and mouse
//! for debugging and development purposes.

use bevy_app::{App, Plugin, Update};
use bevy_camera::Camera;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::{
    AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseScrollUnit,
};
use bevy_input::ButtonInput;
use bevy_log::info;
use bevy_math::{EulerRot, Quat, Vec2, Vec3};
use bevy_time::Time;
use bevy_transform::prelude::Transform;
use bevy_window::{CursorGrabMode, CursorOptions, Window};

use std::{f32::consts::*, fmt};

/// A freecam-style camera controller plugin.
///
/// Use [`FreeCamController`] to add a freecam controller to a camera entity,
/// and change its values to customize the controls and change its behavior.
pub struct FreeCamPlugin;

impl Plugin for FreeCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, run_freecam_controller);
    }
}

/// Based on Valorant's default sensitivity, not entirely sure why it is exactly 1.0 / 180.0,
/// but I'm guessing it is a misunderstanding between degrees/radians and then sticking with
/// it because it felt nice.
pub const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

/// Freecam controller settings and state.
///
/// Add this component to a [`Camera`] entity and add [`FreeCamPlugin`]
/// to your [`App`] to enable freecam controls.
#[derive(Component)]
pub struct FreeCamController {
    /// Enables this [`FreeCamController`] when `true`.
    pub enabled: bool,
    /// Indicates if this controller has been initialized by the [`FreeCamControllerPlugin`].
    pub initialized: bool,
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
    /// [`KeyCode`] to use [`run_speed`](FreeCamController::run_speed) instead of
    /// [`walk_speed`](FreeCamController::walk_speed) for translation.
    pub key_run: KeyCode,
    /// [`MouseButton`] for grabbing the mouse focus.
    pub mouse_key_cursor_grab: MouseButton,
    /// [`KeyCode`] for grabbing the keyboard focus.
    pub keyboard_key_toggle_cursor_grab: KeyCode,
    /// Multiplier for unmodified translation speed.
    pub walk_speed: f32,
    /// Multiplier for running translation speed.
    pub run_speed: f32,
    /// Multiplier for how the mouse scroll wheel modifies [`walk_speed`](FreeCamController::walk_speed)
    /// and [`run_speed`](FreeCamController::run_speed).
    pub scroll_factor: f32,
    /// Friction factor used to exponentially decay [`velocity`](FreeCamController::velocity) over time.
    pub friction: f32,
    /// This [`FreeCamController`]'s pitch rotation.
    pub pitch: f32,
    /// This [`FreeCamController`]'s yaw rotation.
    pub yaw: f32,
    /// This [`FreeCamController`]'s translation velocity.
    pub velocity: Vec3,
}

impl Default for FreeCamController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 1.0,
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
            scroll_factor: 0.1,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

impl fmt::Display for FreeCamController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Freecam Controls:
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

fn run_freecam_controller(
    time: Res<Time>,
    mut windows: Query<(&Window, &mut CursorOptions)>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut toggle_cursor_grab: Local<bool>,
    mut mouse_cursor_grab: Local<bool>,
    mut query: Query<(&mut Transform, &mut FreeCamController), With<Camera>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, mut controller)) = query.single_mut() else {
        return;
    };

    if !controller.initialized {
        let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
        controller.yaw = yaw;
        controller.pitch = pitch;
        controller.initialized = true;
        info!("{}", *controller);
    }
    if !controller.enabled {
        return;
    }

    let mut scroll = 0.0;

    let amount = match accumulated_mouse_scroll.unit {
        MouseScrollUnit::Line => accumulated_mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => accumulated_mouse_scroll.delta.y / 16.0,
    };
    scroll += amount;
    controller.walk_speed += scroll * controller.scroll_factor * controller.walk_speed;
    controller.run_speed = controller.walk_speed * 3.0;

    // Handle key input
    let mut axis_input = Vec3::ZERO;
    if key_input.pressed(controller.key_forward) {
        axis_input.z += 1.0;
    }
    if key_input.pressed(controller.key_back) {
        axis_input.z -= 1.0;
    }
    if key_input.pressed(controller.key_right) {
        axis_input.x += 1.0;
    }
    if key_input.pressed(controller.key_left) {
        axis_input.x -= 1.0;
    }
    if key_input.pressed(controller.key_up) {
        axis_input.y += 1.0;
    }
    if key_input.pressed(controller.key_down) {
        axis_input.y -= 1.0;
    }

    let mut cursor_grab_change = false;
    if key_input.just_pressed(controller.keyboard_key_toggle_cursor_grab) {
        *toggle_cursor_grab = !*toggle_cursor_grab;
        cursor_grab_change = true;
    }
    if mouse_button_input.just_pressed(controller.mouse_key_cursor_grab) {
        *mouse_cursor_grab = true;
        cursor_grab_change = true;
    }
    if mouse_button_input.just_released(controller.mouse_key_cursor_grab) {
        *mouse_cursor_grab = false;
        cursor_grab_change = true;
    }
    let cursor_grab = *mouse_cursor_grab || *toggle_cursor_grab;

    // Update velocity
    if axis_input != Vec3::ZERO {
        let max_speed = if key_input.pressed(controller.key_run) {
            controller.run_speed
        } else {
            controller.walk_speed
        };
        controller.velocity = axis_input.normalize() * max_speed;
    } else {
        let friction = controller.friction.clamp(0.0, 1.0);
        controller.velocity *= 1.0 - friction;
        if controller.velocity.length_squared() < 1e-6 {
            controller.velocity = Vec3::ZERO;
        }
    }

    // Apply movement update
    if controller.velocity != Vec3::ZERO {
        let forward = *transform.forward();
        let right = *transform.right();
        transform.translation += controller.velocity.x * dt * right
            + controller.velocity.y * dt * Vec3::Y
            + controller.velocity.z * dt * forward;
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
        controller.pitch = (controller.pitch
            - accumulated_mouse_motion.delta.y * RADIANS_PER_DOT * controller.sensitivity)
            .clamp(-PI / 2., PI / 2.);
        controller.yaw -=
            accumulated_mouse_motion.delta.x * RADIANS_PER_DOT * controller.sensitivity;
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
    }
}
