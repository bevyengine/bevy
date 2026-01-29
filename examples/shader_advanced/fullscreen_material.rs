//! Demonstrates how to write a custom fullscreen shader
//!
//! This example demonstrates working in 3d. To make the example work in 2d,
//! replace 3d components with their 2d counterparts, and schedule the work
//! to run in the `Core2d` schedule as described in the `FullscreenMaterial`
//! comment in this file.

use bevy::{
    core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FullscreenMaterialPlugin::<FullscreenEffect>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::default(), Vec3::Y),
        FullscreenEffect { intensity: 0.005 },
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::default(),
    ));

    commands.spawn(DirectionalLight {
        illuminance: 1_000.,
        ..default()
    });
}

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
struct FullscreenEffect {
    intensity: f32,
}

impl FullscreenMaterial for FullscreenEffect {
    fn fragment_shader() -> ShaderRef {
        "shaders/fullscreen_effect.wgsl".into()
    }

    // The `FullscreenMaterial` uses 3d schedules by default.
    // To make this work in 2d, you would need to schedule to
    // run in `Core2d` and in a `Core2dSystems` set.
    //
    // fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone {
    //     bevy::core_pipeline::Core2d
    // }
    // fn run_in() -> impl SystemSet {
    //     bevy::core_pipeline::Core2dSystems::PostProcess
    // }
}
