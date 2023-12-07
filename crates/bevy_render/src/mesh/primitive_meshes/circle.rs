use crate::mesh::Mesh;

use super::{Facing, Meshable};
use bevy_math::primitives::{Circle, RegularPolygon};

#[derive(Debug)]
pub struct CircleMesh {
    /// The circle shape.
    pub circle: Circle,
    /// The number of vertices used for the circle mesh.
    pub vertices: usize,
    pub facing: Facing,
}

impl CircleMesh {
    /// Creates a new [`CircleMesh`] from a given radius and vertex count.
    pub const fn new(radius: f32, vertices: usize) -> Self {
        Self {
            circle: Circle { radius },
            vertices,
            facing: Facing::Z,
        }
    }

    /// Sets the number of vertices used for the circle mesh.
    #[doc(alias = "segments")]
    pub const fn vertices(mut self, vertices: usize) -> Self {
        self.vertices = vertices;
        self
    }

    pub const fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }

    pub const fn facing_x(mut self) -> Self {
        self.facing = Facing::X;
        self
    }

    pub const fn facing_y(mut self) -> Self {
        self.facing = Facing::Y;
        self
    }

    pub const fn facing_z(mut self) -> Self {
        self.facing = Facing::Z;
        self
    }

    pub const fn facing_neg_x(mut self) -> Self {
        self.facing = Facing::NegX;
        self
    }

    pub const fn facing_neg_y(mut self) -> Self {
        self.facing = Facing::NegY;
        self
    }

    pub const fn facing_neg_z(mut self) -> Self {
        self.facing = Facing::NegZ;
        self
    }

    pub fn build(&self) -> Mesh {
        RegularPolygon::new(self.circle.radius, self.vertices).into()
    }

    pub(super) fn build_mesh_data(
        &self,
        translation: [f32; 3],
        indices: &mut Vec<u32>,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        uvs: &mut Vec<[f32; 2]>,
    ) {
        RegularPolygon::new(self.circle.radius, self.vertices)
            .mesh()
            .facing(self.facing)
            .build_mesh_data(translation, indices, positions, normals, uvs);
    }
}

impl Meshable for Circle {
    type Output = CircleMesh;

    fn mesh(&self) -> Self::Output {
        CircleMesh {
            circle: *self,
            vertices: 64,
            facing: Facing::Z,
        }
    }
}

impl From<Circle> for Mesh {
    fn from(circle: Circle) -> Self {
        circle.mesh().build()
    }
}

impl From<CircleMesh> for Mesh {
    fn from(circle: CircleMesh) -> Self {
        circle.build()
    }
}
