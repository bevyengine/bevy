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
//!
//! fn spawn_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn(SceneBundle {
//!         // The `#Scene0` label here is very important because it tells bevy to load the first scene in the glTF file.
//!         // If this isn't specified bevy doesn't know which part of the glTF file to load.
//!         scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
//!         // You can use the transform to give it a position
//!         transform: Transform::from_xyz(2.0, 0.0, -5.0),
//!         ..Default::default()
//!     });
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
//!     commands.spawn(SceneBundle {
//!         // Gets the first scene in the file
//!         scene: gltf.scenes[0].clone(),
//!         ..Default::default()
//!     });
//!
//!     commands.spawn(SceneBundle {
//!         // Gets the scene named "Lenses_low"
//!         scene: gltf.named_scenes["Lenses_low"].clone(),
//!         transform: Transform::from_xyz(1.0, 2.0, 3.0),
//!         ..Default::default()
//!     });
//! }
//! ```
//!
//! ## Asset Labels
//!
//! The glTF loader let's you specify labels that let you target specific parts of the glTF.
//!
//! Be careful when using this feature, if you misspell a label it will simply ignore it without warning.
//!
//! Here's the list of supported labels (`{}` is the index in the file):
//!
//! - `Scene{}`: glTF Scene as a Bevy `Scene`
//! - `Node{}`: glTF Node as a `GltfNode`
//! - `Mesh{}`: glTF Mesh as a `GltfMesh`
//! - `Mesh{}/Primitive{}`: glTF Primitive as a Bevy `Mesh`
//! - `Mesh{}/Primitive{}/MorphTargets`: Morph target animation data for a glTF Primitive
//! - `Texture{}`: glTF Texture as a Bevy `Image`
//! - `Material{}`: glTF Material as a Bevy `StandardMaterial`
//! - `DefaultMaterial`: as above, if the glTF file contains a default material with no index
//! - `Animation{}`: glTF Animation as Bevy `AnimationClip`
//! - `Skin{}`: glTF mesh skin as Bevy `SkinnedMeshInverseBindposes`

use std::marker::PhantomData;

#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationClip;
use bevy_utils::HashMap;

mod loader;
mod material;
mod vertex_attributes;

pub use loader::*;
pub use material::FromStandardMaterial;

use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp, Handle};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_pbr::StandardMaterial;
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    mesh::{Mesh, MeshVertexAttribute},
    renderer::RenderDevice,
    texture::CompressedImageFormats,
};
use bevy_scene::Scene;

/// Adds support for glTF file loading to the app.
pub struct GltfPlugin<M: FromStandardMaterial = StandardMaterial> {
    custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
    p: PhantomData<M>,
}

impl Default for GltfPlugin {
    fn default() -> Self {
        Self {
            custom_vertex_attributes: Default::default(),
            p: Default::default(),
        }
    }
}

impl GltfPlugin {
    /// Construct a [`GltfPlugin`] with a custom standard material.
    pub fn with_standard_material<M: FromStandardMaterial>() -> GltfPlugin<M> {
        GltfPlugin {
            custom_vertex_attributes: Default::default(),
            p: Default::default(),
        }
    }
}

impl<M: FromStandardMaterial> GltfPlugin<M> {
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

impl<M: FromStandardMaterial + bevy_pbr::Material> Plugin for GltfPlugin<M> {
    fn build(&self, app: &mut App) {
        app.register_type::<GltfExtras>()
            .init_asset::<Gltf<M>>()
            .init_asset::<GltfNode<M>>()
            .init_asset::<GltfPrimitive<M>>()
            .init_asset::<GltfMesh<M>>()
            .preregister_asset_loader::<GltfLoader<M>>(&["gltf", "glb"]);
    }

    fn finish(&self, app: &mut App) {
        let supported_compressed_formats = match app.world().get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),
            None => CompressedImageFormats::NONE,
        };
        app.register_asset_loader(GltfLoader {
            supported_compressed_formats,
            custom_vertex_attributes: self.custom_vertex_attributes.clone(),
            p: PhantomData::<M>,
        });
    }
}

/// Representation of a loaded glTF file.
#[derive(Asset, Debug, TypePath)]
pub struct Gltf<M: FromStandardMaterial = StandardMaterial> {
    /// All scenes loaded from the glTF file.
    pub scenes: Vec<Handle<Scene>>,
    /// Named scenes loaded from the glTF file.
    pub named_scenes: HashMap<Box<str>, Handle<Scene>>,
    /// All meshes loaded from the glTF file.
    pub meshes: Vec<Handle<GltfMesh<M>>>,
    /// Named meshes loaded from the glTF file.
    pub named_meshes: HashMap<Box<str>, Handle<GltfMesh<M>>>,
    /// All materials loaded from the glTF file.
    pub materials: Vec<Handle<M>>,
    /// Named materials loaded from the glTF file.
    pub named_materials: HashMap<Box<str>, Handle<M>>,
    /// All nodes loaded from the glTF file.
    pub nodes: Vec<Handle<GltfNode<M>>>,
    /// Named nodes loaded from the glTF file.
    pub named_nodes: HashMap<Box<str>, Handle<GltfNode<M>>>,
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
/// [`Transform`](bevy_transform::prelude::Transform) and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-node).
#[derive(Asset, Debug, TypePath)]
pub struct GltfNode<M: FromStandardMaterial = StandardMaterial> {
    /// Direct children of the node.
    pub children: Vec<GltfNode<M>>,
    /// Mesh of the node.
    pub mesh: Option<Handle<GltfMesh<M>>>,
    /// Local transform.
    pub transform: bevy_transform::prelude::Transform,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl<M: FromStandardMaterial> Clone for GltfNode<M> {
    fn clone(&self) -> Self {
        GltfNode {
            children: self.children.clone(),
            mesh: self.mesh.clone(),
            transform: self.transform,
            extras: self.extras.clone(),
        }
    }
}

/// A glTF mesh, which may consist of multiple [`GltfPrimitives`](GltfPrimitive)
/// and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh).
#[derive(Asset, Debug, TypePath)]
pub struct GltfMesh<M: FromStandardMaterial = StandardMaterial> {
    /// Primitives of the glTF mesh.
    pub primitives: Vec<GltfPrimitive<M>>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl<M: FromStandardMaterial> Clone for GltfMesh<M> {
    fn clone(&self) -> Self {
        GltfMesh {
            primitives: self.primitives.clone(),
            extras: self.extras.clone(),
        }
    }
}

/// Part of a [`GltfMesh`] that consists of a [`Mesh`], an optional [`StandardMaterial`] and [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh-primitive).
#[derive(Asset, Debug, TypePath)]
pub struct GltfPrimitive<M: FromStandardMaterial = StandardMaterial> {
    /// Topology to be rendered.
    pub mesh: Handle<Mesh>,
    /// Material to apply to the `mesh`.
    pub material: Option<Handle<M>>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
    /// Additional data of the `material`.
    pub material_extras: Option<GltfExtras>,
}

impl<M: FromStandardMaterial> Clone for GltfPrimitive<M> {
    fn clone(&self) -> Self {
        Self {
            mesh: self.mesh.clone(),
            material: self.material.clone(),
            extras: self.extras.clone(),
            material_extras: self.material_extras.clone(),
        }
    }
}

/// Additional untyped data that can be present on most glTF types.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component)]
pub struct GltfExtras {
    /// Content of the extra data.
    pub value: String,
}
