use super::{Mesh, Meshable};
use bevy_math::{
    primitives::{Ellipse, RegularPolygon},
    Vec2,
};

/// A builder used for creating a [`Mesh`] with a [`RegularPolygon`] shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegularPolygonMeshBuilder {
    /// The [`RegularPolygon`] shape.
    pub polygon: RegularPolygon,
}

impl RegularPolygonMeshBuilder {
    /// Creates a new [`RegularPolygonMeshBuilder`] from the radius
    /// of the circumcircle and a number of sides.
    ///
    /// # Panics
    ///
    /// Panics if `circumradius` is non-positive.
    #[inline]
    pub fn new(circumradius: f32, sides: usize) -> Self {
        Self {
            polygon: RegularPolygon::new(circumradius, sides),
        }
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        // The ellipse mesh is just a regular polygon with two radii
        Ellipse {
            half_size: Vec2::splat(self.polygon.circumcircle.radius),
        }
        .mesh()
        .resolution(self.polygon.sides)
        .build()
    }
}

impl Meshable for RegularPolygon {
    type Output = RegularPolygonMeshBuilder;

    fn mesh(&self) -> Self::Output {
        RegularPolygonMeshBuilder { polygon: *self }
    }
}

impl From<RegularPolygon> for Mesh {
    fn from(polygon: RegularPolygon) -> Self {
        polygon.mesh().build()
    }
}

impl From<RegularPolygonMeshBuilder> for Mesh {
    fn from(polygon: RegularPolygonMeshBuilder) -> Self {
        polygon.build()
    }
}
