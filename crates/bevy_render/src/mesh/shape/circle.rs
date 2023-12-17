use crate::mesh::Mesh;

use super::{Facing, MeshFacingExtension, Meshable};
use bevy_math::primitives::{Circle, RegularPolygon};

/// A builder used for creating a [`Mesh`] with a [`Circle`] shape.
#[derive(Clone, Copy, Debug)]
pub struct CircleMesh {
    /// The [`Circle`] shape.
    pub circle: Circle,
    /// The number of vertices used for the circle mesh.
    /// The default is `32`.
    #[doc(alias = "vertices")]
    pub resolution: usize,
    /// The XYZ direction that the mesh is facing.
    /// The default is [`Facing::Z`].
    pub facing: Facing,
}

impl Default for CircleMesh {
    fn default() -> Self {
        Self {
            circle: Circle::default(),
            resolution: 32,
            facing: Facing::Z,
        }
    }
}

impl MeshFacingExtension for CircleMesh {
    #[inline]
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl CircleMesh {
    /// Creates a new [`CircleMesh`] from a given radius and vertex count.
    #[inline]
    pub const fn new(radius: f32, resolution: usize) -> Self {
        Self {
            circle: Circle { radius },
            resolution,
            facing: Facing::Z,
        }
    }

    /// Sets the number of resolution used for the circle mesh.
    #[inline]
    #[doc(alias = "vertices")]
    pub const fn resolution(mut self, resolution: usize) -> Self {
        self.resolution = resolution;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        RegularPolygon::new(self.circle.radius, self.resolution)
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
        RegularPolygon::new(self.circle.radius, self.resolution)
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
