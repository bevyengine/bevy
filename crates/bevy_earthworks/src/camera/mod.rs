//! Camera control for earthworks visualization.
//!
//! Features:
//! - Orbit camera with mouse and keyboard controls
//! - Trauma-based camera shake system for game feel

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::mouse::{MouseMotion, MouseWheel};
use bevy_input::prelude::*;
use bevy_math::{Quat, Vec2, Vec3};
use bevy_reflect::Reflect;
use bevy_time::Time;
use bevy_transform::components::Transform;

/// Plugin for camera systems.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraShake>()
            .add_systems(Update, (orbit_camera_system, apply_camera_shake).chain());
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

/// Trauma-based camera shake component.
///
/// Uses Squirrel Eiserloh's GDC talk "Juice it or Lose it" approach:
/// - Trauma accumulates from impacts (0.0 - 1.0)
/// - Shake amount = trauma² (smoother feel at low trauma)
/// - Trauma decays over time
#[derive(Component, Clone, Debug, Reflect)]
pub struct CameraShake {
    /// Current trauma level (0.0 - 1.0).
    pub trauma: f32,
    /// Maximum translation offset in world units.
    pub max_offset: Vec3,
    /// Maximum rotation offset in radians.
    pub max_rotation: f32,
    /// Trauma decay rate per second.
    pub decay: f32,
    /// Noise time accumulator for smooth shake.
    noise_time: f32,
}

impl Default for CameraShake {
    fn default() -> Self {
        Self {
            trauma: 0.0,
            max_offset: Vec3::new(0.3, 0.2, 0.0),
            max_rotation: 0.02,
            decay: 1.5,
            noise_time: 0.0,
        }
    }
}

impl CameraShake {
    /// Creates a new camera shake with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds trauma to the shake (capped at 1.0).
    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// Returns the current shake intensity (trauma squared for smoother feel).
    pub fn shake_amount(&self) -> f32 {
        self.trauma * self.trauma
    }

    /// Returns true if there is active shake.
    pub fn is_shaking(&self) -> bool {
        self.trauma > 0.001
    }
}

/// Event to add camera trauma from anywhere in the codebase.
#[derive(Clone, Debug, bevy_ecs::prelude::Message)]
pub struct CameraTraumaEvent {
    /// Amount of trauma to add (0.0 - 1.0).
    pub amount: f32,
}

impl CameraTraumaEvent {
    /// Light impact (blade scrape, small excavation).
    pub fn light() -> Self {
        Self { amount: 0.03 }
    }

    /// Medium impact (digging, pushing load).
    pub fn medium() -> Self {
        Self { amount: 0.08 }
    }

    /// Heavy impact (dump load, job complete).
    pub fn heavy() -> Self {
        Self { amount: 0.15 }
    }

    /// Custom trauma amount.
    pub fn custom(amount: f32) -> Self {
        Self { amount }
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

/// System that applies camera shake effect.
fn apply_camera_shake(time: Res<Time>, mut query: Query<(&mut Transform, &mut CameraShake)>) {
    let dt = time.delta_secs();

    for (mut transform, mut shake) in query.iter_mut() {
        if !shake.is_shaking() {
            continue;
        }

        let shake_amount = shake.shake_amount();

        // Update noise time for smooth shake
        shake.noise_time += dt * 25.0;
        let t = shake.noise_time;

        // Use multiple sine waves at different frequencies for pseudo-noise
        // This creates a more organic shake than single frequency
        let noise_x = (t * 1.0).sin() * 0.5
            + (t * 2.3).sin() * 0.3
            + (t * 5.7).sin() * 0.2;
        let noise_y = (t * 1.1).sin() * 0.5
            + (t * 2.7).sin() * 0.3
            + (t * 6.1).sin() * 0.2;
        let noise_rot = (t * 0.9).sin() * 0.5
            + (t * 2.1).sin() * 0.3
            + (t * 4.3).sin() * 0.2;

        // Apply offset scaled by shake amount
        let offset = Vec3::new(
            noise_x * shake.max_offset.x * shake_amount,
            noise_y * shake.max_offset.y * shake_amount,
            0.0,
        );

        transform.translation += offset;

        // Apply rotation shake (roll only for best effect)
        let rotation_offset = noise_rot * shake.max_rotation * shake_amount;
        transform.rotate_local_z(rotation_offset);

        // Decay trauma
        shake.trauma = (shake.trauma - shake.decay * dt).max(0.0);
    }
}

/// System to handle camera trauma events.
pub fn handle_camera_trauma_events(
    mut events: MessageReader<CameraTraumaEvent>,
    mut cameras: Query<&mut CameraShake>,
) {
    for event in events.read() {
        for mut shake in cameras.iter_mut() {
            shake.add_trauma(event.amount);
        }
    }
}
