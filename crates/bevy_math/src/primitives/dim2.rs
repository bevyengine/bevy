use super::{InvalidDirectionError, Primitive2d, WindingOrder};
use crate::Vec2;

/// A normalized vector pointing in a direction in 2D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Direction2d(Vec2);

impl Direction2d {
    /// A unit vector pointing along the positive X axis.
    pub const X: Self = Self(Vec2::X);
    /// A unit vector pointing along the positive Y axis.
    pub const Y: Self = Self(Vec2::Y);
    /// A unit vector pointing along the negative X axis.
    pub const NEG_X: Self = Self(Vec2::NEG_X);
    /// A unit vector pointing along the negative Y axis.
    pub const NEG_Y: Self = Self(Vec2::NEG_Y);

    /// Create a direction from a finite, nonzero [`Vec2`].
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new(value: Vec2) -> Result<Self, InvalidDirectionError> {
        Self::new_and_length(value).map(|(dir, _)| dir)
    }

    /// Create a direction from a finite, nonzero [`Vec2`], also returning its original length.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new_and_length(value: Vec2) -> Result<(Self, f32), InvalidDirectionError> {
        let length = value.length();
        let direction = (length.is_finite() && length > 0.0).then_some(value / length);

        direction
            .map(|dir| (Self(dir), length))
            .map_or(Err(InvalidDirectionError::from_length(length)), Ok)
    }

    /// Create a direction from its `x` and `y` components.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the vector formed by the components is zero (or very close to zero), infinite, or `NaN`.
    pub fn from_xy(x: f32, y: f32) -> Result<Self, InvalidDirectionError> {
        Self::new(Vec2::new(x, y))
    }

    /// Create a direction from a [`Vec2`] that is already normalized.
    pub fn from_normalized(value: Vec2) -> Self {
        debug_assert!(value.is_normalized());
        Self(value)
    }
}

impl TryFrom<Vec2> for Direction2d {
    type Error = InvalidDirectionError;

    fn try_from(value: Vec2) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::ops::Deref for Direction2d {
    type Target = Vec2;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Neg for Direction2d {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

/// A circle primitive
#[derive(Clone, Copy, Debug)]
pub struct Circle {
    /// The radius of the circle
    pub radius: f32,
}
impl Primitive2d for Circle {}

impl Default for Circle {
    /// Returns the default [`Circle`] with a radius of `0.5`.
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

/// An ellipse primitive
#[derive(Clone, Copy, Debug)]
pub struct Ellipse {
    /// Half of the width and height of the ellipse.
    ///
    /// This corresponds to the two perpendicular radii defining the ellipse.
    pub half_size: Vec2,
}
impl Primitive2d for Ellipse {}

impl Default for Ellipse {
    /// Returns the default [`Ellipse`] with a half-width of `1.0` and a half-height of `0.5`.
    fn default() -> Self {
        Self {
            half_size: Vec2::new(1.0, 0.5),
        }
    }
}

impl Ellipse {
    /// Create a new `Ellipse` from half of its width and height.
    ///
    /// This corresponds to the two perpendicular radii defining the ellipse.
    #[inline]
    pub const fn new(half_width: f32, half_height: f32) -> Self {
        Self {
            half_size: Vec2::new(half_width, half_height),
        }
    }

    /// Create a new `Ellipse` from a given full size.
    ///
    /// `size.x` is the diameter along the X axis, and `size.y` is the diameter along the Y axis.
    #[inline]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_size: size / 2.0,
        }
    }

    /// Returns the length of the semi-major axis. This corresponds to the longest radius of the ellipse.
    #[inline]
    pub fn semi_major(self) -> f32 {
        self.half_size.max_element()
    }

    /// Returns the length of the semi-minor axis. This corresponds to the shortest radius of the ellipse.
    #[inline]
    pub fn semi_minor(self) -> f32 {
        self.half_size.min_element()
    }
}

/// An unbounded plane in 2D space. It forms a separating surface through the origin,
/// stretching infinitely far
#[derive(Clone, Copy, Debug)]
pub struct Plane2d {
    /// The normal of the plane. The plane will be placed perpendicular to this direction
    pub normal: Direction2d,
}
impl Primitive2d for Plane2d {}

impl Default for Plane2d {
    /// Returns the default [`Plane2d`] with a normal pointing in the `+Y` direction.
    fn default() -> Self {
        Self {
            normal: Direction2d::Y,
        }
    }
}

impl Plane2d {
    /// Create a new `Plane2d` from a normal
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline]
    pub fn new(normal: Vec2) -> Self {
        Self {
            normal: Direction2d::new(normal).expect("normal must be nonzero and finite"),
        }
    }
}

/// An infinite line along a direction in 2D space.
///
/// For a finite line: [`Segment2d`]
#[derive(Clone, Copy, Debug)]
pub struct Line2d {
    /// The direction of the line. The line extends infinitely in both the given direction
    /// and its opposite direction
    pub direction: Direction2d,
}
impl Primitive2d for Line2d {}

/// A segment of a line along a direction in 2D space.
#[doc(alias = "LineSegment2d")]
#[derive(Clone, Copy, Debug)]
pub struct Segment2d {
    /// The direction of the line segment
    pub direction: Direction2d,
    /// Half the length of the line segment. The segment extends by this amount in both
    /// the given direction and its opposite direction
    pub half_length: f32,
}
impl Primitive2d for Segment2d {}

impl Segment2d {
    /// Create a line segment from a direction and full length of the segment
    pub fn new(direction: Direction2d, length: f32) -> Self {
        Self {
            direction,
            half_length: length / 2.,
        }
    }

    /// Get a line segment and translation from two points at each end of a line segment
    ///
    /// Panics if point1 == point2
    pub fn from_points(point1: Vec2, point2: Vec2) -> (Self, Vec2) {
        let diff = point2 - point1;
        let length = diff.length();
        (
            Self::new(Direction2d::from_normalized(diff / length), length),
            (point1 + point2) / 2.,
        )
    }

    /// Get the position of the first point on the line segment
    pub fn point1(&self) -> Vec2 {
        *self.direction * -self.half_length
    }

    /// Get the position of the second point on the line segment
    pub fn point2(&self) -> Vec2 {
        *self.direction * self.half_length
    }
}

/// A series of connected line segments in 2D space.
///
/// For a version without generics: [`BoxedPolyline2d`]
#[derive(Clone, Debug)]
pub struct Polyline2d<const N: usize> {
    /// The vertices of the polyline
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polyline2d<N> {}

impl<const N: usize> FromIterator<Vec2> for Polyline2d<N> {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let mut vertices: [Vec2; N] = [Vec2::ZERO; N];

        for (index, i) in iter.into_iter().take(N).enumerate() {
            vertices[index] = i;
        }
        Self { vertices }
    }
}

impl<const N: usize> Polyline2d<N> {
    /// Create a new `Polyline2d` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A series of connected line segments in 2D space, allocated on the heap
/// in a `Box<[Vec2]>`.
///
/// For a version without alloc: [`Polyline2d`]
#[derive(Clone, Debug)]
pub struct BoxedPolyline2d {
    /// The vertices of the polyline
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolyline2d {}

impl FromIterator<Vec2> for BoxedPolyline2d {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let vertices: Vec<Vec2> = iter.into_iter().collect();
        Self {
            vertices: vertices.into_boxed_slice(),
        }
    }
}

impl BoxedPolyline2d {
    /// Create a new `BoxedPolyline2d` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A triangle in 2D space
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle2d {
    /// The vertices of the triangle
    pub vertices: [Vec2; 3],
}
impl Primitive2d for Triangle2d {}

impl Default for Triangle2d {
    /// Returns the default [`Triangle2d`] with the vertices `[0.0, 0.5]`, `[-0.5, -0.5]`, and `[0.5, -0.5]`.
    fn default() -> Self {
        Self {
            vertices: [Vec2::Y * 0.5, Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5)],
        }
    }
}

impl Triangle2d {
    /// Create a new `Triangle2d` from points `a`, `b`, and `c`
    pub const fn new(a: Vec2, b: Vec2, c: Vec2) -> Self {
        Self {
            vertices: [a, b, c],
        }
    }

    /// Get the [`WindingOrder`] of the triangle
    #[doc(alias = "orientation")]
    pub fn winding_order(&self) -> WindingOrder {
        let [a, b, c] = self.vertices;
        let area = (b - a).perp_dot(c - a);
        if area > f32::EPSILON {
            WindingOrder::CounterClockwise
        } else if area < -f32::EPSILON {
            WindingOrder::Clockwise
        } else {
            WindingOrder::Invalid
        }
    }

    /// Compute the circle passing through all three vertices of the triangle.
    /// The vector in the returned tuple is the circumcenter.
    pub fn circumcircle(&self) -> (Circle, Vec2) {
        // We treat the triangle as translated so that vertex A is at the origin. This simplifies calculations.
        //
        //     A = (0, 0)
        //        *
        //       / \
        //      /   \
        //     /     \
        //    /       \
        //   /    U    \
        //  /           \
        // *-------------*
        // B             C

        let a = self.vertices[0];
        let (b, c) = (self.vertices[1] - a, self.vertices[2] - a);
        let b_length_sq = b.length_squared();
        let c_length_sq = c.length_squared();

        // Reference: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2
        let inv_d = (2.0 * (b.x * c.y - b.y * c.x)).recip();
        let ux = inv_d * (c.y * b_length_sq - b.y * c_length_sq);
        let uy = inv_d * (b.x * c_length_sq - c.x * b_length_sq);
        let u = Vec2::new(ux, uy);

        // Compute true circumcenter and circumradius, adding the tip coordinate so that
        // A is translated back to its actual coordinate.
        let center = u + a;
        let radius = u.length();

        (Circle { radius }, center)
    }

    /// Reverse the [`WindingOrder`] of the triangle
    /// by swapping the second and third vertices
    pub fn reverse(&mut self) {
        self.vertices.swap(1, 2);
    }
}

/// A rectangle primitive
#[doc(alias = "Quad")]
#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    /// Half of the width and height of the rectangle
    pub half_size: Vec2,
}

impl Default for Rectangle {
    /// Returns the default [`Rectangle`] with a half-width and half-height of `0.5`.
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(0.5),
        }
    }
}

impl Rectangle {
    /// Create a rectangle from a full width and height
    pub fn new(width: f32, height: f32) -> Self {
        Self::from_size(Vec2::new(width, height))
    }

    /// Create a rectangle from a given full size
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_size: size / 2.,
        }
    }
}

/// A polygon with N vertices.
///
/// For a version without generics: [`BoxedPolygon`]
#[derive(Clone, Debug)]
pub struct Polygon<const N: usize> {
    /// The vertices of the `Polygon`
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polygon<N> {}

impl<const N: usize> FromIterator<Vec2> for Polygon<N> {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let mut vertices: [Vec2; N] = [Vec2::ZERO; N];

        for (index, i) in iter.into_iter().take(N).enumerate() {
            vertices[index] = i;
        }
        Self { vertices }
    }
}

impl<const N: usize> Polygon<N> {
    /// Create a new `Polygon` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A polygon with a variable number of vertices, allocated on the heap
/// in a `Box<[Vec2]>`.
///
/// For a version without alloc: [`Polygon`]
#[derive(Clone, Debug)]
pub struct BoxedPolygon {
    /// The vertices of the `BoxedPolygon`
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolygon {}

impl FromIterator<Vec2> for BoxedPolygon {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let vertices: Vec<Vec2> = iter.into_iter().collect();
        Self {
            vertices: vertices.into_boxed_slice(),
        }
    }
}

impl BoxedPolygon {
    /// Create a new `BoxedPolygon` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A polygon where all vertices lie on a circle, equally far apart.
#[derive(Clone, Copy, Debug)]
pub struct RegularPolygon {
    /// The circumcircle on which all vertices lie
    pub circumcircle: Circle,
    /// The number of sides
    pub sides: usize,
}
impl Primitive2d for RegularPolygon {}

impl Default for RegularPolygon {
    /// Returns the default [`RegularPolygon`] with six sides (a hexagon) and a circumradius of `0.5`.
    fn default() -> Self {
        Self {
            circumcircle: Circle { radius: 0.5 },
            sides: 6,
        }
    }
}

impl RegularPolygon {
    /// Create a new `RegularPolygon`
    /// from the radius of the circumcircle and a number of sides
    ///
    /// # Panics
    ///
    /// Panics if `circumradius` is non-positive
    pub fn new(circumradius: f32, sides: usize) -> Self {
        assert!(circumradius > 0.0);
        Self {
            circumcircle: Circle {
                radius: circumradius,
            },
            sides,
        }
    }

    /// Returns an iterator over the vertices of the regular polygon,
    /// rotated counterclockwise by the given angle in radians.
    ///
    /// With a rotation of 0, a vertex will be placed at the top `(0.0, circumradius)`.
    pub fn vertices(self, rotation: f32) -> impl IntoIterator<Item = Vec2> {
        // Add pi/2 so that the polygon has a vertex at the top (sin is 1.0 and cos is 0.0)
        let start_angle = rotation + std::f32::consts::FRAC_PI_2;
        let step = std::f32::consts::TAU / self.sides as f32;

        (0..self.sides).map(move |i| {
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            Vec2::new(cos, sin) * self.circumcircle.radius
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_creation() {
        assert_eq!(Direction2d::new(Vec2::X * 12.5), Ok(Direction2d::X));
        assert_eq!(
            Direction2d::new(Vec2::new(0.0, 0.0)),
            Err(InvalidDirectionError::Zero)
        );
        assert_eq!(
            Direction2d::new(Vec2::new(f32::INFINITY, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction2d::new(Vec2::new(f32::NEG_INFINITY, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction2d::new(Vec2::new(f32::NAN, 0.0)),
            Err(InvalidDirectionError::NaN)
        );
        assert_eq!(
            Direction2d::new_and_length(Vec2::X * 6.5),
            Ok((Direction2d::from_normalized(Vec2::X), 6.5))
        );
    }

    #[test]
    fn triangle_winding_order() {
        let mut cw_triangle = Triangle2d::new(
            Vec2::new(0.0, 2.0),
            Vec2::new(-0.5, -1.2),
            Vec2::new(-1.0, -1.0),
        );
        assert_eq!(cw_triangle.winding_order(), WindingOrder::Clockwise);

        let ccw_triangle = Triangle2d::new(
            Vec2::new(0.0, 2.0),
            Vec2::new(-1.0, -1.0),
            Vec2::new(-0.5, -1.2),
        );
        assert_eq!(ccw_triangle.winding_order(), WindingOrder::CounterClockwise);

        // The clockwise triangle should be the same as the counterclockwise
        // triangle when reversed
        cw_triangle.reverse();
        assert_eq!(cw_triangle, ccw_triangle);

        let invalid_triangle = Triangle2d::new(
            Vec2::new(0.0, 2.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, -1.2),
        );
        assert_eq!(invalid_triangle.winding_order(), WindingOrder::Invalid);
    }

    #[test]
    fn triangle_circumcenter() {
        let triangle = Triangle2d::new(
            Vec2::new(10.0, 2.0),
            Vec2::new(-5.0, -3.0),
            Vec2::new(2.0, -1.0),
        );
        let (Circle { radius }, circumcenter) = triangle.circumcircle();

        // Calculated with external calculator
        assert_eq!(radius, 98.34887);
        assert_eq!(circumcenter, Vec2::new(-28.5, 92.5));
    }

    #[test]
    fn regular_polygon_vertices() {
        let polygon = RegularPolygon::new(1.0, 4);

        // Regular polygons have a vertex at the top by default
        let mut vertices = polygon.vertices(0.0).into_iter();
        assert!((vertices.next().unwrap() - Vec2::Y).length() < 1e-7);

        // Rotate by 45 degrees, forming an axis-aligned square
        let mut rotated_vertices = polygon.vertices(std::f32::consts::FRAC_PI_4).into_iter();

        // Distance from the origin to the middle of a side, derived using Pythagorean theorem
        let side_sistance = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (rotated_vertices.next().unwrap() - Vec2::new(-side_sistance, side_sistance)).length()
                < 1e-7,
        );
    }
}
