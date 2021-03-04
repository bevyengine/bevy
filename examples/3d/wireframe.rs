use bevy::prelude::*;
use bevy_internal::{
    render::wireframe::{Wireframe, WireframeConfig, WireframePlugin},
    wgpu::{WgpuFeature, WgpuFeatures, WgpuOptions},
};

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WgpuOptions {
            features: WgpuFeatures {
                // The Wireframe requires NonFillPolygonMode feature
                features: vec![WgpuFeature::NonFillPolygonMode],
            },
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
        .add_startup_system(setup.system())
        .run();
}

/// set up a simple 3D scene
fn setup(
    commands: &mut Commands,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // To draw the wireframe on all entities, set this to 'true'
    wireframe_config.global = false;
    // add entities to the world
    commands
        // plane
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..Default::default()
        })
        // cube
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..Default::default()
        })
        .with(Wireframe) // This enables wireframe drawing on this entity
        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
