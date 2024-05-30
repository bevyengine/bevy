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

use std::pin::Pin;

#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationClip;
use bevy_hierarchy::WorldChildBuilder;
use bevy_pbr::StandardMaterial;
use bevy_utils::{ConditionalSendFuture, HashMap};

mod loader;
mod material;
mod vertex_attributes;

use gltf::{Document, Material};
pub use loader::*;
pub use material::FromStandardMaterial;

use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp, Handle, LoadContext, UntypedHandle};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent, world::EntityWorldMut};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    mesh::{Mesh, MeshVertexAttribute},
    renderer::RenderDevice,
    texture::CompressedImageFormats,
};
use bevy_scene::Scene;

pub(crate) type LoaderFn =
    for<'t> fn(
        &'t GltfLoader,
        &'t [u8],
        &'t mut LoadContext<'_>,
        &'t GltfLoaderSettings,
    ) -> Pin<Box<dyn ConditionalSendFuture<Output = Result<Gltf, GltfError>> + 't>>;

pub(crate) type LoadMaterialFn = for<'t> fn(
    &'t Material,
    &'t mut LoadContext<'_>,
    &'t Document,
    bool,
) -> (UntypedHandle, MaterialMeshBundleSpawner);

pub(crate) type MaterialMeshBundleSpawner = for<'t> fn(
    &'t mut WorldChildBuilder<'_>,
    &'t mut LoadContext<'_>,
    String,
    String,
) -> EntityWorldMut<'t>;

/// Adds support for glTF file loading to the app.
pub struct GltfPlugin {
    custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
    default_loader: LoaderFn,
    loaders: HashMap<String, LoadMaterialFn>,
}

impl Default for GltfPlugin {
    fn default() -> Self {
        Self {
            custom_vertex_attributes: Default::default(),
            loaders: Default::default(),
            default_loader: |loader, bytes, load_context, settings| {
                Box::pin(load_gltf::<StandardMaterial>(
                    loader,
                    bytes,
                    load_context,
                    settings,
                ))
            },
        }
    }
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

    /// Replace [`StandardMaterial`](bevy_pbr::StandardMaterial) as the default material loaded by [`GltfLoader`].
    pub fn with_standard_material<M: FromStandardMaterial + bevy_pbr::Material>(mut self) -> Self {
        self.default_loader = |loader, bytes, load_context, settings| {
            Box::pin(load_gltf::<M>(loader, bytes, load_context, settings))
        };
        self
    }

    /// Register a [`FromStandardMaterial`] material that can be created from the `Gltf` specification.
    /// If a material has `gltf_extras` attribute `{ "material": "name" }`, will use this material instead.
    pub fn add_material<M: FromStandardMaterial + bevy_pbr::Material>(
        mut self,
        name: &str,
    ) -> Self {
        self.loaders.insert(name.into(), load_material_inner::<M>);
        self
    }
}

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GltfExtras>()
            .init_asset::<Gltf>()
            .init_asset::<GltfNode>()
            .init_asset::<GltfPrimitive>()
            .init_asset::<GltfMesh>()
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
            loaders: self.loaders.clone(),
            default_loader: self.default_loader,
        });
    }
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
    pub materials: Vec<UntypedHandle>,
    /// Named materials loaded from the glTF file.
    pub named_materials: HashMap<Box<str>, UntypedHandle>,
    /// All nodes loaded from the glTF file.
    pub nodes: Vec<Handle<GltfNode>>,
    /// Named nodes loaded from the glTF file.
    pub named_nodes: HashMap<Box<str>, Handle<GltfNode>>,
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

impl Gltf {
    /// Try downcast and obtain a typed [`Handle`] of a material.
    pub fn get_material<T: Asset>(&self, name: &str) -> Option<Handle<T>> {
        self.named_materials
            .get(name)
            .and_then(|x| x.clone().try_into().ok())
    }

    /// Iterate through all [`Handle`]s of a given material.
    pub fn iter_materials<T: Asset>(&self) -> impl Iterator<Item = Handle<T>> + '_ {
        self.materials
            .iter()
            .filter_map(|handle| handle.clone().try_into().ok())
    }

    /// Iterate through all named [`Handle`]s of a given material.
    pub fn iter_name_materials<T: Asset>(&self) -> impl Iterator<Item = (&str, Handle<T>)> + '_ {
        self.named_materials
            .iter()
            .filter_map(|(name, handle)| Some((name.as_ref(), handle.clone().try_into().ok()?)))
    }
}

/// A glTF node with all of its child nodes, its [`GltfMesh`],
/// [`Transform`](bevy_transform::prelude::Transform) and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-node).
#[derive(Asset, Debug, TypePath)]
pub struct GltfNode {
    /// Direct children of the node.
    pub children: Vec<GltfNode>,
    /// Mesh of the node.
    pub mesh: Option<Handle<GltfMesh>>,
    /// Local transform.
    pub transform: bevy_transform::prelude::Transform,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl Clone for GltfNode {
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
pub struct GltfMesh {
    /// Primitives of the glTF mesh.
    pub primitives: Vec<GltfPrimitive>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl Clone for GltfMesh {
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
pub struct GltfPrimitive {
    /// Topology to be rendered.
    pub mesh: Handle<Mesh>,
    /// Material to apply to the `mesh`.
    pub material: Option<UntypedHandle>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
    /// Additional data of the `material`.
    pub material_extras: Option<GltfExtras>,
}

impl Clone for GltfPrimitive {
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
