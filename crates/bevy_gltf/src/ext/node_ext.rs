use gltf::Node;

use bevy_core::Name;
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::Transform;

use crate::GltfAssetLabel;

/// [`Node`] extension
pub trait NodeExt {
    /// Calculate the transform of gLTF node.
    ///
    /// This should be used instead of calling [`gltf::scene::Transform::matrix()`]
    /// on [`Node::transform()`] directly because it uses optimized glam types and
    /// if `libm` feature of `bevy_math` crate is enabled also handles cross
    /// platform determinism properly.
    fn node_transform(&self) -> Transform;

    /// Create a [`GltfAssetLabel`] for the [`Node`]
    fn to_label(&self) -> GltfAssetLabel;

    /// Create a [`Name`] for the [`Node`]
    fn to_name(&self) -> Name;

    /// Check if node is skinned
    fn is_skinned(&self) -> bool;

    /// Get index of [`Mesh`](gltf::Mesh) on [`Node`]
    fn mesh_index(&self) -> Option<usize>;
}

impl NodeExt for Node<'_> {
    fn node_transform(&self) -> Transform {
        match self.transform() {
            gltf::scene::Transform::Matrix { matrix } => {
                Transform::from_matrix(Mat4::from_cols_array_2d(&matrix))
            }
            gltf::scene::Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => Transform {
                translation: Vec3::from(translation),
                rotation: bevy_math::Quat::from_array(rotation),
                scale: Vec3::from(scale),
            },
        }
    }

    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Node(self.index())
    }

    fn to_name(&self) -> Name {
        let name = self
            .name()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("GltfNode{}", self.index()));
        Name::new(name)
    }

    fn is_skinned(&self) -> bool {
        self.skin().is_some()
    }

    fn mesh_index(&self) -> Option<usize> {
        self.mesh().map(|mesh_info| mesh_info.index())
    }
}
