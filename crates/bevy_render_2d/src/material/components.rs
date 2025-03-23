use bevy_asset::{AsAssetId, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use derive_more::derive::From;

use super::Material2d;

/// A [material](Material2d) used for rendering a [`Mesh2d`](bevy_render::mesh::Mesh2d).
///
/// See [`Material2d`] for general information about 2D materials and how to implement your own materials.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::{mesh::{Mesh, Mesh2d}, render_resource::AsBindGroup};
/// # use bevy_render_2d::material::{MeshMaterial2d, Material2d, AlphaMode2d};
/// # use bevy_color::{Color, palettes::basic::RED};
/// # use bevy_asset::{Asset, Assets};
/// # use bevy_math::primitives::Circle;
/// # use bevy_reflect::Reflect;
/// #
/// # // Defining locally so that there is no need to depend on `bevy_sprite`
/// # #[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
/// # struct ColorMaterial {}
/// # impl ColorMaterial {
/// #   pub fn from_color(_color: impl Into<Color>) -> Self { Self {} }
/// # }
/// # impl Material2d for ColorMaterial {
/// #     fn alpha_mode(&self) -> AlphaMode2d {
/// #         AlphaMode2d::Opaque
/// #     }
/// # }
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
///
/// [`MeshMaterial2d`]: crate::MeshMaterial2d
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, From)]
#[reflect(Component, Default, Clone)]
pub struct MeshMaterial2d<M: Material2d>(pub Handle<M>);

impl<M: Material2d> Default for MeshMaterial2d<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: Material2d> PartialEq for MeshMaterial2d<M> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<M: Material2d> Eq for MeshMaterial2d<M> {}

impl<M: Material2d> From<MeshMaterial2d<M>> for AssetId<M> {
    fn from(material: MeshMaterial2d<M>) -> Self {
        material.id()
    }
}

impl<M: Material2d> From<&MeshMaterial2d<M>> for AssetId<M> {
    fn from(material: &MeshMaterial2d<M>) -> Self {
        material.id()
    }
}

impl<M: Material2d> AsAssetId for MeshMaterial2d<M> {
    type Asset = M;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}
