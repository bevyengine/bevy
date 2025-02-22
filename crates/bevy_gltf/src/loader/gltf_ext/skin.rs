use gltf::Skin;

use crate::GltfAssetLabel;

/// Return the label for the [`Skin`] as [`GltfAssetLabel::Skin`].
pub(crate) fn skin_label(skin: &Skin<'_>) -> GltfAssetLabel {
    GltfAssetLabel::Skin(skin.index())
}

/// Return the label for the [`Skin`] as [`GltfAssetLabel::InverseBindMatrices`].
pub(crate) fn inverse_bind_matrices_label(skin: &Skin<'_>) -> GltfAssetLabel {
    GltfAssetLabel::InverseBindMatrices(skin.index())
}
