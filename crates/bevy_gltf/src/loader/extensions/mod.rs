//! glTF extensions defined by the Khronos Group and other vendors

mod khr_materials_anisotropy;
mod khr_materials_clearcoat;
mod khr_materials_specular;

use alloc::sync::Arc;
use async_lock::RwLock;

use bevy_asset::{Handle, LoadContext};
use bevy_ecs::{
    entity::Entity,
    resource::Resource,
    world::{EntityWorldMut, World},
};
use gltf::Node;

#[cfg(feature = "bevy_animation")]
use {
    bevy_animation::AnimationClip,
    bevy_platform::collections::{HashMap, HashSet},
};

use crate::{GltfMaterial, GltfMesh};

pub(crate) use self::{
    khr_materials_anisotropy::AnisotropyExtension, khr_materials_clearcoat::ClearcoatExtension,
    khr_materials_specular::SpecularExtension,
};

/// Stores the `GltfExtensionHandler` implementations so that they
/// can be added by users and also passed to the glTF loader
#[derive(Resource, Default)]
pub struct GltfExtensionHandlers(pub Arc<RwLock<Vec<Box<dyn GltfExtensionHandler>>>>);

/// glTF Extensions can attach data to any objects in a glTF file.
/// This is done by inserting data in the `extensions` sub-object, and
/// data in the extensions sub-object is keyed by the id of the extension.
/// For example: `KHR_materials_variants`, `EXT_meshopt_compression`, or `BEVY_my_tool`
///
/// A list of publicly known extensions and their ids can be found
/// in the [KhronosGroup/glTF](https://github.com/KhronosGroup/glTF/blob/main/extensions/README.md)
/// git repo. Vendors reserve prefixes, such as the `BEVY` prefix,
/// which is also listed in the [KhronosGroup repo](https://github.com/KhronosGroup/glTF/blob/main/extensions/Prefixes.md).
///
/// The `GltfExtensionHandler` trait should be implemented to participate in
/// processing glTF files as they load, and exposes glTF extension data via
/// a series of hook callbacks.
///
/// The type a `GltfExtensionHandler` is implemented for can define data
/// which will be cloned for each new glTF load. This enables stateful
/// handling of glTF extension data during a single load.
///
/// When loading a glTF file, a glTF object that could contain extension
/// data will cause the relevant hook to execute once per object.
/// Each invocation will receive all extension data, which is required because
/// many extensions require accessing data defined by other extensions.
///
/// The hooks are always called once, even if there is no extension data
/// This is useful for scenarios where additional extension data isn't
/// required, but processing should still happen.
pub trait GltfExtensionHandler: Send + Sync {
    /// Required for dyn cloning
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler>;

    /// Called when the "global" data for an extension
    /// at the root of a glTF file is encountered.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_root(&mut self, load_context: &mut LoadContext<'_>, gltf: &gltf::Gltf) {}

    #[cfg(feature = "bevy_animation")]
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    /// Called when an individual animation is processed
    fn on_animation(&mut self, gltf_animation: &gltf::Animation, handle: Handle<AnimationClip>) {}

    #[cfg(feature = "bevy_animation")]
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    /// Called when all animations have been collected.
    /// `animations` is the glTF ordered list of `Handle<AnimationClip>`s
    /// `named_animations` is a `HashMap` from animation name to `Handle<AnimationClip>`
    /// `animation_roots` is the glTF index of the animation root object
    fn on_animations_collected(
        &mut self,
        load_context: &mut LoadContext<'_>,
        animations: &[Handle<AnimationClip>],
        named_animations: &HashMap<Box<str>, Handle<AnimationClip>>,
        animation_roots: &HashSet<usize>,
    ) {
    }

    /// Called when an individual texture is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_texture(&mut self, gltf_texture: &gltf::Texture, texture: Handle<bevy_image::Image>) {}

    /// Called when an individual material is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_material: &gltf::Material,
        material: Handle<GltfMaterial>,
        material_asset: &GltfMaterial,
        material_label: &str,
    ) {
    }

    /// Called when an individual glTF primitive is processed
    /// glTF primitives are what become a Bevy `Mesh`
    ///
    /// `buffer_data` is the raw buffer data from the glTF file, where each `Vec<u8>` represents
    /// a buffer containing geometry data such as vertex attributes and indices. Extensions can
    /// read this data to process compressed or encoded primitive data.
    ///
    /// `out_doc` allows extensions to provide a modified or
    /// replacement glTF document. If set, the loader will use this modified document for subsequent
    /// primitive processing. This is useful for extensions that need to decompress or transform
    /// the glTF structure before it is processed.
    ///
    /// `out_data` allows extensions to provide modified or
    /// replacement buffer data. If set, the loader will use this modified buffer data instead of
    /// the original `buffer_data`. This is useful for extensions like `EXT_meshopt_compression`
    /// that need to decompress buffer data before the primitive is processed.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_gltf_primitive(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_document: &gltf::Gltf,
        gltf_primitive: &gltf::Primitive,
        buffer_data: &[Vec<u8>],
        out_doc: &mut Option<gltf::Document>,
        out_data: &mut Option<Vec<Vec<u8>>>,
    ) {
    }

    /// Called when an individual glTF Mesh is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_gltf_mesh(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_mesh: &gltf::Mesh,
        mesh: Handle<GltfMesh>,
    ) {
    }

    /// mesh and material are spawned as a single Entity,
    /// which means an extension would have to decide for
    /// itself how to merge the extension data.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_spawn_mesh_and_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        primitive: &gltf::Primitive,
        mesh: &gltf::Mesh,
        material: &gltf::Material,
        entity: &mut EntityWorldMut,
        material_label: &str,
    ) {
    }

    /// Called when an individual Scene is done processing
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_scene_completed(
        &mut self,
        load_context: &mut LoadContext<'_>,
        scene: &gltf::Scene,
        world_root_id: Entity,
        scene_world: &mut World,
    ) {
    }

    /// Called when a node is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_gltf_node(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
    }

    /// Called with a `DirectionalLight` node is spawned
    /// which is typically created as a result of
    /// `KHR_lights_punctual`
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_spawn_light_directional(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
    }
    /// Called with a `PointLight` node is spawned
    /// which is typically created as a result of
    /// `KHR_lights_punctual`
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_spawn_light_point(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
    }
    /// Called with a `SpotLight` node is spawned
    /// which is typically created as a result of
    /// `KHR_lights_punctual`
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_spawn_light_spot(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
    }
}

impl Clone for Box<dyn GltfExtensionHandler> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}
