//! Labels that can be used to load part of a glTF

use bevy_asset::AssetPath;

/// Labels that can be used to load part of a glTF
///
/// You can use [`GltfAssetLabel::from_asset`] to add it to an asset path
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_scene::prelude::*;
/// # use bevy_gltf::prelude::*;
///
/// fn load_gltf_scene(asset_server: Res<AssetServer>) {
///     let gltf_scene: Handle<Scene> = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
/// }
/// ```
///
/// Or when formatting a string for the path
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_scene::prelude::*;
/// # use bevy_gltf::prelude::*;
///
/// fn load_gltf_scene(asset_server: Res<AssetServer>) {
///     let gltf_scene: Handle<Scene> = asset_server.load(format!("models/FlightHelmet/FlightHelmet.gltf#{}", GltfAssetLabel::Scene(0)));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfAssetLabel {
    /// `Scene{}`: glTF Scene as a Bevy [`Scene`](bevy_scene::Scene)
    Scene(usize),
    /// `Node{}`: glTF Node as a [`GltfNode`](crate::GltfNode)
    Node(usize),
    /// `Mesh{}`: glTF Mesh as a [`GltfMesh`](crate::GltfMesh)
    Mesh(usize),
    /// `Mesh{}/Primitive{}`: glTF Primitive as a Bevy [`Mesh`](bevy_mesh::Mesh)
    Primitive {
        /// Index of the mesh for this primitive
        mesh: usize,
        /// Index of this primitive in its parent mesh
        primitive: usize,
    },
    /// `Mesh{}/Primitive{}/MorphTargets`: Morph target animation data for a glTF Primitive
    /// as a Bevy [`Image`](bevy_image::prelude::Image)
    MorphTarget {
        /// Index of the mesh for this primitive
        mesh: usize,
        /// Index of this primitive in its parent mesh
        primitive: usize,
    },
    /// `Texture{}`: glTF Texture as a Bevy [`Image`](bevy_image::prelude::Image)
    Texture(usize),
    /// `Material{}`: glTF Material as a Bevy [`StandardMaterial`](bevy_pbr::StandardMaterial)
    Material {
        /// Index of this material
        index: usize,
        /// Used to set the [`Face`](bevy_render::render_resource::Face) of the material,
        /// useful if it is used with negative scale
        is_scale_inverted: bool,
    },
    /// `DefaultMaterial`: glTF's default Material as a
    /// Bevy [`StandardMaterial`](bevy_pbr::StandardMaterial)
    DefaultMaterial,
    /// `Animation{}`: glTF Animation as Bevy [`AnimationClip`](bevy_animation::AnimationClip)
    Animation(usize),
    /// `Skin{}`: glTF mesh skin as [`GltfSkin`](crate::GltfSkin)
    Skin(usize),
    /// `Skin{}/InverseBindMatrices`: glTF mesh skin matrices as Bevy
    /// [`SkinnedMeshInverseBindposes`](bevy_mesh::skinning::SkinnedMeshInverseBindposes)
    InverseBindMatrices(usize),
}

impl core::fmt::Display for GltfAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GltfAssetLabel::Scene(index) => f.write_str(&format!("Scene{index}")),
            GltfAssetLabel::Node(index) => f.write_str(&format!("Node{index}")),
            GltfAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            GltfAssetLabel::Primitive { mesh, primitive } => {
                f.write_str(&format!("Mesh{mesh}/Primitive{primitive}"))
            }
            GltfAssetLabel::MorphTarget { mesh, primitive } => {
                f.write_str(&format!("Mesh{mesh}/Primitive{primitive}/MorphTargets"))
            }
            GltfAssetLabel::Texture(index) => f.write_str(&format!("Texture{index}")),
            GltfAssetLabel::Material {
                index,
                is_scale_inverted,
            } => f.write_str(&format!(
                "Material{index}{}",
                if *is_scale_inverted {
                    " (inverted)"
                } else {
                    ""
                }
            )),
            GltfAssetLabel::DefaultMaterial => f.write_str("DefaultMaterial"),
            GltfAssetLabel::Animation(index) => f.write_str(&format!("Animation{index}")),
            GltfAssetLabel::Skin(index) => f.write_str(&format!("Skin{index}")),
            GltfAssetLabel::InverseBindMatrices(index) => {
                f.write_str(&format!("Skin{index}/InverseBindMatrices"))
            }
        }
    }
}

impl GltfAssetLabel {
    /// Add this label to an asset path
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::prelude::*;
    /// # use bevy_scene::prelude::*;
    /// # use bevy_gltf::prelude::*;
    ///
    /// fn load_gltf_scene(asset_server: Res<AssetServer>) {
    ///     let gltf_scene: Handle<Scene> = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
    /// }
    /// ```
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}
