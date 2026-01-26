//! Subasset names that can be used to load part of a glTF

use bevy_asset::AssetPath;

/// Subasset names that can be used to load part of a glTF
///
/// You can use [`GltfSubassetName::from_asset`] to add it to an asset path
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_scene::prelude::*;
/// # use bevy_gltf::prelude::*;
///
/// fn load_gltf_scene(asset_server: Res<AssetServer>) {
///     let gltf_scene: Handle<Scene> = asset_server.load(GltfSubassetName::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
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
///     let gltf_scene: Handle<Scene> = asset_server.load(format!("models/FlightHelmet/FlightHelmet.gltf#{}", GltfSubassetName::Scene(0)));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfSubassetName {
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

impl core::fmt::Display for GltfSubassetName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GltfSubassetName::Scene(index) => f.write_str(&format!("Scene{index}")),
            GltfSubassetName::Node(index) => f.write_str(&format!("Node{index}")),
            GltfSubassetName::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            GltfSubassetName::Primitive { mesh, primitive } => {
                f.write_str(&format!("Mesh{mesh}/Primitive{primitive}"))
            }
            GltfSubassetName::MorphTarget { mesh, primitive } => {
                f.write_str(&format!("Mesh{mesh}/Primitive{primitive}/MorphTargets"))
            }
            GltfSubassetName::Texture(index) => f.write_str(&format!("Texture{index}")),
            GltfSubassetName::Material {
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
            GltfSubassetName::DefaultMaterial => f.write_str("DefaultMaterial"),
            GltfSubassetName::Animation(index) => f.write_str(&format!("Animation{index}")),
            GltfSubassetName::Skin(index) => f.write_str(&format!("Skin{index}")),
            GltfSubassetName::InverseBindMatrices(index) => {
                f.write_str(&format!("Skin{index}/InverseBindMatrices"))
            }
        }
    }
}

impl GltfSubassetName {
    /// Add this subasset name to an asset path.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::prelude::*;
    /// # use bevy_scene::prelude::*;
    /// # use bevy_gltf::prelude::*;
    ///
    /// fn load_gltf_scene(asset_server: Res<AssetServer>) {
    ///     let gltf_scene: Handle<Scene> = asset_server.load(GltfSubassetName::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
    /// }
    /// ```
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_subasset_name(self.to_string())
    }
}
