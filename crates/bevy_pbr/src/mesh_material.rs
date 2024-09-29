use crate::Material;
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;

/// A [material](Material) for a [`Mesh3d`].
///
/// See [`Material`] for general information about 3D materials and how to implement your own materials.
///
/// [`Mesh3d`]: bevy_render::mesh::Mesh3d
///
/// # Example
///
/// ```
/// # use bevy_pbr::{Material3d, Mesh3d, MeshMaterial3d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::mesh::Mesh;
/// # use bevy_asset::{AssetServer, Assets};
/// #
/// // Spawn an entity with a mesh using `StandardMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     asset_server: Res<AssetServer>
/// ) {
///     commands.spawn((
///         Mesh3d(meshes.add(Capsule::default())),
///         MeshMaterial3d(materials.add(StandardMaterial {
///             base_color: RED.into(),
///             ..Default::default()
///         })),
///     });
/// }
/// ```
///
/// ## Default Material
///
/// Meshes without a [`MeshMaterial3d`] are rendered with a default [`StandardMaterial`].
/// This material can be overridden by inserting a custom material for the default asset handle.
///
/// ```
/// # use bevy_pbr::{Material3d, Mesh3d, MeshMaterial3d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::mesh::Mesh;
/// # use bevy_asset::{AssetServer, Assets};
/// #
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     // Optional: Insert a custom default material.
///     materials.insert(
///         Handle::<StandardMaterial>::default(),
///         StandardMaterial::from(Color::from_srgb(1.0, 0.0, 1.0)),
///     );
///
///     // Spawn a circle with no material.
///     // The mesh will be rendered with the default material.
///     commands.spawn(Mesh3d(meshes.add(Capsule::default())));
/// }
/// ```
///
/// [`StandardMaterial`]: crate::StandardMaterial
#[derive(Component, Clone, Debug, Deref, DerefMut, PartialEq, Eq)]
#[require(HasMaterial3d)]
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

/// A component that marks an entity as having a [`MeshMaterial3d`].
/// [`Mesh3d`] entities without this component are rendered with a [default material].
///
/// [`Mesh3d`]: bevy_render::mesh::Mesh3d
/// [default material]: crate::MeshMaterial3d#default-material
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct HasMaterial3d;
