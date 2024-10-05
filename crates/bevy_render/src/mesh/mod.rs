#[allow(clippy::module_inception)]
mod mesh;

pub mod allocator;
pub mod bounding;
mod components;
mod conversions;
mod index;
mod mikktspace;
pub mod morph;
pub mod primitives;
mod render_mesh;
pub mod skinning;
mod vertex;
use allocator::MeshAllocatorPlugin;
pub use components::{Mesh2d, Mesh3d};
pub use index::*;
pub use mesh::*;
pub use mikktspace::*;
pub use primitives::*;
pub use render_mesh::*;
pub use vertex::*;

use crate::{render_asset::RenderAssetPlugin, texture::GpuImage, RenderApp};
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::entity::Entity;

/// Adds the [`Mesh`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_asset_reflect::<Mesh>()
            .register_type::<Mesh3d>()
            .register_type::<skinning::SkinnedMesh>()
            .register_type::<Vec<Entity>>()
            // 'Mesh' must be prepared after 'Image' as meshes rely on the morph target image being ready
            .add_plugins(RenderAssetPlugin::<RenderMesh, GpuImage>::default())
            .add_plugins(MeshAllocatorPlugin);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<MeshVertexBufferLayouts>();
    }
}
