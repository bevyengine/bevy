use crate::{mesh::Mesh, view::Visibility};
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;

/// A component for rendering 3D meshes, typically with a [material] such as [`StandardMaterial`].
///
/// [material]: <https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html>
/// [`StandardMaterial`]: <https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html>
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct Mesh3d(pub Handle<Mesh>);

impl From<Handle<Mesh>> for Mesh3d {
    fn from(handle: Handle<Mesh>) -> Self {
        Self(handle)
    }
}

impl From<Mesh3d> for AssetId<Mesh> {
    fn from(mesh: Mesh3d) -> Self {
        mesh.id()
    }
}

impl From<&Mesh3d> for AssetId<Mesh> {
    fn from(mesh: &Mesh3d) -> Self {
        mesh.id()
    }
}
