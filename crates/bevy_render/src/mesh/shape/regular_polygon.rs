use super::{Facing, Mesh, MeshFacingExtension, Meshable};
use bevy_math::primitives::{Ellipse, RegularPolygon};

/// A builder used for creating a [`Mesh`] with a [`RegularPolygon`] shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegularPolygonMesh {
    /// The [`RegularPolygon`] shape.
    pub polygon: RegularPolygon,
    /// The XYZ direction that the mesh is facing.
    /// The default is [`Facing::Z`].
    pub facing: Facing,
}

impl MeshFacingExtension for RegularPolygonMesh {
    #[inline]
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl RegularPolygonMesh {
    /// Creates a new [`RegularPolygonMesh`] from the radius
    /// of the circumcircle and a number of sides.
    ///
    /// # Panics
    ///
    /// Panics if `circumradius` is non-positive.
    #[inline]
    pub fn new(circumradius: f32, sides: usize) -> Self {
        Self {
            polygon: RegularPolygon::new(circumradius, sides),
            ..Default::default()
        }
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        // The ellipse mesh is just a regular polygon with two radii
        Ellipse {
            half_width: self.polygon.circumcircle.radius,
            half_height: self.polygon.circumcircle.radius,
        }
        .mesh()
        .resolution(self.polygon.sides)
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
        .resolution(self.polygon.sides)
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
