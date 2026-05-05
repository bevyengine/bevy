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
use bevy_mesh::{Mesh, MeshVertexAttribute};
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use gltf::Node;

use bevy_platform::collections::HashMap;
#[cfg(feature = "bevy_animation")]
use {bevy_animation::AnimationClip, bevy_platform::collections::HashSet};

use crate::{GltfLoaderSettings, GltfMaterial, GltfMesh};

pub(crate) use self::{
    khr_materials_anisotropy::AnisotropyExtension, khr_materials_clearcoat::ClearcoatExtension,
    khr_materials_specular::SpecularExtension,
};

/// Stores the `ErasedGltfExtensionHandler` implementations so that they
/// can be added by users and also passed to the glTF loader
#[derive(Resource, Default)]
pub struct GltfExtensionHandlers(pub Arc<RwLock<Vec<Box<dyn ErasedGltfExtensionHandler>>>>);

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
pub trait GltfExtensionHandler: Send + Sync + 'static {
    /// Required for dyn cloning
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler>;

    /// Called when the "global" data for an extension
    /// at the root of a glTF file is encountered.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_root(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf: &gltf::Gltf,
        settings: &GltfLoaderSettings,
    ) {
    }

    #[cfg(feature = "bevy_animation")]
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    /// Called when an individual animation is processed
    fn on_animation(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_animation: &gltf::Animation,
        animation_clip: &mut AnimationClip,
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
    /// This hook is useful for extensions that need to
    /// decompress or transform primitives and their associated
    /// glTF data.
    ///
    /// `buffer_data` is a reference to all of the buffers from the
    /// glTF document, in order, after it has been loaded by Bevy. Extensions
    /// in glTF are allowed to add arbitrary buffers, so while this
    /// data is often vertex data, it can not be assumed to be
    /// vertex data.
    ///
    /// `out_doc` is an optional `gltf::Document` which, if set,
    /// must contain a single `gltf::Mesh` with a single
    /// `gltf::Primitive`. This document is only used by Bevy for
    /// the processing of the relevant primitive and can not affect
    /// other processing.
    ///
    /// `out_data` is a single buffer wrapped in a `Vec`, which mirrors
    /// the buffer structure of a loaded `gltf::Document`'s buffers, which
    /// is the same structure as `buffer_data`. The outer `Vec` must
    /// contain a single `Vec<u8>` of data, as only the first generated
    /// buffer is used. If set, the loader will use this modified buffer
    /// data instead of the original `buffer_data` to construct the Mesh.
    #[expect(
        unused,
        reason = "default trait implementations do not use the arguments because they are no-ops"
    )]
    fn on_gltf_primitive(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_document: &gltf::Gltf,
        gltf_mesh: &gltf::Mesh,
        gltf_primitive: &gltf::Primitive,
        buffer_data: &[Vec<u8>],
        custom_vertex_attributes: &HashMap<Box<str>, MeshVertexAttribute>,
        gltf_mesh_on_skinned_nodes: bool,
        gltf_mesh_on_non_skinned_nodes: bool,
        user_mesh: &mut Option<Mesh>,
    ) -> impl ConditionalSendFuture<Output = ()> {
        async {}
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

/// Type-erased version of [`GltfExtensionHandler`].
/// This is used to store heterogeneous handlers in a collection.
pub trait ErasedGltfExtensionHandler: Send + Sync + 'static {
    /// Required for dyn cloning
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler>;

    /// Called when the "global" data for an extension
    /// at the root of a glTF file is encountered.
    fn on_root(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf: &gltf::Gltf,
        settings: &GltfLoaderSettings,
    );

    #[cfg(feature = "bevy_animation")]
    /// Called when an individual animation is processed
    fn on_animation(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_animation: &gltf::Animation,
        animation_clip: &mut AnimationClip,
    );

    #[cfg(feature = "bevy_animation")]
    /// Called when all animations have been collected.
    fn on_animations_collected(
        &mut self,
        load_context: &mut LoadContext<'_>,
        animations: &[Handle<AnimationClip>],
        named_animations: &HashMap<Box<str>, Handle<AnimationClip>>,
        animation_roots: &HashSet<usize>,
    );

    /// Called when an individual texture is processed
    fn on_texture(&mut self, gltf_texture: &gltf::Texture, texture: Handle<bevy_image::Image>);

    /// Called when an individual material is processed
    fn on_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_material: &gltf::Material,
        material: Handle<GltfMaterial>,
        material_asset: &GltfMaterial,
        material_label: &str,
    );

    /// Called when an individual glTF primitive is processed
    fn on_gltf_primitive<'a>(
        &'a mut self,
        load_context: &'a mut LoadContext<'_>,
        gltf_document: &'a gltf::Gltf,
        gltf_mesh: &'a gltf::Mesh,
        gltf_primitive: &'a gltf::Primitive,
        buffer_data: &'a [Vec<u8>],
        custom_vertex_attributes: &'a HashMap<Box<str>, MeshVertexAttribute>,
        gltf_mesh_on_skinned_nodes: bool,
        gltf_mesh_on_non_skinned_nodes: bool,
        user_mesh: &'a mut Option<Mesh>,
    ) -> BoxedFuture<'a, ()>;

    /// Called when an individual glTF Mesh is processed
    fn on_gltf_mesh(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_mesh: &gltf::Mesh,
        mesh: Handle<GltfMesh>,
    );

    /// Called when mesh and material are spawned as a single Entity
    fn on_spawn_mesh_and_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        primitive: &gltf::Primitive,
        mesh: &gltf::Mesh,
        material: &gltf::Material,
        entity: &mut EntityWorldMut,
        material_label: &str,
    );

    /// Called when an individual Scene is done processing
    fn on_scene_completed(
        &mut self,
        load_context: &mut LoadContext<'_>,
        scene: &gltf::Scene,
        world_root_id: Entity,
        scene_world: &mut World,
    );

    /// Called when a node is processed
    fn on_gltf_node(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    );

    /// Called when a `DirectionalLight` node is spawned
    fn on_spawn_light_directional(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    );

    /// Called when a `PointLight` node is spawned
    fn on_spawn_light_point(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    );

    /// Called when a `SpotLight` node is spawned
    fn on_spawn_light_spot(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    );
}

impl<H: GltfExtensionHandler> ErasedGltfExtensionHandler for H {
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler> {
        Self::dyn_clone(self)
    }

    fn on_root(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf: &gltf::Gltf,
        settings: &GltfLoaderSettings,
    ) {
        Self::on_root(self, load_context, gltf, settings);
    }

    #[cfg(feature = "bevy_animation")]
    fn on_animation(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_animation: &gltf::Animation,
        animation_clip: &mut AnimationClip,
    ) {
        Self::on_animation(self, load_context, gltf_animation, animation_clip);
    }

    #[cfg(feature = "bevy_animation")]
    fn on_animations_collected(
        &mut self,
        load_context: &mut LoadContext<'_>,
        animations: &[Handle<AnimationClip>],
        named_animations: &HashMap<Box<str>, Handle<AnimationClip>>,
        animation_roots: &HashSet<usize>,
    ) {
        Self::on_animations_collected(
            self,
            load_context,
            animations,
            named_animations,
            animation_roots,
        );
    }

    fn on_texture(&mut self, gltf_texture: &gltf::Texture, texture: Handle<bevy_image::Image>) {
        Self::on_texture(self, gltf_texture, texture);
    }

    fn on_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_material: &gltf::Material,
        material: Handle<GltfMaterial>,
        material_asset: &GltfMaterial,
        material_label: &str,
    ) {
        Self::on_material(
            self,
            load_context,
            gltf_material,
            material,
            material_asset,
            material_label,
        );
    }

    fn on_gltf_primitive<'a>(
        &'a mut self,
        load_context: &'a mut LoadContext<'_>,
        gltf_document: &'a gltf::Gltf,
        gltf_mesh: &'a gltf::Mesh,
        gltf_primitive: &'a gltf::Primitive,
        buffer_data: &'a [Vec<u8>],
        custom_vertex_attributes: &'a HashMap<Box<str>, MeshVertexAttribute>,
        gltf_mesh_on_skinned_nodes: bool,
        gltf_mesh_on_non_skinned_nodes: bool,
        user_mesh: &'a mut Option<Mesh>,
    ) -> BoxedFuture<'a, ()> {
        Box::pin(async move {
            Self::on_gltf_primitive(
                self,
                load_context,
                gltf_document,
                gltf_mesh,
                gltf_primitive,
                buffer_data,
                custom_vertex_attributes,
                gltf_mesh_on_skinned_nodes,
                gltf_mesh_on_non_skinned_nodes,
                user_mesh,
            )
            .await;
        })
    }

    fn on_gltf_mesh(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_mesh: &gltf::Mesh,
        mesh: Handle<GltfMesh>,
    ) {
        Self::on_gltf_mesh(self, load_context, gltf_mesh, mesh);
    }

    fn on_spawn_mesh_and_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        primitive: &gltf::Primitive,
        mesh: &gltf::Mesh,
        material: &gltf::Material,
        entity: &mut EntityWorldMut,
        material_label: &str,
    ) {
        Self::on_spawn_mesh_and_material(
            self,
            load_context,
            primitive,
            mesh,
            material,
            entity,
            material_label,
        );
    }

    fn on_scene_completed(
        &mut self,
        load_context: &mut LoadContext<'_>,
        scene: &gltf::Scene,
        world_root_id: Entity,
        scene_world: &mut World,
    ) {
        Self::on_scene_completed(self, load_context, scene, world_root_id, scene_world);
    }

    fn on_gltf_node(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
        Self::on_gltf_node(self, load_context, gltf_node, entity);
    }

    fn on_spawn_light_directional(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
        Self::on_spawn_light_directional(self, load_context, gltf_node, entity);
    }

    fn on_spawn_light_point(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
        Self::on_spawn_light_point(self, load_context, gltf_node, entity);
    }

    fn on_spawn_light_spot(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut,
    ) {
        Self::on_spawn_light_spot(self, load_context, gltf_node, entity);
    }
}

impl Clone for Box<dyn ErasedGltfExtensionHandler> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}
