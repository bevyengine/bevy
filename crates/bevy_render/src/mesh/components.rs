use crate::{mesh::Mesh, view::Visibility};
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

/// A component for rendering 2D meshes, typically with a [`MeshMaterial2d`] using a [`ColorMaterial`].
///
/// Meshes without a [`MeshMaterial2d`] will be rendered with a [default material].
///
/// [`MeshMaterial2d`]: <https://docs.rs/bevy/latest/bevy/sprite/struct.MeshMaterial2d.html>
/// [`ColorMaterial`]: <https://docs.rs/bevy/latest/bevy/sprite/struct.ColorMaterial.html>
/// [default material]: <https://docs.rs/bevy/latest/bevy/sprite/struct.MeshMaterial2d.html#default-material>
///
/// # Example
///
/// ```ignore
/// # use bevy_sprite::{ColorMaterial, Mesh2d, MeshMaterial2d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::mesh::Mesh;
/// # use bevy_color::palettes::basic::RED;
/// # use bevy_asset::Assets;
/// # use bevy_math::primitives::Circle;
/// #
/// // Spawn an entity with a mesh using `ColorMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<ColorMaterial>>,
/// ) {
///     commands.spawn((
///         Mesh2d(meshes.add(Circle::new(50.0))),
///         MeshMaterial2d(materials.add(ColorMaterial::from_color(RED))),
///     ));
/// }
/// ```
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct Mesh2d(pub Handle<Mesh>);

impl From<Mesh2d> for AssetId<Mesh> {
    fn from(mesh: Mesh2d) -> Self {
        mesh.id()
    }
}

impl From<&Mesh2d> for AssetId<Mesh> {
    fn from(mesh: &Mesh2d) -> Self {
        mesh.id()
    }
}

/// A component for rendering 3D meshes, typically with a [`MeshMaterial3d`] using a [`StandardMaterial`].
///
/// Meshes without a [`MeshMaterial3d`] will be rendered with a [default material].
///
/// [`MeshMaterial3d`]: <https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html>
/// [`StandardMaterial`]: <https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html>
/// [default material]: <https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html#default-material>
///
/// # Example
///
/// ```ignore
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
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct Mesh3d(pub Handle<Mesh>);

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
