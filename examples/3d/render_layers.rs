//! A 3D scene showcasing the use of render layers.

use bevy::{
    color::palettes::css::{BLUE, RED},
    prelude::*,
};
use bevy_render::view::RenderLayers;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, orbit)
        .run();
}

const LAYER_A: RenderLayers = RenderLayers::layer(0);
const LAYER_B: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
struct Orbit;

/// set up a scene using render layers
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // -------------------
    // both layers
    // -------------------

    // circular base
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Circle::new(4.0)),
            material: materials.add(Color::WHITE),
            transform: Transform::from_rotation(Quat::from_rotation_x(
                -std::f32::consts::FRAC_PI_2,
            )),
            ..default()
        })
        .insert(LAYER_A | LAYER_B);

    // camera
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 9.0, 0.0).looking_at(Vec3::ZERO, -Vec3::Z),
            ..default()
        })
        .insert(LAYER_A | LAYER_B);

    // -------------------
    // layer A
    // -------------------

    // sphere
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Sphere::new(0.75)),
            material: materials.add(Color::from(RED)),
            transform: Transform::from_xyz(-1.0, 0.5, 0.0),
            ..default()
        })
        .insert(LAYER_A);

    // light
    commands
        .spawn(PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 6.0, -4.0),
            ..default()
        })
        .insert(Orbit)
        .insert(LAYER_A);

    // -------------------
    // layer B
    // -------------------

    // sphere
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Sphere::new(0.75)),
            material: materials.add(Color::from(BLUE)),
            transform: Transform::from_xyz(1.0, 0.5, 0.0),
            ..default()
        })
        .insert(LAYER_B);

    // light
    commands
        .spawn(PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 6.0, -4.0),
            ..default()
        })
        .insert(LAYER_B);
}

/// make the entity orbit around the origin
fn orbit(time: Res<Time>, mut query: Query<&mut Transform, With<Orbit>>) {
    for mut transform in &mut query {
        transform.translation = Vec3::new(
            5.65 * time.elapsed_seconds().cos(),
            6.0,
            5.65 * time.elapsed_seconds().sin(),
        );
    }
}
