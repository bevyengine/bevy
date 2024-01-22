use crate::mesh::Mesh;

use super::Meshable;
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
}

impl Default for CircleMeshBuilder {
    fn default() -> Self {
        Self {
            circle: Circle::default(),
            resolution: 32,
        }
    }
}

impl CircleMeshBuilder {
    /// Creates a new [`CircleMeshBuilder`] from a given radius and vertex count.
    #[inline]
    pub const fn new(radius: f32, resolution: usize) -> Self {
        Self {
            circle: Circle { radius },
            resolution,
        }
    }

    /// Sets the number of vertices used for the circle mesh.
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
