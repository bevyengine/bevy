//! Showcases wireframe rendering.

use bevy::{
    pbr::wireframe::{Wireframe, WireframeColor, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::POLYGON_MODE_LINE,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(WireframeConfig {
            // To draw the wireframe on all entities with a Mesh, set this to 'true'
            on_all_meshes: false,
            // You can also change the default color of the wireframes, which controls:
            // - all wireframes if `WireframeConfig::on_all_meshes` is set to 'true'
            // - the wireframe of all entities that do not have a `WireframeColor` otherwise
            default_color: Color::AQUAMARINE,
        })
        .add_plugin(WireframePlugin)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        // This enables wireframe drawing for the entity
        .insert(Wireframe)
        // This overrides the WireframeConfig::default_color (just for this entity)
        .insert(WireframeColor(Color::FUCHSIA));
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
