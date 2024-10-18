use crate::Material;
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use derive_more::derive::From;

/// A [material](Material) for a [`Mesh3d`].
///
/// See [`Material`] for general information about 3D materials and how to implement your own materials.
///
/// [`Mesh3d`]: bevy_render::mesh::Mesh3d
///
/// # Example
///
/// ```
/// # use bevy_pbr::{Material, MeshMaterial3d, StandardMaterial};
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::mesh::{Mesh, Mesh3d};
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
///
/// ## Default Material
///
/// Meshes without a [`MeshMaterial3d`] are rendered with a default [`StandardMaterial`].
/// This material can be overridden by inserting a custom material for the default asset handle.
///
/// ```
/// # use bevy_pbr::{Material, MeshMaterial3d, StandardMaterial};
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::mesh::{Mesh, Mesh3d};
/// # use bevy_color::Color;
/// # use bevy_asset::{Assets, Handle};
/// # use bevy_math::primitives::Capsule3d;
/// #
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     // Optional: Insert a custom default material.
///     materials.insert(
///         &Handle::<StandardMaterial>::default(),
///         StandardMaterial::from(Color::srgb(1.0, 0.0, 1.0)),
///     );
///
///     // Spawn a capsule with no material.
///     // The mesh will be rendered with the default material.
///     commands.spawn(Mesh3d(meshes.add(Capsule3d::default())));
/// }
/// ```
///
/// [`StandardMaterial`]: crate::StandardMaterial
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default)]
#[require(HasMaterial3d)]
pub struct MeshMaterial3d<M: Material>(pub Handle<M>);

impl<M: Material> Default for MeshMaterial3d<M> {
    fn default() -> Self {
        Self(Handle::default())
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
#[reflect(Component, Default)]
pub struct HasMaterial3d;
