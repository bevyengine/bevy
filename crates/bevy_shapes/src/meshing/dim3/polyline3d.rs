use crate::{
    meshing::{MeshBuilder, Meshable},
    primitives::Polyline3d,
};
use alloc::vec::Vec;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_reflect::prelude::*;

/// A builder used for creating a [`Mesh`] with a [`Polyline3d`] shape.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Default, Debug, Clone)]
pub struct Polyline3dMeshBuilder {
    polyline: Polyline3d,
}

impl MeshBuilder for Polyline3dMeshBuilder {
    fn build(&self) -> Mesh {
        let positions: Vec<_> = self.polyline.vertices.clone();

        let indices = Indices::U32(
            (0..self.polyline.vertices.len() as u32 - 1)
                .flat_map(|i| [i, i + 1])
                .collect(),
        );

        Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
            .with_inserted_indices(indices)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    }
}

impl Meshable for Polyline3d {
    type Output = Polyline3dMeshBuilder;

    fn mesh_builder(&self) -> Self::Output {
        Polyline3dMeshBuilder {
            polyline: self.clone(),
        }
    }
}
