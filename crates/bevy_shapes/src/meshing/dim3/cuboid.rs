use crate::primitives::Cuboid;
use alloc::vec::Vec;
use bevy_asset::RenderAssetUsages;
use bevy_math::Vec3;
use bevy_mesh::{Indices, Mesh, MeshBuilder, Meshable, PrimitiveTopology};
use bevy_reflect::prelude::*;

/// A builder used for creating a [`Mesh`] with a [`Cuboid`] shape.
#[derive(Clone, Copy, Debug, Reflect)]
#[reflect(Default, Debug, Clone)]
pub struct CuboidMeshBuilder {
    half_size: Vec3,
}

impl Default for CuboidMeshBuilder {
    /// Returns the default [`CuboidMeshBuilder`] with a width, height, and depth of `1.0`.
    fn default() -> Self {
        Self {
            half_size: Vec3::splat(0.5),
        }
    }
}

impl MeshBuilder for CuboidMeshBuilder {
    fn build(&self) -> Mesh {
        Mesh::cuboid_mesh(self.half_size)
    }
}

impl Meshable for Cuboid {
    type Output = CuboidMeshBuilder;

    fn mesh_builder(&self) -> Self::Output {
        CuboidMeshBuilder {
            half_size: self.half_size,
        }
    }
}
