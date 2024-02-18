use crate::mesh::{Mesh, Meshable};

/// A regular polygon in the `XY` plane
#[deprecated(
    since = "0.13.0",
    note = "please use the `RegularPolygon` primitive in `bevy_math` instead"
)]
#[derive(Debug, Copy, Clone)]
pub struct RegularPolygon {
    /// Circumscribed radius in the `XY` plane.
    ///
    /// In other words, the vertices of this polygon will all touch a circle of this radius.
    pub radius: f32,
    /// Number of sides.
    pub sides: usize,
}

impl Default for RegularPolygon {
    fn default() -> Self {
        Self {
            radius: 0.5,
            sides: 6,
        }
    }
}

impl RegularPolygon {
    /// Creates a regular polygon in the `XY` plane
    pub fn new(radius: f32, sides: usize) -> Self {
        Self { radius, sides }
    }
}

impl From<RegularPolygon> for Mesh {
    fn from(polygon: RegularPolygon) -> Self {
        bevy_math::primitives::RegularPolygon::new(polygon.radius, polygon.sides).mesh()
    }
}

/// A circle in the `XY` plane
#[deprecated(
    since = "0.13.0",
    note = "please use the `Circle` primitive in `bevy_math` instead"
)]
#[derive(Debug, Copy, Clone)]
pub struct Circle {
    /// Inscribed radius in the `XY` plane.
    pub radius: f32,
    /// The number of vertices used.
    pub vertices: usize,
}

impl Default for Circle {
    fn default() -> Self {
        Self {
            radius: 0.5,
            vertices: 64,
        }
    }
}

impl Circle {
    /// Creates a circle in the `XY` plane
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            ..Default::default()
        }
    }
}

impl From<Circle> for RegularPolygon {
    fn from(circle: Circle) -> Self {
        Self {
            radius: circle.radius,
            sides: circle.vertices,
        }
    }
}

impl From<Circle> for Mesh {
    fn from(circle: Circle) -> Self {
        Mesh::from(RegularPolygon::from(circle))
    }
}
