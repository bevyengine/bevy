use crate::mesh::Mesh;

use super::{Facing, MeshFacingExtension, Meshable};
use bevy_math::primitives::{Circle, RegularPolygon};

#[derive(Clone, Copy, Debug)]
pub struct CircleMesh {
    /// The circle shape.
    pub circle: Circle,
    /// The number of vertices used for the circle mesh.
    pub vertices: usize,
    pub facing: Facing,
}

impl Default for CircleMesh {
    fn default() -> Self {
        Self {
            circle: Circle::default(),
            vertices: 32,
            facing: Facing::Z,
        }
    }
}

impl MeshFacingExtension for CircleMesh {
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
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

    pub fn build(&self) -> Mesh {
        RegularPolygon::new(self.circle.radius, self.vertices)
            .mesh()
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
            ..Default::default()
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
