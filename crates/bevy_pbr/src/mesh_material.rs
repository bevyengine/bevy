use crate::Material;
use bevy_asset::{AsAssetId, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use derive_more::derive::From;

/// A [material](Material) used for rendering a [`Mesh3d`].
///
/// See [`Material`] for general information about 3D materials and how to implement your own materials.
///
/// [`Mesh3d`]: bevy_mesh::Mesh3d
///
/// # Example
///
/// ```
/// # use bevy_pbr::{Material, MeshMaterial3d, StandardMaterial};
/// # use bevy_ecs::prelude::*;
/// # use bevy_mesh::{Mesh, Mesh3d};
/// # use bevy_color::palettes::basic::RED;
/// # use bevy_asset::Assets;
/// # use bevy_math::primitives::Capsule3d;
/// #
/// // Spawn an entity with a mesh using `StandardMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     commands.spawn((
///         Mesh3d(meshes.add(Capsule3d::default())),
///         MeshMaterial3d(materials.add(StandardMaterial {
///             base_color: RED.into(),
///             ..Default::default()
///         })),
///     ));
/// }
/// ```
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, From)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct MeshMaterial3d<M: Material>(pub Handle<M>);

impl<M: Material> Default for MeshMaterial3d<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: Material> PartialEq for MeshMaterial3d<M> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<M: Material> Eq for MeshMaterial3d<M> {}

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

impl<M: Material> AsAssetId for MeshMaterial3d<M> {
    type Asset = M;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}
