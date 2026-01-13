//! Demonstrates how to make an entity invisible to the main camera while still allowing it to cast
//! shadows.
//!
//! This uses `RenderPasses(RenderPassMask::SHADOW)` to exclude entities from the main camera passes
//! while keeping them in the shadow pass.
//!
//! This example also shows a simple “fake shadow caster” setup: the visible red cube is marked
//! `NotShadowCaster`, while a shadow-only sphere is co-located with it so the cube appears to cast a
//! sphere-shaped shadow.

use std::f32::consts::PI;

use bevy::{
    color::palettes::basic::{BLUE, GREEN, RED},
    light::NotShadowCaster,
    prelude::*,
    render::{RenderPassMask, RenderPasses},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_light_direction)
        .run();
}

/// Set up a 3D scene to test shadow casters
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere_radius = 0.5;
    let cube_edge = sphere_radius * 2.0;
    let default_material = materials.add(StandardMaterial::default());
    let sphere_handle = meshes.add(Sphere::new(sphere_radius));

    // Floor/ground plane - our shadow receiver
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(default_material.clone()),
        Name::new("Ground Plane"),
    ));

    // Visible red cube that appears to cast a sphere-shaped shadow
    let cuboid_handle = meshes.add(Cuboid::new(cube_edge, cube_edge, cube_edge));
    // Make the visible red cube not cast shadows; the invisible sphere will cast
    // a shadow in the same place so that it appears to belong to this cube.
    commands.spawn((
        Mesh3d(cuboid_handle),
        MeshMaterial3d(materials.add(Color::from(RED))),
        Transform::from_xyz(0.0, cube_edge * 0.5, 0.0),
        NotShadowCaster,
        Name::new("Red Cube"),
    ));

    // Visible green cube that casts and receives shadows normally
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(cube_edge, cube_edge, cube_edge))),
        MeshMaterial3d(materials.add(Color::from(GREEN))),
        Transform::from_xyz(1.25, cube_edge * 0.5, 0.0),
        Name::new("Green cube"),
    ));

    // Visible blue rectilinear cuboid that casts and receives shadows normally
    let pillar_width = 0.50;
    let pillar_height = pillar_width * 2.75;
    let pillar_depth = pillar_width;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(pillar_width, pillar_height, pillar_depth))),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(0.25, pillar_height * 0.5, 1.25)
            .with_rotation(Quat::from_rotation_y(PI / 3.0)),
        Name::new("Blue cuboid"),
    ));

    // A sphere that only casts a shadow so that the visible red cube appears to cast a
    // sphere-shaped shadow.
    commands.spawn((
        Mesh3d(sphere_handle),
        MeshMaterial3d(default_material.clone()),
        Transform::from_xyz(0.0, sphere_radius, 0.0),
        RenderPasses(RenderPassMask::SHADOW),
        Name::new("Invisible sphere"),
    ));

    // An invisible rectilinear cuboid that casts a shadow but is excluded from the main passes.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(pillar_width, pillar_height, pillar_depth))),
        MeshMaterial3d(default_material.clone()),
        Transform::from_xyz(-2.0, pillar_height * 0.5, 0.0),
        RenderPasses(RenderPassMask::SHADOW),
        Name::new("Invisible cuboid"),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 2., -PI / 4.)),
        Name::new("Light"),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-3.0, 5.0, 3.0).looking_at(Vec3::new(0.0, 1.0, 1.0), Vec3::Y),
        Name::new("Camera"),
    ));
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * PI / 5.0);
    }
}
