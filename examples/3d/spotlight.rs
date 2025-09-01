//! Illustrates spot lights.

use std::f32::consts::*;

use bevy::{
    color::palettes::basic::{MAROON, RED},
    light::NotShadowCaster,
    math::ops,
    prelude::*,
    render::view::Hdr,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const INSTRUCTIONS: &str = "\
Controls
--------
Horizontal Movement: WASD
Vertical Movement: Space and Shift
Rotate Camera: Left and Right Arrows";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            brightness: 20.0,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (light_sway, movement, rotation))
        .run();
}

#[derive(Component)]
struct Movable;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Movable,
    ));

    // cubes

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut rng = ChaCha8Rng::seed_from_u64(19878367467713);
    let cube_mesh = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
    let blue = materials.add(Color::srgb_u8(124, 144, 255));

    commands.spawn_batch(
        std::iter::repeat_with(move || {
            let x = rng.random_range(-5.0..5.0);
            let y = rng.random_range(0.0..3.0);
            let z = rng.random_range(-5.0..5.0);

            (
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(blue.clone()),
                Transform::from_xyz(x, y, z),
                Movable,
            )
        })
        .take(40),
    );

    let sphere_mesh = meshes.add(Sphere::new(0.05).mesh().uv(32, 18));
    let sphere_mesh_direction = meshes.add(Sphere::new(0.1).mesh().uv(32, 18));
    let red_emissive = materials.add(StandardMaterial {
        base_color: RED.into(),
        emissive: LinearRgba::new(1.0, 0.0, 0.0, 0.0),
        ..default()
    });
    let maroon_emissive = materials.add(StandardMaterial {
        base_color: MAROON.into(),
        emissive: LinearRgba::new(0.369, 0.0, 0.0, 0.0),
        ..default()
    });

    for x in 0..4 {
        for z in 0..4 {
            let x = x as f32 - 2.0;
            let z = z as f32 - 2.0;
            // red spot_light
            commands
                .spawn((
                    SpotLight {
                        intensity: 40_000.0, // lumens
                        color: Color::WHITE,
                        shadows_enabled: true,
                        inner_angle: PI / 4.0 * 0.85,
                        outer_angle: PI / 4.0,
                        ..default()
                    },
                    Transform::from_xyz(1.0 + x, 2.0, z)
                        .looking_at(Vec3::new(1.0 + x, 0.0, z), Vec3::X),
                ))
                .with_children(|builder| {
                    builder.spawn((
                        Mesh3d(sphere_mesh.clone()),
                        MeshMaterial3d(red_emissive.clone()),
                    ));
                    builder.spawn((
                        Mesh3d(sphere_mesh_direction.clone()),
                        MeshMaterial3d(maroon_emissive.clone()),
                        Transform::from_translation(Vec3::Z * -0.1),
                        NotShadowCaster,
                    ));
                });
        }
    }

    // camera
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(-4.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::new(INSTRUCTIONS),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn light_sway(time: Res<Time>, mut query: Query<(&mut Transform, &mut SpotLight)>) {
    for (mut transform, mut angles) in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            -FRAC_PI_2 + ops::sin(time.elapsed_secs() * 0.67 * 3.0) * 0.5,
            ops::sin(time.elapsed_secs() * 3.0) * 0.5,
            0.0,
        );
        let angle = (ops::sin(time.elapsed_secs() * 1.2) + 1.0) * (FRAC_PI_4 - 0.1);
        angles.inner_angle = angle * 0.8;
        angles.outer_angle = angle;
    }
}

fn movement(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Movable>>,
) {
    // Calculate translation to move the cubes and ground plane
    let mut translation = Vec3::ZERO;

    // Horizontal forward and backward movement
    if input.pressed(KeyCode::KeyW) {
        translation.z += 1.0;
    } else if input.pressed(KeyCode::KeyS) {
        translation.z -= 1.0;
    }

    // Horizontal left and right movement
    if input.pressed(KeyCode::KeyA) {
        translation.x += 1.0;
    } else if input.pressed(KeyCode::KeyD) {
        translation.x -= 1.0;
    }

    // Vertical movement
    if input.pressed(KeyCode::ShiftLeft) {
        translation.y += 1.0;
    } else if input.pressed(KeyCode::Space) {
        translation.y -= 1.0;
    }

    translation *= 2.0 * time.delta_secs();

    // Apply translation
    for mut transform in &mut query {
        transform.translation += translation;
    }
}

fn rotation(
    mut transform: Single<&mut Transform, With<Camera>>,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();

    if input.pressed(KeyCode::ArrowLeft) {
        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(delta));
    } else if input.pressed(KeyCode::ArrowRight) {
        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(-delta));
    }
}
