#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Plugin providing an [`AssetLoader`](bevy_asset::AssetLoader) and type definitions
//! for loading glTF 2.0 (a standard 3D scene definition format) files in Bevy.
//!
//! The [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) defines the format of the glTF files.
//!
//! # Quick Start
//!
//! Here's how to spawn a simple glTF scene
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! # use bevy_asset::prelude::*;
//! # use bevy_scene::prelude::*;
//! # use bevy_transform::prelude::*;
//! # use bevy_gltf::prelude::*;
//!
//! fn spawn_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn((
//!         // This is equivalent to "models/FlightHelmet/FlightHelmet.gltf#Scene0"
//!         // The `#Scene0` label here is very important because it tells bevy to load the first scene in the glTF file.
//!         // If this isn't specified bevy doesn't know which part of the glTF file to load.
//!         SceneRoot(asset_server.load(GltfAssetLabel::scene("0").from_asset("models/FlightHelmet/FlightHelmet.gltf"))),
//!         // You can use the transform to give it a position
//!         Transform::from_xyz(2.0, 0.0, -5.0),
//!     ));
//! }
//! ```
//! # Loading parts of a glTF asset
//!
//! ## Using `Gltf`
//!
//! If you want to access part of the asset, you can load the entire `Gltf` using the `AssetServer`.
//! Once the `Handle<Gltf>` is loaded you can then use it to access named parts of it.
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! # use bevy_asset::prelude::*;
//! # use bevy_scene::prelude::*;
//! # use bevy_transform::prelude::*;
//! # use bevy_gltf::Gltf;
//!
//! // Holds the scene handle
//! #[derive(Resource)]
//! struct HelmetScene(Handle<Gltf>);
//!
//! fn load_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     let gltf = asset_server.load("models/FlightHelmet/FlightHelmet.gltf");
//!     commands.insert_resource(HelmetScene(gltf));
//! }
//!
//! fn spawn_gltf_objects(
//!     mut commands: Commands,
//!     helmet_scene: Res<HelmetScene>,
//!     gltf_assets: Res<Assets<Gltf>>,
//!     mut loaded: Local<bool>,
//! ) {
//!     // Only do this once
//!     if *loaded {
//!         return;
//!     }
//!     // Wait until the scene is loaded
//!     let Some(gltf) = gltf_assets.get(&helmet_scene.0) else {
//!         return;
//!     };
//!     *loaded = true;
//!
//!     // Spawns the first scene in the file
//!     commands.spawn(SceneRoot(gltf.scenes[0].clone()));
//!
//!     // Spawns the scene named "Lenses_low"
//!     commands.spawn((
//!         SceneRoot(gltf.named_scenes["Lenses_low"].clone()),
//!         Transform::from_xyz(1.0, 2.0, 3.0),
//!     ));
//! }
//! ```
//!
//! ## Asset Labels
//!
//! The glTF loader let's you specify labels that let you target specific parts of the glTF.
//!
//! Be careful when using this feature, if you misspell a label it will simply ignore it without warning.
//!
//! You can use [`GltfAssetLabel`] to ensure you are using the correct label.

extern crate alloc;

use alloc::borrow::Cow;

#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationClip;
use bevy_utils::HashMap;

mod loader;
mod vertex_attributes;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp, AssetPath, Handle};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_image::CompressedImageFormats;
use bevy_pbr::StandardMaterial;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath};
use bevy_render::{
    mesh::{skinning::SkinnedMeshInverseBindposes, Mesh, MeshVertexAttribute},
    renderer::RenderDevice,
};
use bevy_scene::Scene;

/// The glTF prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{Gltf, GltfAssetLabel, GltfExtras};
}

/// Adds support for glTF file loading to the app.
#[derive(Default)]
pub struct GltfPlugin {
    custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
}

impl GltfPlugin {
    /// Register a custom vertex attribute so that it is recognized when loading a glTF file with the [`GltfLoader`].
    ///
    /// `name` must be the attribute name as found in the glTF data, which must start with an underscore.
    /// See [this section of the glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview)
    /// for additional details on custom attributes.
    pub fn add_custom_vertex_attribute(
        mut self,
        name: &str,
        attribute: MeshVertexAttribute,
    ) -> Self {
        self.custom_vertex_attributes.insert(name.into(), attribute);
        self
    }
}

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GltfExtras>()
            .register_type::<GltfSceneExtras>()
            .register_type::<GltfMeshExtras>()
            .register_type::<GltfMaterialExtras>()
            .register_type::<GltfMaterialName>()
            .init_asset::<Gltf>()
            .init_asset::<GltfNode>()
            .init_asset::<GltfPrimitive>()
            .init_asset::<GltfMesh>()
            .init_asset::<GltfSkin>()
            .preregister_asset_loader::<GltfLoader>(&["gltf", "glb"]);
    }

    fn finish(&self, app: &mut App) {
        let supported_compressed_formats = match app.world().get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),
            None => CompressedImageFormats::NONE,
        };
        app.register_asset_loader(GltfLoader {
            supported_compressed_formats,
            custom_vertex_attributes: self.custom_vertex_attributes.clone(),
        });
    }
}

fn gltf_name_or_index_string(name: Option<&str>, index: usize) -> String {
    name.map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{}", index))
}

/// Representation of a loaded glTF file.
#[derive(Asset, Debug, TypePath)]
pub struct Gltf {
    /// All scenes loaded from the glTF file.
    pub scenes: Vec<Handle<Scene>>,
    /// Named scenes loaded from the glTF file.
    pub named_scenes: HashMap<Box<str>, Handle<Scene>>,
    /// All meshes loaded from the glTF file.
    pub meshes: Vec<Handle<GltfMesh>>,
    /// Named meshes loaded from the glTF file.
    pub named_meshes: HashMap<Box<str>, Handle<GltfMesh>>,
    /// All materials loaded from the glTF file.
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Named materials loaded from the glTF file.
    pub named_materials: HashMap<Box<str>, Handle<StandardMaterial>>,
    /// All nodes loaded from the glTF file.
    pub nodes: Vec<Handle<GltfNode>>,
    /// Named nodes loaded from the glTF file.
    pub named_nodes: HashMap<Box<str>, Handle<GltfNode>>,
    /// All skins loaded from the glTF file.
    pub skins: Vec<Handle<GltfSkin>>,
    /// Named skins loaded from the glTF file.
    pub named_skins: HashMap<Box<str>, Handle<GltfSkin>>,
    /// Default scene to be displayed.
    pub default_scene: Option<Handle<Scene>>,
    /// All animations loaded from the glTF file.
    #[cfg(feature = "bevy_animation")]
    pub animations: Vec<Handle<AnimationClip>>,
    /// Named animations loaded from the glTF file.
    #[cfg(feature = "bevy_animation")]
    pub named_animations: HashMap<Box<str>, Handle<AnimationClip>>,
    /// The gltf root of the gltf asset, see <https://docs.rs/gltf/latest/gltf/struct.Gltf.html>. Only has a value when `GltfLoaderSettings::include_source` is true.
    pub source: Option<gltf::Gltf>,
}

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
        GltfAssetLabel::Node(self.name.clone().into())
    }
}

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
    /// Inverse-bind matrices of this skin.
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

    /// Subasset label for this skin within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Skin(self.name.clone().into())
    }
}

/// A glTF mesh, which may consist of multiple [`GltfPrimitives`](GltfPrimitive)
/// and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfMesh {
    /// Index of the mesh inside the scene
    pub index: usize,
    /// Computed name for a mesh - either a user defined mesh name from gLTF or a generated name from index
    pub name: String,
    /// Primitives of the glTF mesh.
    pub primitives: Vec<GltfPrimitive>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl GltfMesh {
    /// Create a mesh extracting name and index from glTF def
    pub fn new(
        mesh: &gltf::Mesh,
        primitives: Vec<GltfPrimitive>,
        extras: Option<GltfExtras>,
    ) -> Self {
        Self {
            index: mesh.index(),
            name: gltf_name_or_index_string(mesh.name(), mesh.index()),
            primitives,
            extras,
        }
    }

    /// Subasset label for this mesh within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Mesh(self.name.clone().into())
    }
}

/// Part of a [`GltfMesh`] that consists of a [`Mesh`], an optional [`StandardMaterial`] and [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh-primitive).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfPrimitive {
    /// Index of the primitive inside the mesh
    pub index: usize,
    /// Name of the parent [`GltfMesh`] of this primitive
    pub parent_mesh_name: String,
    /// Topology to be rendered.
    pub mesh: Handle<Mesh>,
    /// Material to apply to the `mesh`.
    pub material: Option<Handle<StandardMaterial>>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
    /// Additional data of the `material`.
    pub material_extras: Option<GltfExtras>,
}

impl GltfPrimitive {
    /// Create a primitive extracting name and index from glTF def
    pub fn new(
        gltf_mesh: &gltf::Mesh,
        gltf_primitive: &gltf::Primitive,
        mesh: Handle<Mesh>,
        material: Option<Handle<StandardMaterial>>,
        extras: Option<GltfExtras>,
        material_extras: Option<GltfExtras>,
    ) -> Self {
        GltfPrimitive {
            index: gltf_primitive.index(),
            parent_mesh_name: gltf_name_or_index_string(gltf_mesh.name(), gltf_mesh.index()),
            mesh,
            material,
            extras,
            material_extras,
        }
    }

    /// Subasset label for this primitive within its parent [`GltfMesh`] within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Primitive {
            mesh: self.parent_mesh_name.clone().into(),
            primitive: self.index,
        }
    }
}

/// Additional untyped data that can be present on most glTF types at the primitive level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfExtras {
    /// Content of the extra data.
    pub value: String,
}

/// Additional untyped data that can be present on most glTF types at the scene level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfSceneExtras {
    /// Content of the extra data.
    pub value: String,
}

/// Additional untyped data that can be present on most glTF types at the mesh level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfMeshExtras {
    /// Content of the extra data.
    pub value: String,
}

/// Additional untyped data that can be present on most glTF types at the material level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfMaterialExtras {
    /// Content of the extra data.
    pub value: String,
}

/// The material name of a glTF primitive.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-material).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component)]
pub struct GltfMaterialName(pub String);

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
///     let gltf_scene: Handle<Scene> = asset_server.load(GltfAssetLabel::scene("0").from_asset("models/FlightHelmet/FlightHelmet.gltf"));
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
///     let gltf_scene: Handle<Scene> = asset_server.load(format!("models/FlightHelmet/FlightHelmet.gltf#{}", GltfAssetLabel::scene("0")));
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GltfAssetLabel {
    /// `Scene:{name}`: glTF Scene as a Bevy `Scene`
    Scene(Cow<'static, str>),
    /// `Node:{name}`: glTF Node as a `GltfNode`
    Node(Cow<'static, str>),
    /// `Mesh:{name}`: glTF Mesh as a `GltfMesh`
    Mesh(Cow<'static, str>),
    /// `Mesh:{name}/Primitive{id}`: glTF Primitive as a Bevy `Mesh`
    Primitive {
        /// Index of the mesh for this primitive
        mesh: Cow<'static, str>,
        /// Index of this primitive in its parent mesh
        primitive: usize,
    },
    /// `Mesh:{name}/Primitive{id}/MorphTargets`: Morph target animation data for a glTF Primitive
    MorphTarget {
        /// Index of the mesh for this primitive
        mesh: Cow<'static, str>,
        /// Index of this primitive in its parent mesh
        primitive: usize,
    },
    /// `Texture:{name}`: glTF Texture as a Bevy `Image`
    Texture(Cow<'static, str>),
    /// `Material:{name}`: glTF Material as a Bevy `StandardMaterial`
    Material {
        /// Index of this material
        name: Cow<'static, str>,
        /// Used to set the [`Face`](bevy_render::render_resource::Face) of the material, useful if it is used with negative scale
        is_scale_inverted: bool,
    },
    /// `DefaultMaterial`: as above, if the glTF file contains a default material with no index
    DefaultMaterial,
    /// `Animation:{name}`: glTF Animation as Bevy `AnimationClip`
    Animation(Cow<'static, str>),
    /// `Skin:{name}`: glTF mesh skin as `GltfSkin`
    Skin(Cow<'static, str>),
    /// `Skin:{name}/InverseBindMatrices`: glTF mesh skin matrices as Bevy `SkinnedMeshInverseBindposes`
    InverseBindMatrices(Cow<'static, str>),
}

impl core::fmt::Display for GltfAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GltfAssetLabel::Scene(name) => f.write_str(&format!("Scene:{name}")),
            GltfAssetLabel::Node(name) => f.write_str(&format!("Node:{name}")),
            GltfAssetLabel::Mesh(name) => f.write_str(&format!("Mesh:{name}")),
            GltfAssetLabel::Primitive { mesh, primitive } => {
                f.write_str(&format!("Mesh:{mesh}/Primitive{primitive}"))
            }
            GltfAssetLabel::MorphTarget { mesh, primitive } => {
                f.write_str(&format!("Mesh:{mesh}/Primitive{primitive}/MorphTargets"))
            }
            GltfAssetLabel::Texture(name) => f.write_str(name),
            GltfAssetLabel::Material {
                name,
                is_scale_inverted,
            } => {
                f.write_str(&format!("Texture:{name}"))?;
                f.write_str(if *is_scale_inverted {
                    " (inverted)"
                } else {
                    ""
                })
            }
            GltfAssetLabel::DefaultMaterial => f.write_str("DefaultMaterial"),
            GltfAssetLabel::Animation(name) => f.write_str(&format!("Animation:{name}")),
            GltfAssetLabel::Skin(name) => f.write_str(&format!("Skin:{name}")),
            GltfAssetLabel::InverseBindMatrices(name) => {
                f.write_str(&format!("Skin:{name}/InverseBindMatrices"))
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
    ///     let gltf_scene: Handle<Scene> = asset_server.load(GltfAssetLabel::scene("0").from_asset("models/FlightHelmet/FlightHelmet.gltf"));
    /// }
    /// ```
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }

    /// Helper method for creating gltf scene asset label
    pub fn scene(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Scene(name.into())
    }

    /// Helper method for creating gltf node asset label
    pub fn node(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Node(name.into())
    }

    /// Helper method for creating gltf mesh asset label
    pub fn mesh(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Mesh(name.into())
    }

    /// Helper method for creating gltf primitive asset label
    pub fn primitive(mesh: impl Into<Cow<'static, str>>, primitive_index: usize) -> Self {
        Self::Primitive {
            mesh: mesh.into(),
            primitive: primitive_index,
        }
    }

    /// Helper method for creating gltf morph target asset label
    pub fn morph_target(mesh: impl Into<Cow<'static, str>>, primitive_index: usize) -> Self {
        Self::MorphTarget {
            mesh: mesh.into(),
            primitive: primitive_index,
        }
    }

    /// Helper method for creating gltf material asset label
    pub fn material(name: impl Into<Cow<'static, str>>, is_scale_inverted: bool) -> Self {
        Self::Material {
            name: name.into(),
            is_scale_inverted,
        }
    }

    /// Helper method for creating gltf animation asset label
    pub fn animation(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Animation(name.into())
    }

    /// Helper method for creating gltf skin asset label
    pub fn skin(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Skin(name.into())
    }

    /// Helper method for creating gltf inverse bind matrices asset label
    pub fn inverse_bind_matrices(mesh: impl Into<Cow<'static, str>>) -> Self {
        Self::InverseBindMatrices(mesh.into())
    }
}
