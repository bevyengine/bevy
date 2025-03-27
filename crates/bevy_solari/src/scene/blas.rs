use bevy_asset::AssetId;
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_mesh::Mesh;
use bevy_platform_support::collections::HashMap;
use bevy_render::{
    mesh::{allocator::MeshAllocator, RenderMesh},
    render_asset::ExtractedAssets,
    render_resource::Blas,
    renderer::{RenderDevice, RenderQueue},
};

#[derive(Resource, Default)]
pub struct BlasManager(HashMap<AssetId<Mesh>, Blas>);

impl BlasManager {
    pub fn get(&self, mesh: &AssetId<Mesh>) -> Option<&Blas> {
        self.0.get(mesh)
    }
}

pub fn manage_blas(
    mut blas_manager: ResMut<BlasManager>,
    extracted_meshes: Res<ExtractedAssets<RenderMesh>>,
    mesh_allocator: Res<MeshAllocator>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
}
