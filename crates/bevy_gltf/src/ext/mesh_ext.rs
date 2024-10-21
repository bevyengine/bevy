use bevy_core::Name;

use crate::GltfAssetLabel;

/// [`Mesh`](gltf::Mesh) extension
pub trait MeshExt {
    /// Create a [`GltfAssetLabel`] for the [`Mesh`](gltf::Mesh).
    fn to_label(&self) -> GltfAssetLabel;

    /// Generate the [`Name`] for a [`Primitive`](gltf::Primitive)
    fn primitive_name(&self, primitive: &gltf::Primitive) -> Name;
}

impl MeshExt for gltf::Mesh<'_> {
    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Mesh(self.index())
    }

    fn primitive_name(&self, primitive: &gltf::Primitive) -> Name {
        let mesh_name = self.name().unwrap_or("Mesh");
        if self.primitives().len() > 1 {
            format!("{}.{}", mesh_name, primitive.index()).into()
        } else {
            mesh_name.to_string().into()
        }
    }
}
