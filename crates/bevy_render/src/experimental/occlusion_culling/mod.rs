//! GPU occlusion culling.
//!
//! See [`OcclusionCulling`] for a detailed description of occlusion culling in
//! Bevy.

use bevy_app::{App, Plugin};
use bevy_ecs::{component::Component, entity::Entity, prelude::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_shader::load_shader_library;

use crate::{extract_component::ExtractComponent, render_resource::TextureView};

/// Enables GPU occlusion culling.
///
/// See [`OcclusionCulling`] for a detailed description of occlusion culling in
/// Bevy.
pub struct OcclusionCullingPlugin;

impl Plugin for OcclusionCullingPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "mesh_preprocess_types.wgsl");
    }
}

/// Add this component to a view in order to enable experimental GPU occlusion
/// culling.
///
/// *Bevy's occlusion culling is currently marked as experimental.* There are
/// known issues whereby, in rare circumstances, occlusion culling can result in
/// meshes being culled that shouldn't be (i.e. meshes that turn invisible).
/// Please try it out and report issues.
///
/// *Occlusion culling* allows Bevy to avoid rendering objects that are fully
/// behind other opaque or alpha tested objects. This is different from, and
/// complements, depth fragment rejection as the `DepthPrepass` enables. While
/// depth rejection allows Bevy to avoid rendering *pixels* that are behind
/// other objects, the GPU still has to examine those pixels to reject them,
/// which requires transforming the vertices of the objects and performing
/// skinning if the objects were skinned. Occlusion culling allows the GPU to go
/// a step further, avoiding even transforming the vertices of objects that it
/// can quickly prove to be behind other objects.
///
/// Occlusion culling inherently has some overhead, because Bevy must examine
/// the objects' bounding boxes, and create an acceleration structure
/// (hierarchical Z-buffer) to perform the occlusion tests. Therefore, occlusion
/// culling is disabled by default. Only enable it if you measure it to be a
/// speedup on your scene. Note that, because Bevy's occlusion culling runs on
/// the GPU and is quite efficient, it's rare for occlusion culling to result in
/// a significant slowdown.
///
/// Occlusion culling currently requires a `DepthPrepass`. If no depth prepass
/// is present on the view, the [`OcclusionCulling`] component will be ignored.
/// Additionally, occlusion culling is currently incompatible with deferred
/// shading; including both `DeferredPrepass` and [`OcclusionCulling`] results
/// in unspecified behavior.
///
/// The algorithm that Bevy uses is known as [*two-phase occlusion culling*].
/// When you enable occlusion culling, Bevy splits the depth prepass into two:
/// an *early* depth prepass and a *late* depth prepass. The early depth prepass
/// renders all the meshes that were visible last frame to produce a
/// conservative approximation of the depth buffer. Then, after producing an
/// acceleration structure known as a hierarchical Z-buffer or depth pyramid,
/// Bevy tests the bounding boxes of all meshes against that depth buffer. Those
/// that can be quickly proven to be behind the geometry rendered during the
/// early depth prepass are skipped entirely. The other potentially-visible
/// meshes are rendered during the late prepass, and finally all the visible
/// meshes are rendered as usual during the opaque, transparent, etc. passes.
///
/// Unlike other occlusion culling systems you may be familiar with, Bevy's
/// occlusion culling is fully dynamic and requires no baking step. The CPU
/// overhead is minimal. Large skinned meshes and other dynamic objects can
/// occlude other objects.
///
/// [*two-phase occlusion culling*]:
/// https://medium.com/@mil_kru/two-pass-occlusion-culling-4100edcad501
#[derive(Component, ExtractComponent, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct OcclusionCulling;

/// A render-world component that contains resources necessary to perform
/// occlusion culling on any view other than a camera.
///
/// Bevy automatically places this component on views created for shadow
/// mapping. You don't ordinarily need to add this component yourself.
#[derive(Clone, Component)]
pub struct OcclusionCullingSubview {
    /// A texture view of the Z-buffer.
    pub depth_texture_view: TextureView,
    /// The size of the texture along both dimensions.
    ///
    /// Because [`OcclusionCullingSubview`] is only currently used for shadow
    /// maps, they're guaranteed to have sizes equal to a power of two, so we
    /// don't have to store the two dimensions individually here.
    pub depth_texture_size: u32,
}

/// A render-world component placed on each camera that stores references to all
/// entities other than cameras that need occlusion culling.
///
/// Bevy automatically places this component on cameras that are drawing
/// shadows, when those shadows come from lights with occlusion culling enabled.
/// You don't ordinarily need to add this component yourself.
#[derive(Clone, Component)]
pub struct OcclusionCullingSubviewEntities(pub Vec<Entity>);
