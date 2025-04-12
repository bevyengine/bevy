use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent};
use bevy_mesh::Mesh;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::sync_world::SyncToRenderWorld;
use derive_more::derive::From;

/// Must be used with a [`bevy_render::mesh::Mesh`] or MeshletMesh. Cannot be used standalone.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Clone, PartialEq)]
#[require(SyncToRenderWorld)]
pub struct RaytracingMesh3d(pub Handle<Mesh>);
