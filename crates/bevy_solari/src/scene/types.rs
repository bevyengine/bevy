use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent, template::FromTemplate};
use bevy_mesh::Mesh;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::sync_world::SyncToRenderWorld;
use bevy_transform::components::Transform;
use derive_more::derive::From;

/// A mesh component used for raytracing.
///
/// The mesh used in this component must declare the BLAS its material needs in
/// [`Mesh::raytracing`],
/// use exactly the following set of vertex attributes, in their default formats: `{POSITION, NORMAL, UV_0, TANGENT}`, use [`bevy_mesh::PrimitiveTopology::TriangleList`],
/// and use [`bevy_mesh::Indices::U32`].
///
/// Meshes with incompatible vertex attributes, topology, or index formats are skipped
/// without a warning.
/// Skinned meshes are skipped because joint indices and weights are additional
/// vertex attributes. Morph targets are not vertex attributes and are ignored:
/// a compatible morphed mesh is ray traced in its unmorphed base shape.
///
/// A separate compatible mesh can represent a static pose, but it does not follow
/// later skinning or morph changes. It is placed by this entity's transform, which
/// skinning does not apply, so a pose baked from joint matrices must be expressed
/// in this entity's local space.
///
/// The material used for this entity must be [`MeshMaterial3d<StandardMaterial>`].
#[derive(
    Component, FromTemplate, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From,
)]
#[reflect(Component, Default, Clone, PartialEq)]
#[require(MeshMaterial3d<StandardMaterial>, Transform, SyncToRenderWorld)]
pub struct RaytracingMesh3d(pub Handle<Mesh>);
