use crate::mesh::Mesh;
use bevy_asset::{AsAssetId, AssetEvent, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChangesMut, component::Component, event::EventReader,
    reflect::ReflectComponent, system::Query,
};
use bevy_platform::{collections::HashSet, hash::FixedHasher};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

/// A component for 2D meshes. Requires a [`MeshMaterial2d`] to be rendered, commonly using a [`ColorMaterial`].
///
/// [`MeshMaterial2d`]: <https://docs.rs/bevy/latest/bevy/sprite/struct.MeshMaterial2d.html>
/// [`ColorMaterial`]: <https://docs.rs/bevy/latest/bevy/sprite/struct.ColorMaterial.html>
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
#[reflect(Component, Default, Clone, PartialEq)]
#[require(Transform)]
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

impl AsAssetId for Mesh2d {
    type Asset = Mesh;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// A component for 3D meshes. Requires a [`MeshMaterial3d`] to be rendered, commonly using a [`StandardMaterial`].
///
/// [`MeshMaterial3d`]: <https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html>
/// [`StandardMaterial`]: <https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html>
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
#[reflect(Component, Default, Clone, PartialEq)]
#[require(Transform)]
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

impl AsAssetId for Mesh3d {
    type Asset = Mesh;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// A system that marks a [`Mesh3d`] as changed if the associated [`Mesh`] asset
/// has changed.
///
/// This is needed because the systems that extract meshes, such as
/// `extract_meshes_for_gpu_building`, write some metadata about the mesh (like
/// the location within each slab) into the GPU structures that they build that
/// needs to be kept up to date if the contents of the mesh change.
pub fn mark_3d_meshes_as_changed_if_their_assets_changed(
    mut meshes_3d: Query<&mut Mesh3d>,
    mut mesh_asset_events: EventReader<AssetEvent<Mesh>>,
) {
    let mut changed_meshes: HashSet<AssetId<Mesh>, FixedHasher> = HashSet::default();
    for mesh_asset_event in mesh_asset_events.read() {
        if let AssetEvent::Modified { id } = mesh_asset_event {
            changed_meshes.insert(*id);
        }
    }

    if changed_meshes.is_empty() {
        return;
    }

    for mut mesh_3d in &mut meshes_3d {
        if changed_meshes.contains(&mesh_3d.0.id()) {
            mesh_3d.set_changed();
        }
    }
}

/// A component that stores an arbitrary index used to identify the mesh instance when rendering.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct MeshTag(pub u32);
