use gltf::Node;

use bevy_asset::{Handle, LoadContext};
use bevy_core::Name;
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::Transform;
use bevy_utils::{HashMap, HashSet};

use crate::{GltfAssetLabel, GltfNode};

use super::{ExtrasExt, MeshExt, SkinExt};

/// [`Node`] extension
pub trait NodeExt {
    fn load_node(
        &self,
        load_context: &mut LoadContext,
        unsorted_nodes: &mut HashMap<usize, Handle<GltfNode>>,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    ) -> GltfNode;

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

    fn paths_recur(
        &self,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    );
}

impl NodeExt for Node<'_> {
    fn load_node(
        &self,
        load_context: &mut LoadContext,
        unsorted_nodes: &mut HashMap<usize, Handle<GltfNode>>,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    ) -> GltfNode {
        let skin = self
            .skin()
            .map(|skin| load_context.get_label_handle(skin.to_label().to_string()));

        let children = self
            .children()
            .map(|child| unsorted_nodes.get(&child.index()).unwrap().clone())
            .collect();

        let mesh = self
            .mesh()
            .map(|mesh| load_context.get_label_handle(mesh.to_label().to_string()));

        let gltf_node = GltfNode::new(
            self,
            children,
            mesh,
            self.node_transform(),
            skin,
            self.extras().get(),
        );

        #[cfg(feature = "bevy_animation")]
        let gltf_node = gltf_node.with_animation_root(animation_roots.contains(&self.index()));

        gltf_node
    }

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

    fn paths_recur(
        &self,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    ) {
        let mut path = current_path.to_owned();
        path.push(self.to_name());
        visited.insert(self.index());
        for child in self.children() {
            if !visited.contains(&child.index()) {
                child.paths_recur(&path, paths, root_index, visited);
            }
        }
        paths.insert(self.index(), (root_index, path));
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
