//! Demonstrates how to write a custom fullscreen shader

use bevy::{
    pbr::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    prelude::*,
    shader::ShaderRef,
};
use bevy_render::{extract_component::ExtractComponent, render_resource::ShaderType};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FullscreenMaterialPlugin::<MyPostProcessing>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::default(), Vec3::Y),
        MyPostProcessing { data: 0.005 },
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn(DirectionalLight {
        illuminance: 1_000.,
        ..default()
    });
}

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
struct MyPostProcessing {
    data: f32,
}

impl FullscreenMaterial for MyPostProcessing {
    fn fragment_shader() -> ShaderRef {
        "shaders/my_post_processing.wgsl".into()
    }
}
