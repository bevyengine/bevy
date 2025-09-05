//! This example demonstrates how to query a [`StandardMaterial`] within a glTF scene.
//! It is particularly useful for glTF scenes with a mesh that consists of multiple primitives.

use std::f32::consts::PI;

use bevy::{gltf::GltfMaterialName, mesh::VertexAttributeValues, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, find_top_material_and_mesh)
        .run();
}

fn find_top_material_and_mesh(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    mat_query: Query<(
        &MeshMaterial3d<StandardMaterial>,
        &Mesh3d,
        &GltfMaterialName,
    )>,
) {
    for (mat_handle, mesh_handle, name) in mat_query.iter() {
        // locate a material by material name
        if name.0 == "Top" {
            if let Some(material) = materials.get_mut(mat_handle) {
                if let Color::Hsla(ref mut hsla) = material.base_color {
                    *hsla = hsla.rotate_hue(time.delta_secs() * 100.0);
                } else {
                    material.base_color = Color::from(Hsla::hsl(0.0, 0.9, 0.7));
                }
            }

            if let Some(mesh) = meshes.get_mut(mesh_handle)
                && let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
            {
                for position in positions {
                    *position = (
                        position[0],
                        1.5 + 0.5 * ops::sin(time.elapsed_secs() / 2.0),
                        position[2],
                    )
                        .into();
                }
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(4.0, 4.0, 12.0).looking_at(Vec3::new(0.0, 0.0, 0.5), Vec3::Y),
    ));

    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight::default(),
    ));

    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/GltfPrimitives/gltf_primitives.glb"),
    )));
}
