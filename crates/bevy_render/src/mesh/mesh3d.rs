use crate::{mesh::Mesh, view::Visibility};
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;

/// A component for rendering 3D meshes, typically with a [`MeshMaterial3d`] using a [`StandardMaterial`].
///
/// Meshes without a [`MeshMaterial3d`] will be rendered with a [default material].
///
/// [`MeshMaterial3d`]: <https://docs.rs/bevy/latest/bevy/pbr/trait.MeshMaterial3d.html>
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
/// # use bevy_asset::{AssetServer, Assets};
/// # use bevy_math::primitives::Capsule3d;
/// #
/// // Spawn an entity with a mesh using `StandardMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     asset_server: Res<AssetServer>
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
