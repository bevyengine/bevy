use gltf::Scene;

use crate::GltfAssetLabel;

pub trait SceneExt {
    fn label(&self) -> GltfAssetLabel;
}

impl SceneExt for Scene<'_> {
    /// Returns the label for the `scene`.
    fn label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Scene(self.index())
    }
}
