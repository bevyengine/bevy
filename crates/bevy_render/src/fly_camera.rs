use crate::{
    camera::{Camera, PerspectiveProjection, VisibleEntities},
    render_graph::base,
};
use bevy_app::{AppBuilder, EventReader, Events, Plugin};
use bevy_core::Time;
use bevy_ecs::*;
use bevy_ecs::{Bundle, Query, Res, ResMut};
use bevy_input::{keyboard::KeyCode, mouse::MouseMotion, Input};
use bevy_math::{Quat, Vec2, Vec3};
use bevy_transform::components::{Rotation, Scale, Transform, Translation};

/**
Used in [CameraFlyingComponents](struct.CameraFlyingComponents.html)
**/
pub struct CameraFlyingOptions {
    /// The camera's flying speed
    pub speed: f32,
    /// The camera's mouse sensitivity
    pub sensitivity: f32,
    /// The vertical pitch of the camera, constrained between `89.9f32` and `-89.9f32`. This value is kept up-to-date by the [CameraFlyingPlugin](struct.CameraFlyingPlugin.html) plugin, and can be mutated to adjust the camera's angle
    pub pitch: f32,
    /// The horizontal yaw of the camera. This value is kept up-to-date by the [CameraFlyingPlugin](struct.CameraFlyingPlugin.html) plugin, and can be mutated to adjust the camera's angle
    pub yaw: f32,
}
impl Default for CameraFlyingOptions {
    fn default() -> Self {
        Self {
            speed: 10.0,
            sensitivity: 10.0,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

/**
A basic flying camera for 3D scenes.

| Keybind         | Action                          |
|-----------------|---------------------------------|
| `W / A / S / D` | Move along the horizontal plane |
| `Space`         | Move upward                     |
| `Shift`         | Move downward                   |

```ignore

fn setup(mut commands: Commands) {
    commands.spawn(CameraFlyingComponents::default());
}

fn main () {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

```

**/
#[derive(Bundle)]
pub struct CameraFlyingComponents {
    pub options: CameraFlyingOptions,
    pub camera: Camera,
    pub perspective_projection: PerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for CameraFlyingComponents {
    fn default() -> Self {
        Self {
            options: CameraFlyingOptions::default(),
            camera: Camera {
                name: Some(base::camera::CAMERA3D.to_string()),
                ..Default::default()
            },
            perspective_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

fn forward_vector(rotation: &Rotation) -> Vec3 {
    rotation.mul_vec3(Vec3::unit_z()).normalize()
}

fn forward_walk_vector(rotation: &Rotation) -> Vec3 {
    let f = forward_vector(rotation);
    let f_flattened = Vec3::new(f.x(), 0.0, f.z()).normalize();
    f_flattened
}

fn strafe_vector(rotation: &Rotation) -> Vec3 {
    // Rotate it 90 degrees to get the strafe direction
    Rotation::from_rotation_y(90.0f32.to_radians())
        .mul_vec3(forward_walk_vector(rotation))
        .normalize()
}

fn movement_axis(input: &Res<Input<KeyCode>>, plus: KeyCode, minus: KeyCode) -> f32 {
    let mut axis = 0.0;
    if input.pressed(plus) {
        axis += 1.0;
    }
    if input.pressed(minus) {
        axis -= 1.0;
    }
    axis
}

fn camera_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&CameraFlyingOptions, &mut Translation, &Rotation)>,
) {
    let axis_h = movement_axis(&keyboard_input, KeyCode::D, KeyCode::A);
    let axis_v = movement_axis(&keyboard_input, KeyCode::S, KeyCode::W);

    let axis_float = movement_axis(&keyboard_input, KeyCode::Space, KeyCode::LShift);

    for (options, mut translation, rotation) in &mut query.iter() {
        let delta_f = forward_walk_vector(rotation) * axis_v * options.speed * time.delta_seconds;

        let delta_strafe = strafe_vector(rotation) * axis_h * options.speed * time.delta_seconds;

        let delta_float = Vec3::unit_y() * axis_float * options.speed * time.delta_seconds;

        translation.0 += delta_f + delta_strafe + delta_float;
    }
}

#[derive(Default)]
struct State {
    mouse_motion_event_reader: EventReader<MouseMotion>,
}

fn mouse_motion_system(
    time: Res<Time>,
    mut state: ResMut<State>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    mut query: Query<(&mut CameraFlyingOptions, &mut Rotation)>,
) {
    let mut delta: Vec2 = Vec2::zero();
    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
        delta += event.delta;
    }

    for (mut options, mut rotation) in &mut query.iter() {
        options.yaw -= delta.x() * options.sensitivity * time.delta_seconds;
        options.pitch += delta.y() * options.sensitivity * time.delta_seconds;

        if options.pitch > 89.9 {
            options.pitch = 89.9;
        }
        if options.pitch < -89.9 {
            options.pitch = -89.9;
        }

        let yaw_radians = options.yaw.to_radians();
        let pitch_radians = options.pitch.to_radians();

        rotation.0 = Quat::from_axis_angle(Vec3::unit_y(), yaw_radians)
            * Quat::from_axis_angle(-Vec3::unit_x(), pitch_radians);
    }
}

pub struct CameraFlyingPlugin;

impl Plugin for CameraFlyingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<State>()
            .add_system(camera_movement_system.system())
            .add_system(mouse_motion_system.system());
    }
}
