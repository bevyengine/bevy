use crate::GltfAssetLabel;

/// [`Skin`](gltf::Skin) extension
pub trait SkinExt {
    /// Create a [`GltfAssetLabel`] for the [`Skin`](gltf::Skin)
    fn to_label(&self) -> GltfAssetLabel;

    /// Create a [`GltfAssetLabel`] for the [`Skin`](gltf::Skin)
    /// as a `InverseBindMatrices`
    fn inverse_bind_matrices_label(&self) -> GltfAssetLabel;
}

impl SkinExt for gltf::Skin<'_> {
    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Skin(self.index())
    }

    fn inverse_bind_matrices_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::InverseBindMatrices(self.index())
    }
}
