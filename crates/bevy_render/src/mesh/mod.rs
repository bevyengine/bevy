#[allow(clippy::module_inception)]
mod mesh;
/// Generation for some primitive shape meshes.
pub mod shape;

pub use mesh::*;

use crate::renderer::RenderDevice;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query, Res},
};

/// Adds the [`Mesh`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Mesh>()
            .add_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_type::<skinning::SkinnedMesh>()
            .register_type::<Vec<Entity>>()
            .add_systems(PostUpdate, transfer_meshes_to_gpu);
    }
}

/// Converts Handle<Mesh> components into GpuMesh components.
///
/// This will remove the handle. If the handle is the last
/// handle to the asset, the mesh will be unloaded from the cpu.
pub fn transfer_meshes_to_gpu(
    query: Query<(Entity, &Handle<Mesh>)>,
    meshes: Res<Assets<Mesh>>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, mesh_handle) in &query {
        if let Some(mesh) = meshes.get(mesh_handle) {
            commands
                .entity(entity)
                .insert(mesh.as_gpu(&render_device))
                .remove::<Handle<Mesh>>();
        }
    }
}
