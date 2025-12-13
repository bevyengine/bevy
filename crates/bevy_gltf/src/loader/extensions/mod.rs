//! glTF extensions defined by the Khronos Group and other vendors

mod khr_materials_anisotropy;
mod khr_materials_clearcoat;
mod khr_materials_specular;

use bevy_animation::AnimationClip;
use bevy_asset::{Handle, LoadContext};
use bevy_ecs::{
    entity::Entity,
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

/// Implement this trait to be able to process glTF extension data
pub trait GltfExtensionProcessor: Send + Sync {
    /// Required for dyn cloning
    fn dyn_clone(&self) -> Box<dyn GltfExtensionProcessor>;

    /// The extension ids that this `GltfExtensionProcessor` should process.
    /// This is used to dispatch callbacks when relevant data is encountered.
    /// For example: `KHR_materials_variants`, `EXT_meshopt_compression`, or `BEVY_my_tool`
    ///
    /// The default list of extensions to handle is an empty string so
    /// that extensions get called even if they don't define specific
    /// extensions to handle. This results in all extension data being
    /// `None` in all hooks.
    /// Having the hooks be called even when there is no specific
    /// extension being handled is useful for scenarios where additional
    /// extension data isn't required, but processing should still happen.
    fn extension_ids(&self) -> &'static [&'static str] {
        &[""]
    }

    /// Called when the "global" data for an extension
    /// at the root of a glTF file is encountered.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_root_data(&mut self, value: Option<&serde_json::Value>) {}

    #[cfg(feature = "bevy_animation")]
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    /// Called when an individual animation is processed
    fn on_animation(
        &mut self,
        value: Option<&serde_json::Value>,
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
        value: Option<&serde_json::Value>,
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
        value: Option<&serde_json::Value>,
        load_context: &mut LoadContext<'_>,
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
        value: Option<&serde_json::Value>,
        load_context: &mut LoadContext<'_>,
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
        gltf_node: &Node,
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
        value: Option<&serde_json::Value>,
        name: Option<&str>,
        world_root_id: Entity,
        world: &mut World,
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
        value: Option<&serde_json::Value>,
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
        value: Option<&serde_json::Value>,
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
        value: Option<&serde_json::Value>,
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
        value: Option<&serde_json::Value>,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
    }
}

impl Clone for Box<dyn GltfExtensionProcessor> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}
