//! Display debugging information for PBR materials.
//! See the scene viewer example for an interactive version.

use bevy::{
    pbr::{PbrDebug, PbrDebugMaterial},
    prelude::*,
};

#[derive(Component)]
struct ShouldRotate;

#[derive(Component)]
struct Helmet;

const HELMET_SCALE: Vec3 = Vec3::splat(2.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate)
        .add_system(spawn_with_debug_materials)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut debug_materials: ResMut<Assets<PbrDebugMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: standard_materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // cube
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: debug_materials.add(PbrDebugMaterial {
                pbr_debug: PbrDebug::Uvs,
            }),
            transform: Transform::from_xyz(-1.5, 0.5, 0.0),
            ..default()
        },
        ShouldRotate,
    ));

    commands.spawn((
        SceneBundle {
            scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
            transform: Transform::from_xyz(1.5, 0.0, 2.0).with_scale(HELMET_SCALE),
            ..default()
        },
        Helmet,
        ShouldRotate,
    ));

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

    println!("The cube shows UVs.");
    println!("The helmets show (from back to front):");
    println!("\t- World position");
    println!("\t- World normals");
    println!("\t- UVs");
    println!("\t- World tangents");
}

fn rotate(mut query: Query<&mut Transform, With<ShouldRotate>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

// This system spawns new helmets with a different debug material each time.
fn spawn_with_debug_materials(
    mut done: Local<bool>,
    mut commands: Commands,
    mut debug_materials: ResMut<Assets<PbrDebugMaterial>>,
    helmet_scene: Query<Entity, With<Helmet>>,
    children: Query<&Children>,
    meshes: Query<(&Handle<Mesh>, &Transform)>,
) {
    if *done {
        return;
    }

    for helmet_scene_entity in &helmet_scene {
        for entity in children.iter_descendants(helmet_scene_entity) {
            if let Ok((mesh, transform)) = meshes.get(entity) {
                *done = true;

                // TODO: Enumerate all?
                for (index, pbr_debug) in [
                    PbrDebug::Uvs,
                    PbrDebug::Depth,
                    PbrDebug::InterpolatedVertexNormals,
                    PbrDebug::InterpolatedVertexTangents,
                    PbrDebug::TangentSpaceNormalMap, // pink
                    PbrDebug::NormalMappedNormal,    // pink
                    PbrDebug::ViewSpaceNormalMappedNormal, // pink
                                                     // PbrDebug::BaseColor, // crash
                                                     // PbrDebug::BaseColorTexture, // crash
                                                     // PbrDebug::Emissive, // crash
                                                     // PbrDebug::EmissiveTexture,
                                                     // PbrDebug::Roughness,
                                                     // PbrDebug::RoughnessTexture,
                                                     // PbrDebug::Metallic,
                                                     // PbrDebug::MetallicTexture,
                                                     // PbrDebug::Reflectance,
                                                     // PbrDebug::OcclusionTexture,
                                                     // PbrDebug::Opaque, // crash
                                                     // PbrDebug::AlphaMask,
                                                     // PbrDebug::AlphaBlend,
                                                     // PbrDebug::ClusteredForwardDebugZSlices, // crash
                                                     // PbrDebug::ClusteredForwardDebugClusterLightComplexity,
                                                     // PbrDebug::ClusteredForwardDebugClusterCoherency,
                ]
                .iter()
                .enumerate()
                {
                    let extra_z = (index % 4) as f32;
                    let extra_x = (index / 4) as f32;

                    let mut transform = *transform;
                    transform.translation.z += extra_z;
                    transform.translation.x += extra_x;
                    transform.scale = HELMET_SCALE;
                    let pbr_debug = *pbr_debug;

                    commands.spawn((
                        MaterialMeshBundle {
                            mesh: mesh.clone_weak(),
                            material: debug_materials.add(PbrDebugMaterial { pbr_debug }),
                            transform,
                            ..default()
                        },
                        ShouldRotate,
                    ));
                }
            }
        }
    }
}
