use crate::mesh::Mesh;

use super::{Facing, MeshFacingExtension, Meshable};
use bevy_math::primitives::{Circle, RegularPolygon};

/// A builder used for creating a [`Mesh`] with a [`Circle`] shape.
#[derive(Clone, Copy, Debug)]
pub struct CircleMeshBuilder {
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

impl Default for CircleMeshBuilder {
    fn default() -> Self {
        Self {
            circle: Circle::default(),
            resolution: 32,
            facing: Facing::Z,
        }
    }
}

impl MeshFacingExtension for CircleMeshBuilder {
    #[inline]
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl CircleMeshBuilder {
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
}

impl Meshable for Circle {
    type Output = CircleMeshBuilder;

    fn mesh(&self) -> Self::Output {
        CircleMeshBuilder {
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

impl From<CircleMeshBuilder> for Mesh {
    fn from(circle: CircleMeshBuilder) -> Self {
        circle.build()
    }
}
