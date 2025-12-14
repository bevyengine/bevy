//! glTF extensions defined by the Khronos Group and other vendors

mod khr_materials_anisotropy;
mod khr_materials_clearcoat;
mod khr_materials_specular;

use alloc::sync::Arc;
use async_lock::RwLock;

use bevy_animation::AnimationClip;
use bevy_asset::{Handle, LoadContext};
use bevy_ecs::{
    entity::Entity,
    resource::Resource,
    world::{EntityWorldMut, World},
};
use bevy_pbr::StandardMaterial;
use bevy_platform::collections::{HashMap, HashSet};
use gltf::Node;

use crate::GltfMesh;

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
pub trait GltfExtensionHandler: Send + Sync {
    /// Required for dyn cloning
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler>;

    /// When loading a glTF file, a glTF object that could contain extension
    /// data will cause the relevant hook to execute once for each id in this list.
    /// Each invocation will receive the extension data for one of the extension ids,
    /// along with the `extension_id` itself so implementors can differentiate
    /// between different calls and parse data correctly.
    ///
    /// The hooks are always called, even if there is no extension data
    /// for a specified id. This is useful for scenarios where additional
    /// extension data isn't required, but processing should still happen.
    ///
    /// Most implementors will pick one extension for this list, causing the
    /// relevant hooks to fire once per object. An implementor that does not
    /// wish to receive any data but still wants hooks to be called can use
    /// an empty string `""` as the extension id, which is also the default
    /// value if the function is not implemented by an implementor. If the
    /// empty string is used, all extension data in hooks will be `None`.
    ///
    /// Some implementors will choose to list multiple extensions here.
    /// This is an advanced use case and the alternative of having multiple
    /// independent handlers should be considered as an option first.
    /// If multiple extension ids are listed here, the hooks will fire once
    /// for each extension id, and each successive call will receive the data for
    /// a separate extension. The extension id is also included in hook arguments
    /// for this reason, so multiple extension id implementors can differentiate
    /// between the data received.
    fn extension_ids(&self) -> &'static [&'static str] {
        &[""]
    }

    /// Called when the "global" data for an extension
    /// at the root of a glTF file is encountered.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_root_data(&mut self, extension_id: &str, value: Option<&serde_json::Value>) {}

    #[cfg(feature = "bevy_animation")]
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    /// Called when an individual animation is processed
    fn on_animation(
        &mut self,
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
        gltf_animation: &gltf::Animation,
        name: Option<&str>,
        handle: Handle<AnimationClip>,
    ) {
    }

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
    fn on_texture(
        &mut self,
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
        texture: Handle<bevy_image::Image>,
    ) {
    }

    /// Called when an individual material is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_material(
        &mut self,
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
        load_context: &mut LoadContext<'_>,
        gltf_material: &gltf::Material,
        name: Option<&str>,
        material: Handle<StandardMaterial>,
    ) {
    }

    /// Called when an individual glTF Mesh is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_gltf_mesh(
        &mut self,
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
        load_context: &mut LoadContext<'_>,
        gltf_mesh: &gltf::Mesh,
        name: Option<&str>,
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
    ) {
    }

    /// Called when an individual Scene is done processing
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_scene_completed(
        &mut self,
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
        scene: &gltf::Scene,
        name: Option<&str>,
        world_root_id: Entity,
        scene_world: &mut World,
        load_context: &mut LoadContext<'_>,
    ) {
    }

    /// Called when a node is processed
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_gltf_node(
        &mut self,
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
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
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
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
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
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
        extension_id: &str,
        extension_data: Option<&serde_json::Value>,
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
