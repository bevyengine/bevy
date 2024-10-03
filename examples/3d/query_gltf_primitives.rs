//! This example demonstrates how to query a [`StandardMaterial`] within a glTF scene.
//! It is particularly useful for glTF scenes with a mesh that consists of multiple primitives.

use std::f32::consts::PI;

use bevy::{
    gltf::GltfMaterialName,
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    render::mesh::VertexAttributeValues,
};

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, find_top_material_and_mesh)
        .run();
}

fn find_top_material_and_mesh(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    mat_query: Query<(&Handle<StandardMaterial>, &Handle<Mesh>, &GltfMaterialName)>,
) {
    for (mat_handle, mesh_handle, name) in mat_query.iter() {
        // locate a material by material name
        if name.0 == "Top" {
            if let Some(material) = materials.get_mut(mat_handle) {
                if let Color::Hsla(ref mut hsla) = material.base_color {
                    *hsla = hsla.rotate_hue(time.delta_seconds() * 100.0);
                } else {
                    material.base_color = Color::from(Hsla::hsl(0.0, 0.8, 0.5));
                }
            }

            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                if let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    positions[0] = (
                        ops::sin(2.0 * PI * time.elapsed_seconds()),
                        positions[0][1],
                        positions[0][2],
                    )
                        .into();
                }
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.6, 1.6, 11.3)
                .looking_at(Vec3::new(0.0, 0.0, 3.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 500.0,
            rotation: Quat::IDENTITY,
        },
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // This is a relatively small scene, so use tighter shadow
        // cascade bounds than the default for better quality.
        // We also adjusted the shadow map to be larger since we're
        // only using a single cascade.
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 1.6,
            ..default()
        }
        .build(),
    ));
    commands.spawn((
        SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/GltfPrimitives/gltf_primitives.glb"),
        )),
        Transform {
            rotation: Quat::from_rotation_y(-90.0 / 180.0 * PI),
            ..default()
        },
    ));
}
