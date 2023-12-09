use super::{Facing, Mesh, MeshFacingExtension, Meshable};
use bevy_math::primitives::{Ellipse, RegularPolygon};

#[derive(Clone, Copy, Debug, Default)]
pub struct RegularPolygonMesh {
    pub polygon: RegularPolygon,
    pub facing: Facing,
}

impl MeshFacingExtension for RegularPolygonMesh {
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl RegularPolygonMesh {
    pub fn build(&self) -> Mesh {
        // The ellipse mesh is just a regular polygon with two radii
        Ellipse {
            half_width: self.polygon.circumcircle.radius,
            half_height: self.polygon.circumcircle.radius,
        }
        .mesh()
        .vertices(self.polygon.sides)
        .facing(self.facing)
        .build()
    }

    pub(super) fn build_mesh_data(
        &self,
        translation: [f32; 3],
        indices: &mut Vec<u32>,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        uvs: &mut Vec<[f32; 2]>,
    ) {
        Ellipse {
            half_width: self.polygon.circumcircle.radius,
            half_height: self.polygon.circumcircle.radius,
        }
        .mesh()
        .vertices(self.polygon.sides)
        .facing(self.facing)
        .build_mesh_data(translation, indices, positions, normals, uvs);
    }
}

impl Meshable for RegularPolygon {
    type Output = RegularPolygonMesh;

    fn mesh(&self) -> Self::Output {
        RegularPolygonMesh {
            polygon: *self,
            ..Default::default()
        }
    }
}

impl From<RegularPolygon> for Mesh {
    fn from(polygon: RegularPolygon) -> Self {
        polygon.mesh().build()
    }
}

impl From<RegularPolygonMesh> for Mesh {
    fn from(polygon: RegularPolygonMesh) -> Self {
        polygon.build()
    }
}
