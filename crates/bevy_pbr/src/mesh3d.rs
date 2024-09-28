use crate::Material;
use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{mesh::Mesh, view::Visibility};
use bevy_transform::components::Transform;

/// A component for rendering 3D meshes, typically with a [material] such as [`StandardMaterial`].
///
/// [material]: crate::material::Material
/// [`StandardMaterial`]: crate::StandardMaterial
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct Mesh3d(pub Handle<Mesh>);

impl From<Handle<Mesh>> for Mesh3d {
    fn from(handle: Handle<Mesh>) -> Self {
        Self(handle)
    }
}

/// A [material](Material) for a [`Mesh3d`].
#[derive(Component, Clone, Debug, Deref, DerefMut, PartialEq, Eq)]
pub struct MeshMaterial3d<M: Material>(pub Handle<M>);

impl<M: Material> Default for MeshMaterial3d<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: Material> From<Handle<M>> for MeshMaterial3d<M> {
    fn from(handle: Handle<M>) -> Self {
        Self(handle)
    }
}
