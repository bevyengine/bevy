use bevy_asset::{Handle, LoadContext};
use bevy_pbr::StandardMaterial;
use bevy_utils::HashSet;

use crate::{GltfAssetLabel, GltfError, GltfLoader, GltfLoaderSettings, GltfPrimitive};

use super::primitive_ext::PrimitiveExt;

/// [`Mesh`](gltf::Mesh) extension
pub trait MeshExt {
    #[allow(clippy::too_many_arguments, clippy::result_large_err)]
    fn load_primitives(
        &self,
        load_context: &mut LoadContext,
        loader: &GltfLoader,
        settings: &GltfLoaderSettings,
        file_name: &str,
        buffer_data: &[Vec<u8>],
        meshes_on_skinned_nodes: &HashSet<usize>,
        meshes_on_non_skinned_nodes: &HashSet<usize>,
        materials: &[Handle<StandardMaterial>],
    ) -> Result<Vec<GltfPrimitive>, GltfError>;

    /// Create a [`GltfAssetLabel`] for the [`Mesh`](gltf::Mesh).
    fn to_label(&self) -> GltfAssetLabel;
}

impl MeshExt for gltf::Mesh<'_> {
    fn load_primitives(
        &self,
        load_context: &mut LoadContext,
        loader: &GltfLoader,
        settings: &GltfLoaderSettings,
        file_name: &str,
        buffer_data: &[Vec<u8>],
        meshes_on_skinned_nodes: &HashSet<usize>,
        meshes_on_non_skinned_nodes: &HashSet<usize>,
        materials: &[Handle<StandardMaterial>],
    ) -> Result<Vec<GltfPrimitive>, GltfError> {
        let mut primitives = vec![];
        for primitive in self.primitives() {
            primitives.push(primitive.load_primitive(
                load_context,
                loader,
                settings,
                file_name,
                buffer_data,
                self,
                meshes_on_skinned_nodes,
                meshes_on_non_skinned_nodes,
                materials,
            )?);
        }

        Ok(primitives)
    }

    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Mesh(self.index())
    }
}
