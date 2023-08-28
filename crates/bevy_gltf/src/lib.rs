#![allow(clippy::type_complexity)]

#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationClip;
use bevy_utils::HashMap;

mod loader;
mod vertex_attributes;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Handle};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_pbr::StandardMaterial;
use bevy_reflect::{Reflect, TypePath, TypeUuid};
use bevy_render::{
    mesh::{Mesh, MeshVertexAttribute},
    renderer::RenderDevice,
    texture::CompressedImageFormats,
};
use bevy_scene::Scene;

/// Adds support for glTF file loading to the app.
#[derive(Default)]
pub struct GltfPlugin {
    custom_vertex_attributes: HashMap<String, MeshVertexAttribute>,
}

impl GltfPlugin {
    pub fn add_custom_vertex_attribute(
        mut self,
        name: &str,
        attribute: MeshVertexAttribute,
    ) -> Self {
        self.custom_vertex_attributes
            .insert(name.to_string(), attribute);
        self
    }
}

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GltfExtras>()
            .add_asset::<Gltf>()
            .add_asset::<GltfNode>()
            .add_asset::<GltfPrimitive>()
            .add_asset::<GltfMesh>()
            .preregister_asset_loader(&["gltf", "glb"]);
    }

    fn finish(&self, app: &mut App) {
        let supported_compressed_formats = match app.world.get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),

            None => CompressedImageFormats::NONE,
        };
        app.add_asset_loader::<GltfLoader>(GltfLoader {
            supported_compressed_formats,
            custom_vertex_attributes: self.custom_vertex_attributes.clone(),
        });
    }
}

/// Representation of a loaded glTF file
/// (file loaded via the `AssetServer` with the extension `.glb` or `.gltf`).
#[derive(Debug, TypeUuid, TypePath)]
#[uuid = "5c7d5f8a-f7b0-4e45-a09e-406c0372fea2"]
pub struct Gltf {
    pub scenes: Vec<Handle<Scene>>,
    pub named_scenes: HashMap<String, Handle<Scene>>,
    pub meshes: Vec<Handle<GltfMesh>>,
    pub named_meshes: HashMap<String, Handle<GltfMesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
    pub named_materials: HashMap<String, Handle<StandardMaterial>>,
    pub nodes: Vec<Handle<GltfNode>>,
    pub named_nodes: HashMap<String, Handle<GltfNode>>,
    pub default_scene: Option<Handle<Scene>>,
    #[cfg(feature = "bevy_animation")]
    pub animations: Vec<Handle<AnimationClip>>,
    #[cfg(feature = "bevy_animation")]
    pub named_animations: HashMap<String, Handle<AnimationClip>>,
}

/// A glTF node with all of its child nodes, its [`GltfMesh`],
/// [`Transform`](bevy_transform::prelude::Transform) and an optional [`GltfExtras`].
#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "dad74750-1fd6-460f-ac51-0a7937563865"]
pub struct GltfNode {
    pub children: Vec<GltfNode>,
    pub mesh: Option<Handle<GltfMesh>>,
    pub transform: bevy_transform::prelude::Transform,
    pub extras: Option<GltfExtras>,
}

/// A glTF mesh, which may consist of multiple [`GltfPrimitives`](GltfPrimitive)
/// and an optional [`GltfExtras`].
#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "8ceaec9a-926a-4f29-8ee3-578a69f42315"]
pub struct GltfMesh {
    pub primitives: Vec<GltfPrimitive>,
    pub extras: Option<GltfExtras>,
}

/// Part of a [`GltfMesh`] that consists of a [`Mesh`], an optional [`StandardMaterial`] and [`GltfExtras`].
#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "cbfca302-82fd-41cb-af77-cab6b3d50af1"]
pub struct GltfPrimitive {
    pub mesh: Handle<Mesh>,
    pub material: Option<Handle<StandardMaterial>>,
    pub extras: Option<GltfExtras>,
    pub material_extras: Option<GltfExtras>,
}

#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component)]
pub struct GltfExtras {
    pub value: String,
}
