//! Showcases wireframe rendering.

use bevy::{
    pbr::wireframe::{NeverRenderWireframe, Wireframe, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{render_resource::WgpuFeatures, settings::WgpuSettings, RenderPlugin},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }
                .into(),
            }),
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
        material: materials.add(Color::rgb(0.3, 0.3, 0.5).into()),
        ..default()
    });

    // Red cube: Never renders a wireframe
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.1, 0.1).into()),
            transform: Transform::from_xyz(-1.0, 0.5, -1.0),
            ..default()
        })
        .insert(NeverRenderWireframe);
    // Orange cube: Follows global wireframe setting
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.8, 0.1).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // Green cube: Always renders a wireframe
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.1, 0.8, 0.1).into()),
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
/// [`Wireframe::AlwaysRender`], [`Wireframe::NeverRender`], and no override.
fn toggle_global_wireframe_setting(
    time: Res<Time>,
    mut timer: ResMut<WireframeToggleTimer>,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        // The global wireframe config enables drawing of wireframes on every mesh, except those with
        // `WireframeOverride::NeverRender`. Meshes with `WireframeOverride::AlwaysRender` will
        // always have a wireframe, regardless of the global configuration.
        wireframe_config.global = !wireframe_config.global;
    }
}
