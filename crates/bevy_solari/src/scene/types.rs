use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent};
use bevy_mesh::Mesh;
use bevy_pbr::PreviousGlobalTransform;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Clone, PartialEq)]
#[require(Transform, PreviousGlobalTransform)]
pub struct RaytracingMesh3d(pub Handle<Mesh>);
