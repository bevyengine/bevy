#[allow(clippy::module_inception)]
mod mesh;
pub mod morph;
pub mod primitives;

use bevy_utils::HashSet;
pub use mesh::*;
pub use primitives::*;
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{render_asset::RenderAssetPlugin, texture::GpuImage, RenderApp};
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::{entity::Entity, system::Resource};

/// Adds the [`Mesh`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_asset_reflect::<Mesh>()
            .register_type::<skinning::SkinnedMesh>()
            .register_type::<Vec<Entity>>()
            // 'Mesh' must be prepared after 'Image' as meshes rely on the morph target image being ready
            .add_plugins(RenderAssetPlugin::<GpuMesh, GpuImage>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<MeshVertexBufferLayouts>();
    }
}

/// Describes the layout of the mesh vertices in GPU memory.
///
/// At most one copy of a mesh vertex buffer layout ever exists in GPU memory at
/// once. Therefore, comparing these for equality requires only a single pointer
/// comparison, and this type's [`PartialEq`] and [`Hash`] implementations take
/// advantage of this. To that end, this type doesn't implement
/// [`bevy_derive::Deref`] or [`bevy_derive::DerefMut`] in order to reduce the
/// possibility of accidental deep comparisons, which would be needlessly
/// expensive.
#[derive(Clone, Debug)]
pub struct MeshVertexBufferLayoutRef(pub Arc<MeshVertexBufferLayout>);

/// Stores the single copy of each mesh vertex buffer layout.
#[derive(Clone, Default, Resource)]
pub struct MeshVertexBufferLayouts(HashSet<Arc<MeshVertexBufferLayout>>);

impl MeshVertexBufferLayouts {
    /// Inserts a new mesh vertex buffer layout in the store and returns a
    /// reference to it, reusing the existing reference if this mesh vertex
    /// buffer layout was already in the store.
    pub fn insert(&mut self, layout: MeshVertexBufferLayout) -> MeshVertexBufferLayoutRef {
        // Because the special `PartialEq` and `Hash` implementations that
        // compare by pointer are on `MeshVertexBufferLayoutRef`, not on
        // `Arc<MeshVertexBufferLayout>`, this compares the mesh vertex buffer
        // structurally, not by pointer.
        MeshVertexBufferLayoutRef(
            self.0
                .get_or_insert_with(&layout, |layout| Arc::new(layout.clone()))
                .clone(),
        )
    }
}

impl PartialEq for MeshVertexBufferLayoutRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for MeshVertexBufferLayoutRef {}

impl Hash for MeshVertexBufferLayoutRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the address of the underlying data, so two layouts that share the same
        // `MeshVertexBufferLayout` will have the same hash.
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}
