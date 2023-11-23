use crate::mesh::simple::{SimpleMeshBuilder, SimpleVertex};
use crate::mesh::Mesh;
use bevy_math::{Vec2, Vec3};

/// A regular polygon in the `XY` plane
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
        debug_assert!(
            polygon.sides > 2,
            "RegularPolygon requires at least 3 sides."
        );

        let mut mesh = SimpleMeshBuilder::default();

        fn vertex_at(i: usize, polygon: &RegularPolygon) -> SimpleVertex {
            let step = std::f32::consts::TAU / polygon.sides as f32;
            let theta = std::f32::consts::FRAC_PI_2 - i as f32 * step;
            let (sin, cos) = theta.sin_cos();

            SimpleVertex {
                position: Vec3::new(cos * polygon.radius, sin * polygon.radius, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                uv: Vec2::new(0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)),
            }
        }

        for i in 1..(polygon.sides - 1) {
            // Vertices are generated in CW order above, hence the reversed indices here
            // to emit triangle vertices in CCW order.
            mesh.triangle(
                vertex_at(0, &polygon),
                vertex_at(i + 1, &polygon),
                vertex_at(i, &polygon),
            );
        }

        mesh.build()
    }
}

/// A circle in the `XY` plane
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
