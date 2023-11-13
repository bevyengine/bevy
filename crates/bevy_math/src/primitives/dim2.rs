use super::Primitive2d;
use crate::Vec2;

/// A normalized vector pointing in a direction in 2D space
#[derive(Clone, Copy, Debug)]
pub struct Direction2d(Vec2);

impl From<Vec2> for Direction2d {
    fn from(value: Vec2) -> Self {
        Self(value.normalize())
    }
}

impl Direction2d {
    /// Create a direction from a [Vec2] that is already normalized
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
#[derive(Clone, Copy, Debug)]
pub struct Circle {
    /// The radius of the circle
    pub radius: f32,
}
impl Primitive2d for Circle {}

/// An unbounded plane in 2D space. It forms a separating surface trough the origin,
/// stretching infinitely far
#[derive(Clone, Copy, Debug)]
pub struct Plane2d {
    /// The normal of the plane, the plane will be placed perpendicular to this direction
    pub normal: Direction2d,
}
impl Primitive2d for Plane2d {}

/// An infinite line along a direction in 2D space.
///
/// For a finite line: [`Segment2d`]
#[derive(Clone, Copy, Debug)]
pub struct Line2d {
    /// The direction of the line, the line extends infinitely in both the positive
    /// and negative of this direction
    pub direction: Direction2d,
}
impl Primitive2d for Line2d {}

/// A segment of a line along a direction in 2D space.
#[doc(alias = "LineSegment2d")]
#[derive(Clone, Debug)]
pub struct Segment2d {
    /// The direction of the line
    pub direction: Direction2d,
    /// Half the length of the line segment, the segment extends by this amount in both
    /// the positive and negative direction
    pub half_length: f32,
}
impl Primitive2d for Segment2d {}

impl Segment2d {
    /// Create a line segment from a direction and full length of the line
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

/// A series of connected line segments in 2D space.
///
/// For a version without alloc: [`Polyline2d`]
#[derive(Clone, Debug)]
pub struct BoxedPolyline2d {
    /// The vertices of the polyline
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolyline2d {}

/// A triangle in 2d space
#[derive(Clone, Debug)]
pub struct Triangle2d {
    /// The vertices of the triangle
    pub vertices: [Vec2; 3],
}
impl Primitive2d for Triangle2d {}

/// A rectangle primitive
#[doc(alias = "Quad")]
#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    /// The half width of the rectangle
    pub half_width: f32,
    /// The half height of the rectangle
    pub half_height: f32,
}
impl Primitive2d for Rectangle {}

impl Rectangle {
    /// Create a Rectangle from a full width and height
    pub fn new(width: f32, height: f32) -> Self {
        Self::from_size(Vec2::new(width, height))
    }

    /// Create a Rectangle from the full size of a rectangle
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_width: size.x / 2.,
            half_height: size.y / 2.,
        }
    }
}

/// A polygon with N vertices
/// For a version without generics: [`BoxedPolygon`]
#[derive(Clone, Debug)]
pub struct Polygon<const N: usize> {
    /// The vertices of the polygon
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polygon<N> {}

/// A polygon with a variable number of vertices
/// For a version without alloc: [`Polygon`]
#[derive(Clone, Debug)]
pub struct BoxedPolygon {
    /// The vertices of the polygon
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolygon {}

/// A polygon where all vertices lie on a circle, equally far apart
#[derive(Clone, Copy, Debug)]
pub struct RegularPolygon {
    /// The circumcircle on which all vertices lie
    pub circumcircle: Circle,
    /// The number of vertices
    pub n_vertices: usize,
}
impl Primitive2d for RegularPolygon {}
