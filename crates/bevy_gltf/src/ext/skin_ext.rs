use bevy_asset::{Handle, LoadContext};
use bevy_render::mesh::skinning::SkinnedMeshInverseBindposes;

use crate::{GltfAssetLabel, GltfSkin};

use super::{ExtrasExt, NodeExt};

/// [`Skin`](gltf::Skin) extension
pub trait SkinExt {
    fn load_skin(
        &self,
        load_context: &mut LoadContext,
        skinned_mesh_inverse_bindposes: &[Handle<SkinnedMeshInverseBindposes>],
    ) -> GltfSkin;

    /// Create a [`GltfAssetLabel`] for the [`Skin`](gltf::Skin)
    fn to_label(&self) -> GltfAssetLabel;

    /// Create a [`GltfAssetLabel`] for the [`Skin`](gltf::Skin)
    /// as a `InverseBindMatrices`
    fn inverse_bind_matrices_label(&self) -> GltfAssetLabel;
}

impl SkinExt for gltf::Skin<'_> {
    fn load_skin(
        &self,
        load_context: &mut LoadContext,
        skinned_mesh_inverse_bindposes: &[Handle<SkinnedMeshInverseBindposes>],
    ) -> GltfSkin {
        let joints = self
            .joints()
            .map(|joint| load_context.get_label_handle(joint.to_label().to_string()))
            .collect();

        GltfSkin::new(
            self,
            joints,
            skinned_mesh_inverse_bindposes[self.index()].clone(),
            self.extras().get(),
        )
    }

    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Skin(self.index())
    }

    fn inverse_bind_matrices_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::InverseBindMatrices(self.index())
    }
}
