use crate::mesh::Mesh;

use super::Meshable;
use bevy_math::primitives::{Circle, RegularPolygon};

#[derive(Debug)]
pub struct CircleBuilder {
    /// The circle shape.
    pub circle: Circle,
    /// The number of vertices used for the circle mesh.
    pub vertices: usize,
}

impl CircleBuilder {
    pub fn build(&self) -> Mesh {
        RegularPolygon::new(self.circle.radius, self.vertices).mesh()
    }

    /// Sets the number of vertices used for the circle mesh.
    #[doc(alias = "segments")]
    pub fn vertices(mut self, vertices: usize) -> Self {
        self.vertices = vertices;
        self
    }
}

impl Meshable for Circle {
    type Output = CircleBuilder;

    fn mesh(&self) -> Self::Output {
        CircleBuilder {
            circle: *self,
            vertices: 64,
        }
    }
}

impl From<Circle> for Mesh {
    fn from(circle: Circle) -> Self {
        circle.mesh().build()
    }
}

impl From<CircleBuilder> for Mesh {
    fn from(circle: CircleBuilder) -> Self {
        circle.build()
    }
}
