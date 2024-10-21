use crate::GltfAssetLabel;

/// [`Scene`](gltf::Scene) extension
pub trait SceneExt {
    /// Create a [`GltfAssetLabel`] for the [`Scene`](gltf::Scene).
    fn to_label(&self) -> GltfAssetLabel;
}

impl SceneExt for gltf::Scene<'_> {
    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Scene(self.index())
    }
}
