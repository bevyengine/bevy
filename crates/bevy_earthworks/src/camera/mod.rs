//! Camera control for earthworks visualization.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::mouse::{MouseMotion, MouseWheel};
use bevy_input::prelude::*;
use bevy_math::{Quat, Vec2, Vec3};
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;

/// Plugin for camera systems.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, orbit_camera_system);
    }
}

/// Component for an orbit camera.
#[derive(Component, Clone, Debug, Reflect)]
pub struct OrbitCamera {
    /// Center point to orbit around.
    pub target: Vec3,
    /// Distance from target.
    pub distance: f32,
    /// Horizontal angle in radians.
    pub yaw: f32,
    /// Vertical angle in radians.
    pub pitch: f32,
    /// Rotation speed.
    pub rotation_speed: f32,
    /// Zoom speed.
    pub zoom_speed: f32,
    /// Pan speed.
    pub pan_speed: f32,
    /// Minimum distance.
    pub min_distance: f32,
    /// Maximum distance.
    pub max_distance: f32,
    /// Minimum pitch (looking down).
    pub min_pitch: f32,
    /// Maximum pitch (looking up).
    pub max_pitch: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 50.0,
            yaw: 0.0,
            pitch: -0.5, // Looking slightly down
            rotation_speed: 0.005,
            zoom_speed: 5.0,
            pan_speed: 0.1,
            min_distance: 5.0,
            max_distance: 500.0,
            min_pitch: -std::f32::consts::FRAC_PI_2 + 0.1,
            max_pitch: std::f32::consts::FRAC_PI_2 - 0.1,
        }
    }
}

impl OrbitCamera {
    /// Creates a new orbit camera with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the target to orbit around.
    pub fn with_target(mut self, target: Vec3) -> Self {
        self.target = target;
        self
    }

    /// Sets the initial distance.
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    /// Calculates the camera position based on current orbit parameters.
    pub fn calculate_position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, -y, z)
    }

    /// Calculates the camera rotation to look at the target.
    pub fn calculate_rotation(&self) -> Quat {
        let position = self.calculate_position();
        let direction = (self.target - position).normalize();

        // Calculate rotation to face the target
        let yaw = direction.x.atan2(direction.z);
        let pitch = (-direction.y).asin();

        Quat::from_euler(bevy_math::EulerRot::YXZ, yaw, pitch, 0.0)
    }
}

/// System that handles orbit camera input and updates.
pub fn orbit_camera_system(
    mut cameras: Query<(&mut OrbitCamera, &mut Transform)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
) {
    let mut total_motion = Vec2::ZERO;
    for event in mouse_motion.read() {
        total_motion += event.delta;
    }

    let mut total_scroll = 0.0;
    for event in mouse_wheel.read() {
        total_scroll += event.y;
    }

    for (mut orbit, mut transform) in cameras.iter_mut() {
        // Rotate with right mouse button
        if mouse_button.pressed(MouseButton::Right) {
            orbit.yaw -= total_motion.x * orbit.rotation_speed;
            orbit.pitch -= total_motion.y * orbit.rotation_speed;
            orbit.pitch = orbit.pitch.clamp(orbit.min_pitch, orbit.max_pitch);
        }

        // Pan with middle mouse button
        if mouse_button.pressed(MouseButton::Middle) {
            let right: Vec3 = transform.right().into();
            let pan_horizontal = right * -total_motion.x * orbit.pan_speed;
            let pan_vertical = Vec3::Y * total_motion.y * orbit.pan_speed;
            orbit.target += pan_horizontal + pan_vertical;
        }

        // Zoom with scroll wheel
        if total_scroll != 0.0 {
            orbit.distance -= total_scroll * orbit.zoom_speed;
            orbit.distance = orbit.distance.clamp(orbit.min_distance, orbit.max_distance);
        }

        // Keyboard controls
        let speed = if keyboard.pressed(KeyCode::ShiftLeft) {
            2.0
        } else {
            1.0
        };

        if keyboard.pressed(KeyCode::KeyW) {
            let forward: Vec3 = transform.forward().into();
            orbit.target += Vec3::new(forward.x, 0.0, forward.z).normalize() * speed;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            let forward: Vec3 = transform.forward().into();
            orbit.target -= Vec3::new(forward.x, 0.0, forward.z).normalize() * speed;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            let right: Vec3 = transform.right().into();
            orbit.target -= Vec3::new(right.x, 0.0, right.z).normalize() * speed;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            let right: Vec3 = transform.right().into();
            orbit.target += Vec3::new(right.x, 0.0, right.z).normalize() * speed;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            orbit.target.y -= speed;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            orbit.target.y += speed;
        }

        // Update transform
        transform.translation = orbit.calculate_position();
        transform.rotation = orbit.calculate_rotation();
    }
}
