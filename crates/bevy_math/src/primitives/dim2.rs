use super::Primitive2d;
use crate::Vec2;

/// A normalized vector pointing in a direction in 2D space
pub struct Direction2d(Vec2);

impl From<Vec2> for Direction2d {
    fn from(value: Vec2) -> Self {
        Self(value.normalize())
    }
}

impl Direction2d {
    /// Create a direction from a [Vec2] that is already normalized
    pub fn from_normalized(value: Vec2) -> Self {
        Self(value)
    }
}

/// An infinite half-line pointing in a direction in 2D space
pub struct Ray2d(pub Direction2d);

/// An alias for [Rectangle]
pub type Quad = Rectangle;

/// A circle primitive
pub struct Circle {
    /// The radius of the circle
    pub radius: f32,
}
impl Primitive2d for Circle {}

/// An unbounded plane in 2D space
pub struct Plane2d {
    /// The direction in which the plane points
    pub normal: Direction2d,
}
impl Primitive2d for Plane2d {}

/// An infinite line along a direction in 2D space.
/// For a finite line: [LineSegment2d]
pub struct Line2d {
    /// The direction of the line
    pub direction: Direction2d,
}
impl Primitive2d for Line2d {}

/// A section of a line along a direction in 2D space.
pub struct LineSegment2d {
    /// The direction of the line
    pub direction: Direction2d,
    /// The point where the line starts
    pub start: f32,
    /// The point where the line ends
    pub end: f32,
}
impl Primitive2d for LineSegment2d {}

/// A line alone a path of N vertices in 2D space.
/// For a version without generics: [BoxedPolyline2d]
pub struct Polyline2d<const N: usize> {
    /// The vertices of the polyline
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polyline2d<N> {}

/// A line alone a path of vertices in 2D space.
/// For a version without alloc: [Polyline2d]
pub struct BoxedPolyline2d {
    /// The vertices of the polyline
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolyline2d {}

/// A triangle primitive
pub struct Triangle {
    /// The vertices of the triangle
    pub vertcies: [Vec2; 3],
}
impl Primitive2d for Triangle {}

/// A rectangle primitive
pub struct Rectangle {
    /// The half width of the rectangle
    pub half_width: f32,
    /// The half height of the rectangle
    pub half_height: f32,
}
impl Primitive2d for Rectangle {}

/// A polygon with N vertices
/// For a version without generics: [BoxedPolygon]
pub struct Polygon<const N: usize> {
    /// The vertices of the polygon
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polygon<N> {}

/// A polygon with a variable number of vertices
/// For a version without alloc: [Polygon]
pub struct BoxedPolygon {
    /// The vertices of the polygon
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolygon {}

/// A polygon where all vertices lie on a circumscribed circle, equally far apart
pub struct RegularPolygon {
    /// The circumcircle on which all vertices lie
    pub circumcircle: Circle,
    /// The number of vertices
    pub n_vertices: usize,
}
impl Primitive2d for RegularPolygon {}
