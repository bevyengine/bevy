use crate::Material;
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;

/// A [material](Material) for a [`Mesh3d`](bevy_render::mesh::Mesh3d).
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

impl<M: Material> From<MeshMaterial3d<M>> for AssetId<M> {
    fn from(material: MeshMaterial3d<M>) -> Self {
        material.id()
    }
}

impl<M: Material> From<&MeshMaterial3d<M>> for AssetId<M> {
    fn from(material: &MeshMaterial3d<M>) -> Self {
        material.id()
    }
}
