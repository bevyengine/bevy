#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
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
//!         SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"))),
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

mod assets;
mod convert_coordinates;
mod label;
mod loader;
mod vertex_attributes;

extern crate alloc;

use alloc::sync::Arc;
use std::sync::Mutex;
use tracing::warn;

use bevy_platform::collections::HashMap;

use bevy_app::prelude::*;
use bevy_asset::AssetApp;
use bevy_ecs::prelude::Resource;
use bevy_image::{CompressedImageFormatSupport, CompressedImageFormats, ImageSamplerDescriptor};
use bevy_mesh::MeshVertexAttribute;

/// The glTF prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{assets::Gltf, assets::GltfExtras, label::GltfAssetLabel};
}

pub use {assets::*, label::GltfAssetLabel, loader::*};

// Has to store an Arc<Mutex<...>> as there is no other way to mutate fields of asset loaders.
/// Stores default [`ImageSamplerDescriptor`] in main world.
#[derive(Resource)]
pub struct DefaultGltfImageSampler(Arc<Mutex<ImageSamplerDescriptor>>);

impl DefaultGltfImageSampler {
    /// Creates a new [`DefaultGltfImageSampler`].
    pub fn new(descriptor: &ImageSamplerDescriptor) -> Self {
        Self(Arc::new(Mutex::new(descriptor.clone())))
    }

    /// Returns the current default [`ImageSamplerDescriptor`].
    pub fn get(&self) -> ImageSamplerDescriptor {
        self.0.lock().unwrap().clone()
    }

    /// Makes a clone of internal [`Arc`] pointer.
    ///
    /// Intended only to be used by code with no access to ECS.
    pub fn get_internal(&self) -> Arc<Mutex<ImageSamplerDescriptor>> {
        self.0.clone()
    }

    /// Replaces default [`ImageSamplerDescriptor`].
    ///
    /// Doesn't apply to samplers already built on top of it, i.e. `GltfLoader`'s output.
    /// Assets need to manually be reloaded.
    pub fn set(&self, descriptor: &ImageSamplerDescriptor) {
        *self.0.lock().unwrap() = descriptor.clone();
    }
}

/// Adds support for glTF file loading to the app.
pub struct GltfPlugin {
    /// The default image sampler to lay glTF sampler data on top of.
    ///
    /// Can be modified with the [`DefaultGltfImageSampler`] resource.
    pub default_sampler: ImageSamplerDescriptor,

    /// _CAUTION: This is an experimental feature with [known issues](https://github.com/bevyengine/bevy/issues/20621). Behavior may change in future versions._
    ///
    /// How to convert glTF coordinates on import. Assuming glTF cameras, glTF lights, and glTF meshes had global identity transforms,
    /// their Bevy [`Transform::forward`](bevy_transform::components::Transform::forward) will be pointing in the following global directions:
    /// - When set to `false`
    ///   - glTF cameras and glTF lights: global -Z,
    ///   - glTF models: global +Z.
    /// - When set to `true`
    ///   - glTF cameras and glTF lights: global +Z,
    ///   - glTF models: global -Z.
    ///
    /// The default is `false`.
    pub use_model_forward_direction: bool,

    /// Registry for custom vertex attributes.
    ///
    /// To specify, use [`GltfPlugin::add_custom_vertex_attribute`].
    pub custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
}

impl Default for GltfPlugin {
    fn default() -> Self {
        GltfPlugin {
            default_sampler: ImageSamplerDescriptor::linear(),
            custom_vertex_attributes: HashMap::default(),
            use_model_forward_direction: false,
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
}

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Gltf>()
            .init_asset::<GltfNode>()
            .init_asset::<GltfPrimitive>()
            .init_asset::<GltfMesh>()
            .init_asset::<GltfSkin>()
            .preregister_asset_loader::<GltfLoader>(&["gltf", "glb"]);
    }

    fn finish(&self, app: &mut App) {
        let supported_compressed_formats = if let Some(resource) =
            app.world().get_resource::<CompressedImageFormatSupport>()
        {
            resource.0
        } else {
            warn!("CompressedImageFormatSupport resource not found. It should either be initialized in finish() of \
            RenderPlugin, or manually if not using the RenderPlugin or the WGPU backend.");
            CompressedImageFormats::NONE
        };

        let default_sampler_resource = DefaultGltfImageSampler::new(&self.default_sampler);
        let default_sampler = default_sampler_resource.get_internal();
        app.insert_resource(default_sampler_resource);

        app.register_asset_loader(GltfLoader {
            supported_compressed_formats,
            custom_vertex_attributes: self.custom_vertex_attributes.clone(),
            default_sampler,
            default_use_model_forward_direction: self.use_model_forward_direction,
        });
    }
}
