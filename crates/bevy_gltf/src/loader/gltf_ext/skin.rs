use gltf::Skin;

use crate::GltfAssetLabel;

/// Return the label for the [`Skin`] as [`GltfAssetLabel::Skin`].
pub fn skin_label(skin: &Skin<'_>) -> GltfAssetLabel {
    GltfAssetLabel::Skin(skin.index())
}

/// Return the label for the [`Skin`] as [`GltfAssetLabel::InverseBindMatrices`].
pub fn inverse_bind_matrices_label(skin: &Skin<'_>) -> GltfAssetLabel {
    GltfAssetLabel::InverseBindMatrices(skin.index())
}
