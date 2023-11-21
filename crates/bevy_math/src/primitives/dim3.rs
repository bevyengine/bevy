use std::f32::consts::{FRAC_PI_3, PI};

use super::{Circle, Primitive3d};
use crate::Vec3;

/// A normalized vector pointing in a direction in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction3d(Vec3);

impl From<Vec3> for Direction3d {
    fn from(value: Vec3) -> Self {
        Self(value.normalize())
    }
}

impl Direction3d {
    /// Create a direction from a [`Vec3`] that is already normalized
    pub fn from_normalized(value: Vec3) -> Self {
        debug_assert!(value.is_normalized());
        Self(value)
    }
}

impl std::ops::Deref for Direction3d {
    type Target = Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A sphere primitive
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// The radius of the sphere
    pub radius: f32,
}
impl Primitive3d for Sphere {}

impl Sphere {
    /// Get the diameter of the sphere
    pub fn diameter(&self) -> f32 {
        2.0 * self.radius
    }

    /// Get the surface area of the sphere
    pub fn area(&self) -> f32 {
        4.0 * PI * self.radius.powi(2)
    }

    /// Get the volume of the sphere
    pub fn volume(&self) -> f32 {
        4.0 * FRAC_PI_3 * self.radius.powi(2) * self.radius
    }
}

/// An unbounded plane in 3D space. It forms a separating surface through the origin,
/// stretching infinitely far
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane3d {
    /// The normal of the plane. The plane will be placed perpendicular to this direction
    pub normal: Direction3d,
}
impl Primitive3d for Plane3d {}

impl Plane3d {
    /// Create a new `Plane3d` and get its translation from the origin based on three points.
    /// The direction of the plane normal is determined by the winding order
    /// of the triangle formed by the points.
    ///
    /// # Panics
    ///
    /// Panics if `a == b`, `b == c` or `a == c`.
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> (Self, Vec3) {
        debug_assert!(
            a != b && b != c && a != c,
            "each corner of a triangle must be unique"
        );
        let normal = Direction3d::from((b - a).cross(c - a));
        let translation = (a + b + c) / 3.0;
        (Self { normal }, translation)
    }
}

/// An infinite line along a direction in 3D space.
///
/// For a finite line: [`Segment3d`]
#[derive(Clone, Copy, Debug)]
pub struct Line3d {
    /// The direction of the line
    pub direction: Direction3d,
}
impl Primitive3d for Line3d {}

/// A segment of a line along a direction in 3D space.
#[doc(alias = "LineSegment3d")]
#[derive(Clone, Debug)]
pub struct Segment3d {
    /// The direction of the line
    pub direction: Direction3d,
    /// Half the length of the line segment. The segment extends by this amount in both
    /// the given direction and its opposite direction
    pub half_length: f32,
}
impl Primitive3d for Segment3d {}

impl Segment3d {
    /// Create a line segment from a direction and full length of the segment
    pub fn new(direction: Direction3d, length: f32) -> Self {
        Self {
            direction,
            half_length: length / 2.0,
        }
    }

    /// Get a line segment and translation from two points at each end of a line segment
    ///
    /// # Panics
    ///
    /// Panics if `point1 == point2`
    pub fn from_points(point1: Vec3, point2: Vec3) -> (Self, Vec3) {
        let diff = point2 - point1;
        let length = diff.length();
        (
            Self::new(Direction3d::from_normalized(diff / length), length),
            (point1 + point2) / 2.0,
        )
    }

    /// Get the position of the first point on the line segment
    pub fn point1(&self) -> Vec3 {
        *self.direction * -self.half_length
    }

    /// Get the position of the second point on the line segment
    pub fn point2(&self) -> Vec3 {
        *self.direction * self.half_length
    }
}

/// A series of connected line segments in 3D space.
///
/// For a version without generics: [`BoxedPolyline3d`]
#[derive(Clone, Debug)]
pub struct Polyline3d<const N: usize> {
    /// The vertices of the polyline
    pub vertices: [Vec3; N],
}
impl<const N: usize> Primitive3d for Polyline3d<N> {}

impl<const N: usize> FromIterator<Vec3> for Polyline3d<N> {
    fn from_iter<I: IntoIterator<Item = Vec3>>(iter: I) -> Self {
        let mut vertices: [Vec3; N] = [Vec3::ZERO; N];

        for (index, i) in iter.into_iter().take(N).enumerate() {
            vertices[index] = i;
        }
        Self { vertices }
    }
}

impl<const N: usize> Polyline3d<N> {
    /// Create a new `Polyline3d` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec3>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A series of connected line segments in 3D space, allocated on the heap
/// in a `Box<[Vec3]>`.
///
/// For a version without alloc: [`Polyline3d`]
#[derive(Clone, Debug)]
pub struct BoxedPolyline3d {
    /// The vertices of the polyline
    pub vertices: Box<[Vec3]>,
}
impl Primitive3d for BoxedPolyline3d {}

impl FromIterator<Vec3> for BoxedPolyline3d {
    fn from_iter<I: IntoIterator<Item = Vec3>>(iter: I) -> Self {
        let vertices: Vec<Vec3> = iter.into_iter().collect();
        Self {
            vertices: vertices.into_boxed_slice(),
        }
    }
}

impl BoxedPolyline3d {
    /// Create a new `BoxedPolyline3d` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec3>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A cuboid primitive, more commonly known as a box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cuboid {
    /// Half of the width, height and depth of the cuboid
    pub half_size: Vec3,
}
impl Primitive3d for Cuboid {}

impl Cuboid {
    /// Create a new `Cuboid` from a full x, y, and z length
    pub fn new(x_length: f32, y_length: f32, z_length: f32) -> Self {
        Self::from_size(Vec3::new(x_length, y_length, z_length))
    }

    /// Create a new `Cuboid` from a given full size
    pub fn from_size(size: Vec3) -> Self {
        Self {
            half_size: size / 2.0,
        }
    }

    /// Create a new `Cuboid` from two corner points
    pub fn from_corners(point1: Vec3, point2: Vec3) -> Self {
        Self {
            half_size: (point2 - point1).abs() / 2.0,
        }
    }

    /// Get the size of the cuboid
    pub fn size(&self) -> Vec3 {
        2.0 * self.half_size
    }

    /// Get the surface area of the cuboid
    pub fn area(&self) -> f32 {
        8.0 * (self.half_size.x * self.half_size.y
            + self.half_size.y * self.half_size.z
            + self.half_size.x * self.half_size.z)
    }

    /// Get the volume of the cuboid
    pub fn volume(&self) -> f32 {
        8.0 * self.half_size.x * self.half_size.y * self.half_size.z
    }
}

/// A cylinder primitive
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylinder {
    /// The radius of the cylinder
    pub radius: f32,
    /// The half height of the cylinder
    pub half_height: f32,
}
impl Primitive3d for Cylinder {}

impl Cylinder {
    /// Create a new `Cylinder` from a radius and full height
    pub fn new(radius: f32, height: f32) -> Self {
        Self {
            radius,
            half_height: height / 2.0,
        }
    }

    /// Get the base of the cylinder as a [`Circle`]
    pub fn base(&self) -> Circle {
        Circle {
            radius: self.radius,
        }
    }

    /// Get the surface area of the side of the cylinder,
    /// also known as the lateral area
    #[doc(alias = "side_area")]
    pub fn lateral_area(&self) -> f32 {
        4.0 * PI * self.radius * self.half_height
    }

    /// Get the surface area of one base of the cylinder
    pub fn base_area(&self) -> f32 {
        PI * self.radius.powi(2)
    }

    /// Get the total surface area of the cylinder
    pub fn area(&self) -> f32 {
        2.0 * PI * self.radius * (self.radius + 2.0 * self.half_height)
    }

    /// Get the volume of the cylinder
    pub fn volume(&self) -> f32 {
        self.base_area() * 2.0 * self.half_height
    }
}

/// A capsule primitive.
/// A capsule is defined as a surface at a distance (radius) from a line
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    /// The radius of the capsule
    pub radius: f32,
    /// Half the height of the capsule, excluding the hemispheres
    pub half_length: f32,
}
impl super::Primitive2d for Capsule {}
impl Primitive3d for Capsule {}

impl Capsule {
    /// Create a new `Capsule` from a radius and length
    pub fn new(radius: f32, length: f32) -> Self {
        Self {
            radius,
            half_length: length / 2.0,
        }
    }

    /// Get the part connecting the hemispherical ends
    /// of the capsule as a [`Cylinder`]
    pub fn to_cylinder(&self) -> Cylinder {
        Cylinder {
            radius: self.radius,
            half_height: self.half_length,
        }
    }

    /// Get the surface area of the capsule
    pub fn area(&self) -> f32 {
        // Modified version of 2pi * r * (2r + h)
        4.0 * PI * self.radius * (self.radius + self.half_length)
    }

    /// Get the volume of the capsule
    pub fn volume(&self) -> f32 {
        // Modified version of pi * r^2 * (4/3 * r + a)
        let diameter = self.radius * 2.0;
        PI * self.radius * diameter * (diameter / 3.0 + self.half_length)
    }
}

/// A cone primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cone {
    /// The radius of the base
    pub radius: f32,
    /// The height of the cone
    pub height: f32,
}
impl Primitive3d for Cone {}

impl Cone {
    /// Get the base of the cone as a [`Circle`]
    pub fn base(&self) -> Circle {
        Circle {
            radius: self.radius,
        }
    }

    /// Get the slant height of the cone, the length of the line segment
    /// connecting a point on the base to the apex
    #[doc(alias = "side_length")]
    pub fn slant_height(&self) -> f32 {
        self.radius.hypot(self.height)
    }

    /// Get the surface area of the side of the cone,
    /// also known as the lateral area
    #[doc(alias = "side_area")]
    pub fn lateral_area(&self) -> f32 {
        PI * self.radius * self.slant_height()
    }

    /// Get the surface area of the base of the cone
    pub fn base_area(&self) -> f32 {
        PI * self.radius.powi(2)
    }

    /// Get the total surface area of the cone
    pub fn area(&self) -> f32 {
        self.base_area() + self.lateral_area()
    }

    /// Get the volume of the cone
    pub fn volume(&self) -> f32 {
        (self.base_area() * self.height) / 3.0
    }
}

/// A conical frustum primitive.
/// A conical frustum can be created
/// by slicing off a section of a cone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConicalFrustum {
    /// The radius of the top of the frustum
    pub radius_top: f32,
    /// The radius of the base of the frustum
    pub radius_bottom: f32,
    /// The height of the frustum
    pub height: f32,
}
impl Primitive3d for ConicalFrustum {}

/// The type of torus determined by the minor and major radii
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TorusKind {
    /// A torus that has a ring.
    /// The major radius is greater than the minor radius
    Ring,
    /// A torus that has no hole but also doesn't intersect itself.
    /// The major radius is equal to the minor radius
    Horn,
    /// A self-intersecting torus.
    /// The major radius is less than the minor radius
    Spindle,
    /// A torus with non-geometric properties like
    /// a minor or major radius that is non-positive,
    /// infinite, or `NaN`
    Invalid,
}

/// A torus primitive, often representing a ring or donut shape
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Torus {
    /// The radius of the tube of the torus
    #[doc(
        alias = "ring_radius",
        alias = "tube_radius",
        alias = "cross_section_radius"
    )]
    pub minor_radius: f32,
    /// The distance from the center of the torus to the center of the tube
    #[doc(alias = "radius_of_revolution")]
    pub major_radius: f32,
}
impl Primitive3d for Torus {}

impl Torus {
    /// Create a new `Torus` from an inner and outer radius.
    ///
    /// The inner radius is the radius of the hole, and the outer radius
    /// is the radius of the entire object
    pub fn new(inner_radius: f32, outer_radius: f32) -> Self {
        let minor_radius = (outer_radius - inner_radius) / 2.0;
        let major_radius = outer_radius - minor_radius;

        Self {
            minor_radius,
            major_radius,
        }
    }

    /// Get the inner radius of the torus.
    /// For a ring torus, this corresponds to the radius of the hole,
    /// or `major_radius - minor_radius`
    #[inline]
    pub fn inner_radius(&self) -> f32 {
        self.major_radius - self.minor_radius
    }

    /// Get the outer radius of the torus.
    /// This corresponds to the overall radius of the entire object,
    /// or `major_radius + minor_radius`
    #[inline]
    pub fn outer_radius(&self) -> f32 {
        self.major_radius + self.minor_radius
    }

    /// Get the [`TorusKind`] determined by the minor and major radii.
    ///
    /// The torus can either be a *ring torus* that has a hole,
    /// a *horn torus* that doesn't have a hole but also isn't self-intersecting,
    /// or a *spindle torus* that is self-intersecting.
    ///
    /// If the minor or major radius is non-positive, infinite, or `NaN`,
    /// [`TorusKind::Invalid`] is returned
    #[inline]
    pub fn kind(&self) -> TorusKind {
        // Invalid if minor or major radius is non-positive, infinite, or NaN
        if self.minor_radius <= 0.0
            || !self.minor_radius.is_finite()
            || self.major_radius <= 0.0
            || !self.major_radius.is_finite()
        {
            return TorusKind::Invalid;
        }

        match self.major_radius.partial_cmp(&self.minor_radius).unwrap() {
            std::cmp::Ordering::Greater => TorusKind::Ring,
            std::cmp::Ordering::Equal => TorusKind::Horn,
            std::cmp::Ordering::Less => TorusKind::Spindle,
        }
    }

    /// Get the surface area of the torus. Note that this only produces
    /// the expected result when the torus has a ring and isn't self-intersecting
    pub fn area(&self) -> f32 {
        4.0 * PI.powi(2) * self.major_radius * self.minor_radius
    }

    /// Get the volume of the torus. Note that this only produces
    /// the expected result when the torus has a ring and isn't self-intersecting
    pub fn volume(&self) -> f32 {
        2.0 * PI.powi(2) * self.major_radius * self.minor_radius.powi(2)
    }
}

#[cfg(test)]
mod tests {
    // Reference values were computed by hand and/or with external tools

    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn sphere_math() {
        let sphere = Sphere { radius: 4.0 };
        assert_eq!(sphere.diameter(), 8.0, "incorrect diameter");
        assert_eq!(sphere.area(), 201.06193, "incorrect area");
        assert_eq!(sphere.volume(), 268.08257, "incorrect volume");
    }

    #[test]
    fn plane_from_points() {
        let (plane, translation) = Plane3d::from_points(Vec3::X, Vec3::Z, Vec3::NEG_X);
        assert_eq!(*plane.normal, Vec3::NEG_Y, "incorrect normal");
        assert_eq!(translation, Vec3::Z * 0.33333334, "incorrect translation");
    }

    #[test]
    fn cuboid_math() {
        let cuboid = Cuboid::new(3.0, 7.0, 2.0);
        assert_eq!(
            cuboid,
            Cuboid::from_corners(Vec3::new(-1.5, -3.5, -1.0), Vec3::new(1.5, 3.5, 1.0)),
            "incorrect dimensions when created from corners"
        );
        assert_eq!(cuboid.area(), 82.0, "incorrect area");
        assert_eq!(cuboid.volume(), 42.0, "incorrect volume");
    }

    #[test]
    fn cylinder_math() {
        let cylinder = Cylinder::new(2.0, 9.0);
        assert_eq!(
            cylinder.base(),
            Circle { radius: 2.0 },
            "base produces incorrect circle"
        );
        assert_eq!(
            cylinder.lateral_area(),
            113.097336,
            "incorrect lateral area"
        );
        assert_eq!(cylinder.base_area(), 12.566371, "incorrect base area");
        assert_relative_eq!(cylinder.area(), 138.23007);
        assert_eq!(cylinder.volume(), 113.097336, "incorrect volume");
    }

    #[test]
    fn capsule_math() {
        let capsule = Capsule::new(2.0, 9.0);
        assert_eq!(
            capsule.to_cylinder(),
            Cylinder::new(2.0, 9.0),
            "cylinder wasn't created correctly from a capsule"
        );
        assert_eq!(capsule.area(), 163.36282, "incorrect area");
        assert_relative_eq!(capsule.volume(), 146.60765);
    }

    #[test]
    fn cone_math() {
        let cone = Cone {
            radius: 2.0,
            height: 9.0,
        };
        assert_eq!(
            cone.base(),
            Circle { radius: 2.0 },
            "base produces incorrect circle"
        );
        assert_eq!(cone.slant_height(), 9.219544, "incorrect slant height");
        assert_eq!(cone.lateral_area(), 57.92811, "incorrect lateral area");
        assert_eq!(cone.base_area(), 12.566371, "incorrect base area");
        assert_relative_eq!(cone.area(), 70.49447);
        assert_eq!(cone.volume(), 37.699111, "incorrect volume");
    }

    #[test]
    fn torus_math() {
        let torus = Torus {
            minor_radius: 0.3,
            major_radius: 2.8,
        };
        assert_eq!(torus.inner_radius(), 2.5, "incorrect inner radius");
        assert_eq!(torus.outer_radius(), 3.1, "incorrect outer radius");
        assert_eq!(torus.kind(), TorusKind::Ring, "incorrect torus kind");
        assert_eq!(
            Torus::new(0.0, 1.0).kind(),
            TorusKind::Horn,
            "incorrect torus kind"
        );
        assert_eq!(
            Torus::new(-0.5, 1.0).kind(),
            TorusKind::Spindle,
            "incorrect torus kind"
        );
        assert_eq!(
            Torus::new(1.5, 1.0).kind(),
            TorusKind::Invalid,
            "torus should be invalid"
        );
        assert_relative_eq!(torus.area(), 33.16187);
        assert_relative_eq!(torus.volume(), 4.97428, epsilon = 0.00001);
    }
}
