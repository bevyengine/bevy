//! Loads and renders a glTF file as a scene, and shows all the different `gltf_extras`.

use bevy::{
    gltf::{GltfExtras, GltfMaterialExtras, GltfMeshExtras, GltfSceneExtras},
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, check_for_gltf_extras)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 250.0,
        },
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // This is a relatively small scene, so use tighter shadow
        // cascade bounds than the default for better quality.
        // We also adjusted the shadow map to be larger since we're
        // only using a single cascade.
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 1.6,
            ..default()
        }
        .into(),
        ..default()
    });
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/extras/gltf_extras.glb#Scene0"),
        ..default()
    });
}

fn check_for_gltf_extras(
    gltf_extras_query: Query<(Entity, Option<&Name>, &GltfExtras)>,
    gltf_scene_extras_query: Query<(Entity, Option<&Name>, &GltfSceneExtras)>,
    gltf_mesh_extras_query: Query<(Entity, Option<&Name>, &GltfMeshExtras)>,
    gltf_material_extras_query: Query<(Entity, Option<&Name>, &GltfMaterialExtras)>,
) {
    for extra in gltf_extras_query.iter() {
        println!(
            "primitive extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }

    for extra in gltf_scene_extras_query.iter() {
        println!(
            "scene extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }

    for extra in gltf_mesh_extras_query.iter() {
        println!(
            "mesh extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }

    for extra in gltf_material_extras_query.iter() {
        println!(
            "material extra for {:?} : {:?} (id: {})",
            extra.1, extra.2, extra.0
        );
    }
}
