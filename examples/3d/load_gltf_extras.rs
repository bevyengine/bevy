//! Loads and renders a glTF file as a scene, and list all the different `gltf_extras`.

use bevy::{
    gltf::{GltfExtras, GltfMaterialExtras, GltfMeshExtras, GltfSceneExtras},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, check_for_gltf_extras)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
    // a barebones scene containing one of each gltf_extra type
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/extras/gltf_extras.glb#Scene0"),
        ..default()
    });
}

fn check_for_gltf_extras(
    gltf_extras_query: Query<(Entity, Option<&Name>, &GltfExtras), Added<GltfExtras>>,
    gltf_scene_extras_query: Query<
        (Entity, Option<&Name>, &GltfSceneExtras),
        Added<GltfSceneExtras>,
    >,
    gltf_mesh_extras_query: Query<(Entity, Option<&Name>, &GltfMeshExtras), Added<GltfMeshExtras>>,
    gltf_material_extras_query: Query<
        (Entity, Option<&Name>, &GltfMaterialExtras),
        Added<GltfMaterialExtras>,
    >,
) {
    for extra in gltf_extras_query.iter() {
        info!(
            "primitive extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }

    for extra in gltf_scene_extras_query.iter() {
        info!(
            "scene extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }

    for extra in gltf_mesh_extras_query.iter() {
        info!(
            "mesh extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }

    for extra in gltf_material_extras_query.iter() {
        info!(
            "material extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }
}
