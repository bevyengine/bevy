use gltf::Skin;

use crate::GltfAssetLabel;

pub trait SkinExt {
    fn label(&self) -> GltfAssetLabel;
    fn inverse_bind_matrices_label(&self) -> GltfAssetLabel;
}

impl SkinExt for Skin<'_> {
    /// Return the label for the `skin`.
    fn label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Skin(self.index())
    }

    /// Return the label for the `inverseBindMatrices` of the node.
    fn inverse_bind_matrices_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::InverseBindMatrices(self.index())
    }
}
