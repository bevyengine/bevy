//! Showcases wireframe rendering.
//!
//! Wireframes currently do not work when using webgl or webgpu.
//! Supported platforms:
//! - DX12
//! - Vulkan
//! - Metal
//!
//! This is a native only feature.

use bevy::{
    pbr::wireframe::{NoWireframe, Wireframe, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin,
        ))
        .insert_resource(WireframeToggleTimer(Timer::from_seconds(
            1.0,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_global_wireframe_setting)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::from_size(5.0))),
        material: materials.add(Color::BLUE.into()),
        ..default()
    });

    // Red cube: Never renders a wireframe
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_xyz(-1.0, 0.5, -1.0),
            ..default()
        })
        .insert(NoWireframe);
    // Orange cube: Follows global wireframe setting
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::ORANGE.into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // Green cube: Always renders a wireframe
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::GREEN.into()),
            transform: Transform::from_xyz(1.0, 0.5, 1.0),
            ..default()
        })
        .insert(Wireframe);

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// This timer is used to periodically toggle the wireframe rendering.
#[derive(Resource)]
struct WireframeToggleTimer(Timer);

/// Periodically turns the global wireframe setting on and off, to show the differences between
/// [`Wireframe`], [`NoWireframe`], and just a mesh.
fn toggle_global_wireframe_setting(
    time: Res<Time>,
    mut timer: ResMut<WireframeToggleTimer>,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        // The global wireframe config enables drawing of wireframes on every mesh,
        // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
        // regardless of the global configuration.
        wireframe_config.global = !wireframe_config.global;
    }
}
