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

#[cfg(test)]
mod tests {
    use crate::mesh::shape::RegularPolygon;
    use crate::mesh::{Mesh, VertexAttributeValues};

    /// Sin/cos and multiplication computations result in numbers like 0.4999999.
    /// Round these to numbers we expect like 0.5.
    fn fix_floats<const N: usize>(points: &mut [[f32; N]]) {
        for point in points.iter_mut() {
            for coord in point.iter_mut() {
                let round = (*coord * 2.).round() / 2.;
                if (*coord - round).abs() < 0.00001 {
                    *coord = round;
                }
            }
        }
    }

    #[test]
    fn test_regular_polygon() {
        let mut mesh = Mesh::from(RegularPolygon {
            radius: 7.,
            sides: 4,
        });

        let Some(VertexAttributeValues::Float32x3(mut positions)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Expected positions f32x3");
        };
        let Some(VertexAttributeValues::Float32x2(mut uvs)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("Expected uvs f32x2");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("Expected normals f32x3");
        };

        fix_floats(&mut positions);
        fix_floats(&mut uvs);

        assert_eq!(
            [
                [0.0, 7.0, 0.0],
                [7.0, 0.0, 0.0],
                [0.0, -7.0, 0.0],
                [-7.0, 0.0, 0.0],
            ],
            &positions[..]
        );

        // Note V coordinate increases in the opposite direction to the Y coordinate.
        assert_eq!([[0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]], &uvs[..]);

        assert_eq!(&[[0.0, 0.0, 1.0]; 4], &normals[..]);
    }
}
