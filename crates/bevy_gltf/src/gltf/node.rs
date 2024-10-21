use bevy_asset::{Asset, Handle, LoadContext};
use bevy_reflect::TypePath;
use bevy_utils::HashMap;
#[cfg(feature = "bevy_animation")]
use bevy_utils::HashSet;

use crate::{
    ext::{ExtrasExt, MeshExt, NodeExt, SkinExt},
    gltf_tree_iterator::GltfTreeIterator,
    GltfError,
};

use super::{GltfAssetLabel, GltfExtras, GltfMesh, GltfSkin};

/// A glTF node with all of its child nodes, its [`GltfMesh`],
/// [`Transform`](bevy_transform::prelude::Transform), its optional [`GltfSkin`]
/// and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-node).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfNode {
    /// Index of the node inside the scene
    pub index: usize,
    /// Computed name for a node - either a user defined node name from gLTF or a generated name from index
    pub name: String,
    /// Direct children of the node.
    pub children: Vec<Handle<GltfNode>>,
    /// Mesh of the node.
    pub mesh: Option<Handle<GltfMesh>>,
    /// Skin of the node.
    pub skin: Option<Handle<GltfSkin>>,
    /// Local transform.
    pub transform: bevy_transform::prelude::Transform,
    /// Is this node used as an animation root
    #[cfg(feature = "bevy_animation")]
    pub is_animation_root: bool,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl GltfNode {
    /// Create a node extracting name and index from glTF def
    pub fn new(
        node: &gltf::Node,
        children: Vec<Handle<GltfNode>>,
        mesh: Option<Handle<GltfMesh>>,
        transform: bevy_transform::prelude::Transform,
        skin: Option<Handle<GltfSkin>>,
        extras: Option<GltfExtras>,
    ) -> Self {
        Self {
            index: node.index(),
            name: if let Some(name) = node.name() {
                name.to_string()
            } else {
                format!("GltfNode{}", node.index())
            },
            children,
            mesh,
            transform,
            skin,
            #[cfg(feature = "bevy_animation")]
            is_animation_root: false,
            extras,
        }
    }

    #[allow(clippy::result_large_err)]
    /// Load all nodes of a [`glTF`](gltf::Gltf)
    pub(crate) fn load_nodes(
        load_context: &mut LoadContext,
        gltf: &gltf::Gltf,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    ) -> Result<(Vec<Handle<GltfNode>>, HashMap<Box<str>, Handle<GltfNode>>), GltfError> {
        let mut unsorted_nodes = HashMap::<usize, Handle<GltfNode>>::new();
        let mut named_nodes = HashMap::new();
        for node in GltfTreeIterator::try_new(gltf)? {
            let skin = node
                .skin()
                .map(|skin| load_context.get_label_handle(skin.to_label().to_string()));

            let children = node
                .children()
                .map(|child| unsorted_nodes.get(&child.index()).unwrap().clone())
                .collect();

            let mesh = node
                .mesh()
                .map(|mesh| load_context.get_label_handle(mesh.to_label().to_string()));

            let gltf_node = GltfNode::new(
                &node,
                children,
                mesh,
                node.node_transform(),
                skin,
                node.extras().get(),
            );

            #[cfg(feature = "bevy_animation")]
            let gltf_node = gltf_node.with_animation_root(animation_roots.contains(&node.index()));

            let handle =
                load_context.add_labeled_asset(gltf_node.asset_label().to_string(), gltf_node);
            unsorted_nodes.insert(node.index(), handle.clone());
            if let Some(name) = node.name() {
                named_nodes.insert(name.into(), handle);
            }
        }

        let mut nodes_to_sort = unsorted_nodes.into_iter().collect::<Vec<_>>();
        nodes_to_sort.sort_by_key(|(i, _)| *i);
        let nodes = nodes_to_sort
            .into_iter()
            .map(|(_, resolved)| resolved)
            .collect();

        Ok((nodes, named_nodes))
    }

    /// Create a node with animation root mark
    #[cfg(feature = "bevy_animation")]
    pub fn with_animation_root(self, is_animation_root: bool) -> Self {
        Self {
            is_animation_root,
            ..self
        }
    }

    /// Subasset label for this node within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Node(self.index)
    }
}
