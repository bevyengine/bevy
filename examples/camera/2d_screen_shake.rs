//! This example showcases a 2D screen shake using concept in this video: `<https://www.youtube.com/watch?v=tu-Qe66AvtY>`
//!
//! ## Controls
//!
//! | Key Binding  | Action               |
//! |:-------------|:---------------------|
//! | Space        | Trigger screen shake |

use bevy::prelude::*;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const CAMERA_DECAY_RATE: f32 = 0.9; // Adjust this for smoother or snappier decay
const AMOUNT_DECAY_SPEED: f32 = 0.5; // How fast amount decays
const AMOUNT_INCREMENT: f32 = 1.0; // Increment of amount per frame when holding space
const MAX_ANGLE: f32 = 0.5; // Maximum angle added per frame
const MAX_OFFSET: f32 = 500.0; // Maximum offset added per frame

#[derive(Resource, Clone, Default)]
struct ScreenShake {
    amount: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<ScreenShake>()
        .add_systems(Startup, setup)
        .add_systems(Update, (shake_screen, trigger_shake))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1000., 700.))),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 0.2, 0.3))),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50.0, 100.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.94, 0.91))),
        Transform::from_xyz(0., 0., 2.),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50.0, 50.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.85, 0.0, 0.2))),
        Transform::from_xyz(-450.0, 200.0, 2.),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(70.0, 50.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.5, 0.8, 0.2))),
        Transform::from_xyz(450.0, -150.0, 2.),
    ));
}

fn trigger_shake(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut screen_shake: ResMut<ScreenShake>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        screen_shake.amount += AMOUNT_INCREMENT * time.delta_secs();
    }
}

fn shake_screen(
    time: Res<Time>,
    mut screen_shake: ResMut<ScreenShake>,
    transform: Single<&mut Transform, With<Camera>>,
) {
    let shake = ops::powf(screen_shake.amount, 2.0);
    let mut transform = transform.into_inner();

    if shake > 0.0 {
        let mut rng = ChaCha8Rng::from_entropy();

        let angle = (MAX_ANGLE * shake).to_radians() * rng.gen_range(-1.0..1.0);
        transform.rotation = transform.rotation.interpolate_stable(
            &(transform.rotation.mul_quat(Quat::from_rotation_z(angle))),
            CAMERA_DECAY_RATE,
        );

        let offset_x = MAX_OFFSET * shake * rng.gen_range(-1.0..1.0);
        let offset_y = MAX_OFFSET * shake * rng.gen_range(-1.0..1.0);
        let target = transform.translation + Vec3::new(offset_x, offset_y, 0.0);
        transform
            .translation
            .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
    } else {
        transform
            .translation
            .smooth_nudge(&Vec3::ZERO, 1.0, time.delta_secs());
        transform.rotation = transform.rotation.interpolate_stable(&Quat::IDENTITY, 0.1);
    }

    screen_shake.amount =
        (screen_shake.amount - (AMOUNT_DECAY_SPEED * time.delta_secs())).clamp(0.0, 1.0);
}
