use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::Reflect;
use bevy_render::mesh::Mesh;

/// A simplified mesh component that can be used for [ray casting](crate::MeshRayCast).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component, Debug)]
pub struct SimplifiedMesh(pub Handle<Mesh>);
