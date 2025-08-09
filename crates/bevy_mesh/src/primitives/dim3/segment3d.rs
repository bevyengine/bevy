use crate::{Indices, Mesh, MeshBuilder, Meshable, PrimitiveTopology};
use bevy_asset::RenderAssetUsages;
use bevy_math::primitives::Segment3d;
use bevy_reflect::prelude::*;

/// A builder used for creating a [`Mesh`] with a [`Segment3d`] shape.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Default, Debug, Clone)]
pub struct Segment3dMeshBuilder {
    segment: Segment3d,
}

impl MeshBuilder for Segment3dMeshBuilder {
    fn build(&self) -> Mesh {
        let positions: Vec<_> = self.segment.vertices.into();
        let indices = Indices::U32(vec![0, 1]);

        Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
            .with_inserted_indices(indices)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    }
}

impl Meshable for Segment3d {
    type Output = Segment3dMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Segment3dMeshBuilder { segment: *self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Meshable;
    use bevy_math::Vec3;

    #[test]
    fn segment3d_mesh_builder() {
        let segment = Segment3d::new(Vec3::ZERO, Vec3::X);
        let mesh = segment.mesh().build();
        assert_eq!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len(), 2);
        assert_eq!(mesh.indices().unwrap().len(), 2);
    }
}
