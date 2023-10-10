//! GPU-driven meshlet-based rendering.

use bevy::{
    log::info,
    pbr::meshlet::{MeshletMesh, MeshletPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshletPlugin)
        .add_systems(Update, update)
        .run();
}

fn update(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshlet_meshes: ResMut<Assets<MeshletMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut dragon_meshlet_mesh_handle: Local<Handle<MeshletMesh>>,
    mut dragon_mesh_handle: Local<Handle<Mesh>>,
) {
    if dragon_mesh_handle.id() == AssetId::default() {
        info!("Loading dragon model...");
        *dragon_mesh_handle = asset_server.load("models/dragon.glb#Mesh0/Primitive0");
    }

    if dragon_meshlet_mesh_handle.id() == AssetId::default() {
        if let Some(dragon_mesh) = meshes.get_mut(&*dragon_mesh_handle) {
            dragon_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0]; dragon_mesh.count_vertices()],
            );
            dragon_mesh.generate_tangents().unwrap();

            info!("Calculating dragon meshlets...");
            *dragon_meshlet_mesh_handle =
                meshlet_meshes.add(MeshletMesh::from_mesh(&dragon_mesh).unwrap());
            info!("Dragon meshlets calculated");

            commands.spawn((dragon_meshlet_mesh_handle.clone(), SpatialBundle::default()));
        }
    }
}
