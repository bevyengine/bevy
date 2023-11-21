use std::f32::consts::PI;

use super::{Primitive2d, WindingOrder};
use crate::Vec2;

/// A normalized vector pointing in a direction in 2D space
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction2d(Vec2);

impl From<Vec2> for Direction2d {
    fn from(value: Vec2) -> Self {
        Self(value.normalize())
    }
}

impl Direction2d {
    /// Create a direction from a [`Vec2`] that is already normalized
    pub fn from_normalized(value: Vec2) -> Self {
        debug_assert!(value.is_normalized());
        Self(value)
    }
}

impl std::ops::Deref for Direction2d {
    type Target = Vec2;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A circle primitive
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// The radius of the circle
    pub radius: f32,
}
impl Primitive2d for Circle {}

impl Circle {
    /// Get the diameter of the circle
    pub fn diameter(&self) -> f32 {
        2.0 * self.radius
    }

    /// Get the area of the circle
    pub fn area(&self) -> f32 {
        PI * self.radius.powi(2)
    }

    /// Get the perimeter or circumference of the circle
    #[doc(alias = "circumference")]
    pub fn perimeter(&self) -> f32 {
        2.0 * PI * self.radius
    }
}

/// An ellipse primitive
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipse {
    /// The half "width" of the ellipse
    pub half_width: f32,
    /// The half "height" of the ellipse
    pub half_height: f32,
}
impl Primitive2d for Ellipse {}

impl Ellipse {
    /// Create a new `Ellipse` from a "width" and a "height"
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            half_width: width / 2.0,
            half_height: height / 2.0,
        }
    }

    /// Get the area of the ellipse
    pub fn area(&self) -> f32 {
        PI * self.half_width * self.half_height
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
            half_length: length / 2.0,
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
            (point1 + point2) / 2.0,
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

impl Triangle2d {
    /// Create a new `Triangle2d` from points `a`, `b`, and `c`
    pub fn new(a: Vec2, b: Vec2, c: Vec2) -> Self {
        Self {
            vertices: [a, b, c],
        }
    }

    /// Get the area of the triangle
    pub fn area(&self) -> f32 {
        let [a, b, c] = self.vertices;
        (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)).abs() / 2.0
    }

    /// Get the perimeter of the triangle
    pub fn perimeter(&self) -> f32 {
        let [a, b, c] = self.vertices;

        let ab = a.distance(b);
        let bc = b.distance(c);
        let ca = c.distance(a);

        ab + bc + ca
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

    /// Reverse the [`WindingOrder`] of the triangle
    /// by swapping the second and third vertices
    pub fn reverse(&mut self) {
        self.vertices.swap(1, 2);
    }
}

/// A rectangle primitive
#[doc(alias = "Quad")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    /// The half width of the rectangle
    pub half_width: f32,
    /// The half height of the rectangle
    pub half_height: f32,
}
impl Primitive2d for Rectangle {}

impl Rectangle {
    /// Create a new `Rectangle` from a full width and height
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            half_width: width / 2.0,
            half_height: height / 2.0,
        }
    }

    /// Create a new `Rectangle` from a given full size
    pub fn from_size(size: Vec2) -> Self {
        Self::from_half_size(size / 2.0)
    }

    /// Create a new `Rectangle` from a given half-size
    pub fn from_half_size(half_size: Vec2) -> Self {
        Self {
            half_width: half_size.x,
            half_height: half_size.y,
        }
    }

    /// Create a new `Rectangle` from two corner points
    pub fn from_corners(point1: Vec2, point2: Vec2) -> Self {
        Self {
            half_width: (point2.x - point1.x).abs() / 2.0,
            half_height: (point2.y - point1.y).abs() / 2.0,
        }
    }

    /// Get the size of the rectangle
    pub fn size(&self) -> Vec2 {
        Vec2::new(2.0 * self.half_width, 2.0 * self.half_height)
    }

    /// Get the half-size of the rectangle
    pub fn half_size(&self) -> Vec2 {
        Vec2::new(self.half_width, self.half_height)
    }

    /// Get the area of the rectangle
    pub fn area(&self) -> f32 {
        4.0 * self.half_width * self.half_height
    }

    /// Get the perimeter of the rectangle
    pub fn perimeter(&self) -> f32 {
        4.0 * (self.half_width + self.half_height)
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

/// A polygon where all vertices lie on a circle, equally far apart
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegularPolygon {
    /// The circumcircle on which all vertices lie
    pub circumcircle: Circle,
    /// The number of sides
    pub sides: usize,
}
impl Primitive2d for RegularPolygon {}

impl RegularPolygon {
    /// Create a new `RegularPolygon`
    /// from the radius of the circumcircle and number of sides
    ///
    /// # Panics
    ///
    /// Panics if `circumradius` is non-positive
    pub fn new(circumradius: f32, sides: usize) -> Self {
        assert!(circumradius > 0.0, "polygon has a non-positive radius");
        assert!(sides > 2, "polygon has less than 3 sides");

        Self {
            circumcircle: Circle {
                radius: circumradius,
            },
            sides,
        }
    }

    /// Get the radius of the circumcircle on which all vertices
    /// of the regular polygon lie
    pub fn circumradius(&self) -> f32 {
        self.circumcircle.radius
    }

    /// Get the inradius or apothem of the regular polygon.
    /// This is the radius of the largest circle that can
    /// be drawn within the polygon
    #[doc(alias = "apothem")]
    pub fn inradius(&self) -> f32 {
        self.circumradius() * (PI / self.sides as f32).cos()
    }

    /// Get the length of one side of the regular polygon
    pub fn side_length(&self) -> f32 {
        2.0 * self.circumradius() * (PI / self.sides as f32).sin()
    }

    /// Get the area of the regular polygon
    pub fn area(&self) -> f32 {
        let angle: f32 = 2.0 * PI / (self.sides as f32);
        (self.sides as f32) * self.circumradius().powi(2) * angle.sin() / 2.0
    }

    /// Get the perimeter of the regular polygon.
    /// This is the sum of its sides
    pub fn perimeter(&self) -> f32 {
        self.sides as f32 * self.side_length()
    }

    /// Get the internal angle of the regular polygon in degrees.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the interior of the polygon
    pub fn internal_angle_degrees(&self) -> f32 {
        (self.sides - 2) as f32 / self.sides as f32 * 180.0
    }

    /// Get the internal angle of the regular polygon in radians.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the interior of the polygon
    pub fn internal_angle_radians(&self) -> f32 {
        (self.sides - 2) as f32 * PI / self.sides as f32
    }

    /// Get the external angle of the regular polygon in degrees.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the exterior of the polygon
    pub fn external_angle_degrees(&self) -> f32 {
        360.0 / self.sides as f32
    }

    /// Get the external angle of the regular polygon in radians.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the exterior of the polygon
    pub fn external_angle_radians(&self) -> f32 {
        2.0 * PI / self.sides as f32
    }
}

#[cfg(test)]
mod tests {
    // Reference values were computed by hand and/or with external tools

    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn circle_math() {
        let circle = Circle { radius: 3.0 };
        assert_eq!(circle.diameter(), 6.0, "incorrect diameter");
        assert_eq!(circle.area(), 28.274334, "incorrect area");
        assert_eq!(circle.perimeter(), 18.849556, "incorrect perimeter");
    }

    #[test]
    fn ellipse_math() {
        let ellipse = Ellipse::new(6.0, 2.0);
        assert_eq!(ellipse.area(), 9.424778, "incorrect area");
    }

    #[test]
    fn triangle_math() {
        let triangle = Triangle2d::new(
            Vec2::new(-2.0, -1.0),
            Vec2::new(1.0, 4.0),
            Vec2::new(7.0, 0.0),
        );
        assert_eq!(triangle.area(), 21.0, "incorrect area");
        assert_eq!(triangle.perimeter(), 22.097439, "incorrect perimeter");
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
    fn rectangle_math() {
        let rectangle = Rectangle::new(3.0, 7.0);
        assert_eq!(
            rectangle,
            Rectangle::from_corners(Vec2::new(-1.5, -3.5), Vec2::new(1.5, 3.5))
        );
        assert_eq!(rectangle.area(), 21.0, "incorrect area");
        assert_eq!(rectangle.perimeter(), 20.0, "incorrect perimeter");
    }

    #[test]
    fn regular_polygon_math() {
        let polygon = RegularPolygon::new(3.0, 6);
        assert_eq!(polygon.inradius(), 2.598076, "incorrect inradius");
        assert_eq!(polygon.side_length(), 3.0, "incorrect side length");
        assert_relative_eq!(polygon.area(), 23.38268, epsilon = 0.00001);
        assert_eq!(polygon.perimeter(), 18.0, "incorrect perimeter");
        assert_eq!(
            polygon.internal_angle_degrees(),
            120.0,
            "incorrect internal angle"
        );
        assert_eq!(
            polygon.internal_angle_radians(),
            120_f32.to_radians(),
            "incorrect internal angle"
        );
        assert_eq!(
            polygon.external_angle_degrees(),
            60.0,
            "incorrect external angle"
        );
        assert_eq!(
            polygon.external_angle_radians(),
            60_f32.to_radians(),
            "incorrect external angle"
        );
    }
}
