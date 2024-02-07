use std::f32::consts::{FRAC_PI_3, PI};

use super::{Circle, InvalidDirectionError, Primitive3d};
use crate::{Quat, Vec3};

/// A normalized vector pointing in a direction in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Direction3d(Vec3);
impl Primitive3d for Direction3d {}

impl Direction3d {
    /// A unit vector pointing along the positive X axis.
    pub const X: Self = Self(Vec3::X);
    /// A unit vector pointing along the positive Y axis.
    pub const Y: Self = Self(Vec3::Y);
    /// A unit vector pointing along the positive Z axis.
    pub const Z: Self = Self(Vec3::Z);
    /// A unit vector pointing along the negative X axis.
    pub const NEG_X: Self = Self(Vec3::NEG_X);
    /// A unit vector pointing along the negative Y axis.
    pub const NEG_Y: Self = Self(Vec3::NEG_Y);
    /// A unit vector pointing along the negative Z axis.
    pub const NEG_Z: Self = Self(Vec3::NEG_Z);

    /// Create a direction from a finite, nonzero [`Vec3`].
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new(value: Vec3) -> Result<Self, InvalidDirectionError> {
        Self::new_and_length(value).map(|(dir, _)| dir)
    }

    /// Create a [`Direction3d`] from a [`Vec3`] that is already normalized.
    ///
    /// # Warning
    ///
    /// `value` must be normalized, i.e it's length must be `1.0`.
    pub fn new_unchecked(value: Vec3) -> Self {
        debug_assert!(value.is_normalized());

        Self(value)
    }

    /// Create a direction from a finite, nonzero [`Vec3`], also returning its original length.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new_and_length(value: Vec3) -> Result<(Self, f32), InvalidDirectionError> {
        let length = value.length();
        let direction = (length.is_finite() && length > 0.0).then_some(value / length);

        direction
            .map(|dir| (Self(dir), length))
            .ok_or(InvalidDirectionError::from_length(length))
    }

    /// Create a direction from its `x`, `y`, and `z` components.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the vector formed by the components is zero (or very close to zero), infinite, or `NaN`.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Result<Self, InvalidDirectionError> {
        Self::new(Vec3::new(x, y, z))
    }
}

impl TryFrom<Vec3> for Direction3d {
    type Error = InvalidDirectionError;

    fn try_from(value: Vec3) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Direction3d> for Vec3 {
    fn from(value: Direction3d) -> Self {
        value.0
    }
}

impl std::ops::Deref for Direction3d {
    type Target = Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Neg for Direction3d {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl std::ops::Mul<f32> for Direction3d {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Self::Output {
        self.0 * rhs
    }
}

impl std::ops::Mul<Direction3d> for Quat {
    type Output = Direction3d;

    /// Rotates the [`Direction3d`] using a [`Quat`].
    fn mul(self, direction: Direction3d) -> Self::Output {
        let rotated = self * *direction;

        // Make sure the result is normalized.
        // This can fail for non-unit quaternions.
        debug_assert!(rotated.is_normalized());

        Direction3d::new_unchecked(rotated)
    }
}

#[cfg(feature = "approx")]
impl approx::AbsDiffEq for Direction3d {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.as_ref().abs_diff_eq(other.as_ref(), epsilon)
    }
}

#[cfg(feature = "approx")]
impl approx::RelativeEq for Direction3d {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.as_ref()
            .relative_eq(other.as_ref(), epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl approx::UlpsEq for Direction3d {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.as_ref().ulps_eq(other.as_ref(), epsilon, max_ulps)
    }
}

/// A sphere primitive
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Sphere {
    /// The radius of the sphere
    pub radius: f32,
}
impl Primitive3d for Sphere {}

impl Default for Sphere {
    /// Returns the default [`Sphere`] with a radius of `0.5`.
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

impl Sphere {
    /// Create a new [`Sphere`] from a `radius`
    #[inline(always)]
    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }

    /// Get the diameter of the sphere
    #[inline(always)]
    pub fn diameter(&self) -> f32 {
        2.0 * self.radius
    }

    /// Get the surface area of the sphere
    #[inline(always)]
    pub fn area(&self) -> f32 {
        4.0 * PI * self.radius.powi(2)
    }

    /// Get the volume of the sphere
    #[inline(always)]
    pub fn volume(&self) -> f32 {
        4.0 * FRAC_PI_3 * self.radius.powi(3)
    }

    /// Finds the point on the sphere that is closest to the given `point`.
    ///
    /// If the point is outside the sphere, the returned point will be on the surface of the sphere.
    /// Otherwise, it will be inside the sphere and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        let distance_squared = point.length_squared();

        if distance_squared <= self.radius.powi(2) {
            // The point is inside the sphere.
            point
        } else {
            // The point is outside the sphere.
            // Find the closest point on the surface of the sphere.
            let dir_to_point = point / distance_squared.sqrt();
            self.radius * dir_to_point
        }
    }
}

/// An unbounded plane in 3D space. It forms a separating surface through the origin,
/// stretching infinitely far
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Plane3d {
    /// The normal of the plane. The plane will be placed perpendicular to this direction
    pub normal: Direction3d,
}
impl Primitive3d for Plane3d {}

impl Default for Plane3d {
    /// Returns the default [`Plane3d`] with a normal pointing in the `+Y` direction.
    fn default() -> Self {
        Self {
            normal: Direction3d::Y,
        }
    }
}

impl Plane3d {
    /// Create a new `Plane3d` from a normal
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline(always)]
    pub fn new(normal: Vec3) -> Self {
        Self {
            normal: Direction3d::new(normal).expect("normal must be nonzero and finite"),
        }
    }

    /// Create a new `Plane3d` based on three points and compute the geometric center
    /// of those points.
    ///
    /// The direction of the plane normal is determined by the winding order
    /// of the triangular shape formed by the points.
    ///
    /// # Panics
    ///
    /// Panics if a valid normal can not be computed, for example when the points
    /// are *collinear* and lie on the same line.
    #[inline(always)]
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> (Self, Vec3) {
        let normal = Direction3d::new((b - a).cross(c - a))
            .expect("plane must be defined by three finite points that don't lie on the same line");
        let translation = (a + b + c) / 3.0;

        (Self { normal }, translation)
    }
}

/// An infinite line along a direction in 3D space.
///
/// For a finite line: [`Segment3d`]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Line3d {
    /// The direction of the line
    pub direction: Direction3d,
}
impl Primitive3d for Line3d {}

/// A segment of a line along a direction in 3D space.
#[doc(alias = "LineSegment3d")]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Segment3d {
    /// The direction of the line
    pub direction: Direction3d,
    /// Half the length of the line segment. The segment extends by this amount in both
    /// the given direction and its opposite direction
    pub half_length: f32,
}
impl Primitive3d for Segment3d {}

impl Segment3d {
    /// Create a new `Segment3d` from a direction and full length of the segment
    #[inline(always)]
    pub fn new(direction: Direction3d, length: f32) -> Self {
        Self {
            direction,
            half_length: length / 2.0,
        }
    }

    /// Create a new `Segment3d` from its endpoints and compute its geometric center
    ///
    /// # Panics
    ///
    /// Panics if `point1 == point2`
    #[inline(always)]
    pub fn from_points(point1: Vec3, point2: Vec3) -> (Self, Vec3) {
        let diff = point2 - point1;
        let length = diff.length();

        (
            // We are dividing by the length here, so the vector is normalized.
            Self::new(Direction3d::new_unchecked(diff / length), length),
            (point1 + point2) / 2.,
        )
    }

    /// Get the position of the first point on the line segment
    #[inline(always)]
    pub fn point1(&self) -> Vec3 {
        *self.direction * -self.half_length
    }

    /// Get the position of the second point on the line segment
    #[inline(always)]
    pub fn point2(&self) -> Vec3 {
        *self.direction * self.half_length
    }
}

/// A series of connected line segments in 3D space.
///
/// For a version without generics: [`BoxedPolyline3d`]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline3d<const N: usize> {
    /// The vertices of the polyline
    #[cfg_attr(feature = "serialize", serde(with = "super::serde::array"))]
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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Cuboid {
    /// Half of the width, height and depth of the cuboid
    pub half_size: Vec3,
}
impl Primitive3d for Cuboid {}

impl Default for Cuboid {
    /// Returns the default [`Cuboid`] with a width, height, and depth of `1.0`.
    fn default() -> Self {
        Self {
            half_size: Vec3::splat(0.5),
        }
    }
}

impl Cuboid {
    /// Create a new `Cuboid` from a full x, y, and z length
    #[inline(always)]
    pub fn new(x_length: f32, y_length: f32, z_length: f32) -> Self {
        Self::from_size(Vec3::new(x_length, y_length, z_length))
    }

    /// Create a new `Cuboid` from a given full size
    #[inline(always)]
    pub fn from_size(size: Vec3) -> Self {
        Self {
            half_size: size / 2.0,
        }
    }

    /// Create a new `Cuboid` from two corner points
    #[inline(always)]
    pub fn from_corners(point1: Vec3, point2: Vec3) -> Self {
        Self {
            half_size: (point2 - point1).abs() / 2.0,
        }
    }

    /// Get the size of the cuboid
    #[inline(always)]
    pub fn size(&self) -> Vec3 {
        2.0 * self.half_size
    }

    /// Get the surface area of the cuboid
    #[inline(always)]
    pub fn area(&self) -> f32 {
        8.0 * (self.half_size.x * self.half_size.y
            + self.half_size.y * self.half_size.z
            + self.half_size.x * self.half_size.z)
    }

    /// Get the volume of the cuboid
    #[inline(always)]
    pub fn volume(&self) -> f32 {
        8.0 * self.half_size.x * self.half_size.y * self.half_size.z
    }

    /// Finds the point on the cuboid that is closest to the given `point`.
    ///
    /// If the point is outside the cuboid, the returned point will be on the surface of the cuboid.
    /// Otherwise, it will be inside the cuboid and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        // Clamp point coordinates to the cuboid
        point.clamp(-self.half_size, self.half_size)
    }
}

/// A cylinder primitive
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Cylinder {
    /// The radius of the cylinder
    pub radius: f32,
    /// The half height of the cylinder
    pub half_height: f32,
}
impl Primitive3d for Cylinder {}

impl Default for Cylinder {
    /// Returns the default [`Cylinder`] with a radius of `0.5` and a height of `1.0`.
    fn default() -> Self {
        Self {
            radius: 0.5,
            half_height: 0.5,
        }
    }
}

impl Cylinder {
    /// Create a new `Cylinder` from a radius and full height
    #[inline(always)]
    pub fn new(radius: f32, height: f32) -> Self {
        Self {
            radius,
            half_height: height / 2.0,
        }
    }

    /// Get the base of the cylinder as a [`Circle`]
    #[inline(always)]
    pub fn base(&self) -> Circle {
        Circle {
            radius: self.radius,
        }
    }

    /// Get the surface area of the side of the cylinder,
    /// also known as the lateral area
    #[inline(always)]
    #[doc(alias = "side_area")]
    pub fn lateral_area(&self) -> f32 {
        4.0 * PI * self.radius * self.half_height
    }

    /// Get the surface area of one base of the cylinder
    #[inline(always)]
    pub fn base_area(&self) -> f32 {
        PI * self.radius.powi(2)
    }

    /// Get the total surface area of the cylinder
    #[inline(always)]
    pub fn area(&self) -> f32 {
        2.0 * PI * self.radius * (self.radius + 2.0 * self.half_height)
    }

    /// Get the volume of the cylinder
    #[inline(always)]
    pub fn volume(&self) -> f32 {
        self.base_area() * 2.0 * self.half_height
    }
}

/// A 3D capsule primitive.
/// A three-dimensional capsule is defined as a surface at a distance (radius) from a line
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Capsule3d {
    /// The radius of the capsule
    pub radius: f32,
    /// Half the height of the capsule, excluding the hemispheres
    pub half_length: f32,
}
impl Primitive3d for Capsule3d {}

impl Default for Capsule3d {
    /// Returns the default [`Capsule3d`] with a radius of `0.5` and a segment length of `1.0`.
    /// The total height is `2.0`.
    fn default() -> Self {
        Self {
            radius: 0.5,
            half_length: 0.5,
        }
    }
}

impl Capsule3d {
    /// Create a new `Capsule3d` from a radius and length
    pub fn new(radius: f32, length: f32) -> Self {
        Self {
            radius,
            half_length: length / 2.0,
        }
    }

    /// Get the part connecting the hemispherical ends
    /// of the capsule as a [`Cylinder`]
    #[inline(always)]
    pub fn to_cylinder(&self) -> Cylinder {
        Cylinder {
            radius: self.radius,
            half_height: self.half_length,
        }
    }

    /// Get the surface area of the capsule
    #[inline(always)]
    pub fn area(&self) -> f32 {
        // Modified version of 2pi * r * (2r + h)
        4.0 * PI * self.radius * (self.radius + self.half_length)
    }

    /// Get the volume of the capsule
    #[inline(always)]
    pub fn volume(&self) -> f32 {
        // Modified version of pi * r^2 * (4/3 * r + a)
        let diameter = self.radius * 2.0;
        PI * self.radius * diameter * (diameter / 3.0 + self.half_length)
    }
}

/// A cone primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Cone {
    /// The radius of the base
    pub radius: f32,
    /// The height of the cone
    pub height: f32,
}
impl Primitive3d for Cone {}

impl Cone {
    /// Get the base of the cone as a [`Circle`]
    #[inline(always)]
    pub fn base(&self) -> Circle {
        Circle {
            radius: self.radius,
        }
    }

    /// Get the slant height of the cone, the length of the line segment
    /// connecting a point on the base to the apex
    #[inline(always)]
    #[doc(alias = "side_length")]
    pub fn slant_height(&self) -> f32 {
        self.radius.hypot(self.height)
    }

    /// Get the surface area of the side of the cone,
    /// also known as the lateral area
    #[inline(always)]
    #[doc(alias = "side_area")]
    pub fn lateral_area(&self) -> f32 {
        PI * self.radius * self.slant_height()
    }

    /// Get the surface area of the base of the cone
    #[inline(always)]
    pub fn base_area(&self) -> f32 {
        PI * self.radius.powi(2)
    }

    /// Get the total surface area of the cone
    #[inline(always)]
    pub fn area(&self) -> f32 {
        self.base_area() + self.lateral_area()
    }

    /// Get the volume of the cone
    #[inline(always)]
    pub fn volume(&self) -> f32 {
        (self.base_area() * self.height) / 3.0
    }
}

/// A conical frustum primitive.
/// A conical frustum can be created
/// by slicing off a section of a cone.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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

impl Default for Torus {
    /// Returns the default [`Torus`] with a minor radius of `0.25` and a major radius of `0.75`.
    fn default() -> Self {
        Self {
            minor_radius: 0.25,
            major_radius: 0.75,
        }
    }
}

impl Torus {
    /// Create a new `Torus` from an inner and outer radius.
    ///
    /// The inner radius is the radius of the hole, and the outer radius
    /// is the radius of the entire object
    #[inline(always)]
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
    #[inline(always)]
    pub fn inner_radius(&self) -> f32 {
        self.major_radius - self.minor_radius
    }

    /// Get the outer radius of the torus.
    /// This corresponds to the overall radius of the entire object,
    /// or `major_radius + minor_radius`
    #[inline(always)]
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
    #[inline(always)]
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
    #[inline(always)]
    pub fn area(&self) -> f32 {
        4.0 * PI.powi(2) * self.major_radius * self.minor_radius
    }

    /// Get the volume of the torus. Note that this only produces
    /// the expected result when the torus has a ring and isn't self-intersecting
    #[inline(always)]
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
    fn direction_creation() {
        assert_eq!(Direction3d::new(Vec3::X * 12.5), Ok(Direction3d::X));
        assert_eq!(
            Direction3d::new(Vec3::new(0.0, 0.0, 0.0)),
            Err(InvalidDirectionError::Zero)
        );
        assert_eq!(
            Direction3d::new(Vec3::new(f32::INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction3d::new(Vec3::new(f32::NEG_INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction3d::new(Vec3::new(f32::NAN, 0.0, 0.0)),
            Err(InvalidDirectionError::NaN)
        );
        assert_eq!(
            Direction3d::new_and_length(Vec3::X * 6.5),
            Ok((Direction3d::X, 6.5))
        );

        // Test rotation
        assert!(
            (Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Direction3d::X)
                .abs_diff_eq(Vec3::Y, 10e-6)
        );
    }

    #[test]
    fn cuboid_closest_point() {
        let cuboid = Cuboid::new(2.0, 2.0, 2.0);
        assert_eq!(cuboid.closest_point(Vec3::X * 10.0), Vec3::X);
        assert_eq!(cuboid.closest_point(Vec3::NEG_ONE * 10.0), Vec3::NEG_ONE);
        assert_eq!(
            cuboid.closest_point(Vec3::new(0.25, 0.1, 0.3)),
            Vec3::new(0.25, 0.1, 0.3)
        );
    }

    #[test]
    fn sphere_closest_point() {
        let sphere = Sphere { radius: 1.0 };
        assert_eq!(sphere.closest_point(Vec3::X * 10.0), Vec3::X);
        assert_eq!(
            sphere.closest_point(Vec3::NEG_ONE * 10.0),
            Vec3::NEG_ONE.normalize()
        );
        assert_eq!(
            sphere.closest_point(Vec3::new(0.25, 0.1, 0.3)),
            Vec3::new(0.25, 0.1, 0.3)
        );
    }

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
        let capsule = Capsule3d::new(2.0, 9.0);
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
