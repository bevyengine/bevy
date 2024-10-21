use bevy_asset::{Asset, Handle, LoadContext};
use bevy_reflect::TypePath;
use bevy_render::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy_utils::HashMap;

use crate::{
    ext::{ExtrasExt, GltfExt, NodeExt, SkinExt},
    gltf_tree_iterator::GltfTreeIterator,
    GltfError,
};

use super::{GltfAssetLabel, GltfExtras, GltfNode};

/// A glTF skin with all of its joint nodes, [`SkinnedMeshInversiveBindposes`](bevy_render::mesh::skinning::SkinnedMeshInverseBindposes)
/// and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-skin).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfSkin {
    /// Index of the skin inside the scene
    pub index: usize,
    /// Computed name for a skin - either a user defined skin name from gLTF or a generated name from index
    pub name: String,
    /// All the nodes that form this skin.
    pub joints: Vec<Handle<GltfNode>>,
    /// Inverse-bind matricy of this skin.
    pub inverse_bind_matrices: Handle<SkinnedMeshInverseBindposes>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl GltfSkin {
    /// Create a skin extracting name and index from glTF def
    pub fn new(
        skin: &gltf::Skin,
        joints: Vec<Handle<GltfNode>>,
        inverse_bind_matrices: Handle<SkinnedMeshInverseBindposes>,
        extras: Option<GltfExtras>,
    ) -> Self {
        Self {
            index: skin.index(),
            name: if let Some(name) = skin.name() {
                name.to_string()
            } else {
                format!("GltfSkin{}", skin.index())
            },
            joints,
            inverse_bind_matrices,
            extras,
        }
    }

    #[allow(clippy::result_large_err)]
    /// Load all skins of a [`glTF`](gltf::Gltf)
    pub(crate) fn load_skins(
        load_context: &mut LoadContext,
        gltf: &gltf::Gltf,
        buffer_data: &[Vec<u8>],
    ) -> Result<(Vec<Handle<GltfSkin>>, HashMap<Box<str>, Handle<GltfSkin>>), GltfError> {
        let mut skins = vec![];
        let mut named_skins = HashMap::default();

        let skinned_mesh_inverse_bindposes = gltf.inverse_bind_poses(load_context, buffer_data);

        for node in GltfTreeIterator::try_new(gltf)? {
            if let Some(skin) = node.skin() {
                let joints = skin
                    .joints()
                    .map(|joint| load_context.get_label_handle(joint.to_label().to_string()))
                    .collect();

                let gltf_skin = GltfSkin::new(
                    &skin,
                    joints,
                    skinned_mesh_inverse_bindposes[skin.index()].clone(),
                    skin.extras().get(),
                );

                let handle = load_context.add_labeled_asset(skin.to_label().to_string(), gltf_skin);

                skins.push(handle.clone());
                if let Some(name) = skin.name() {
                    named_skins.insert(name.into(), handle.clone());
                }
            }
        }

        Ok((skins, named_skins))
    }

    /// Subasset label for this skin within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Skin(self.index)
    }
}
