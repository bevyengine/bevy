use super::Primitive3d;
use crate::Vec3;

/// A normalized vector pointing in a direction in 3D space
#[derive(Clone, Copy, Debug)]
pub struct Direction3d(Vec3);

impl From<Vec3> for Direction3d {
    fn from(value: Vec3) -> Self {
        Self(value.normalize())
    }
}

impl Direction3d {
    /// Create a direction from a [Vec3] that is already normalized
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
#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    /// The radius of the sphere
    pub radius: f32,
}
impl Primitive3d for Sphere {}

/// An unbounded plane in 3D space. It forms a separating surface trough the origin,
/// stretching infinitely far
#[derive(Clone, Copy, Debug)]
pub struct Plane3d {
    /// The normal of the plane, the plane will be placed perpendicular to this direction
    pub normal: Direction3d,
}
impl Primitive3d for Plane3d {}

/// An infinite line along a direction in 3D space.
/// For a finite line: [`LineSegment3d`]
#[derive(Clone, Copy, Debug)]
pub struct Line3d {
    /// The direction of the line
    pub direction: Direction3d,
}
impl Primitive3d for Line3d {}

/// A section of a line along a direction in 3D space.
#[derive(Clone, Debug)]
pub struct LineSegment3d {
    /// The direction of the line
    pub direction: Direction3d,
    /// The point on the line where the line starts, this value can be either positive of negative,
    /// depending on where relative to the origin the line should start
    pub start: f32,
    /// The point on the line where the line ends, this value can be either positive of negative,
    /// depending on where relative to the origin the line should start
    pub end: f32,
}
impl Primitive3d for LineSegment3d {}

/// A series of connected line segments in 3D space.
///
/// For a version without generics: [`BoxedPolyline3d`]
#[derive(Clone, Debug)]
pub struct Polyline3d<const N: usize> {
    /// The vertices of the polyline
    pub vertices: [Vec3; N],
}
impl<const N: usize> Primitive3d for Polyline3d<N> {}

/// A series of connected line segments in 3D space.
///
/// For a version without alloc: [`Polyline3d`]
#[derive(Clone, Debug)]
pub struct BoxedPolyline3d {
    /// The vertices of the polyline
    pub vertices: Box<[Vec3]>,
}
impl Primitive3d for BoxedPolyline3d {}

/// A cuboid primitive, more commonly known as a box.
#[derive(Clone, Copy, Debug)]
pub struct Cuboid {
    /// Half of the width, height and depth of the cuboid
    pub half_extents: Vec3,
}
impl Primitive3d for Cuboid {}

/// A cylinder primitive
#[derive(Clone, Copy, Debug)]
pub struct Cylinder {
    /// The radius of the cylinder
    pub radius: f32,
    /// The half height of the cylinder
    pub half_height: f32,
}
impl Primitive3d for Cylinder {}

/// A capsule primitive
#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    /// The radius of the capsule
    pub radius: f32,
    /// Half the height of the capsule, excluding the hemispheres
    pub half_extent: f32,
}
impl Primitive3d for Capsule {}
