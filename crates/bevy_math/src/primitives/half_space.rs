#[cfg(feature = "bevy_ecs")]
use bevy_ecs::component::Component;
#[cfg(all(feature = "bevy_reflect", feature = "bevy_ecs"))]
use bevy_ecs::prelude::ReflectComponent;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::{Vec3A, Vec4};
use glam::Vec4Swizzles;

/// A region of 3D space, specifically an open set whose border is a bisecting 2D plane.
/// This bisecting plane partitions 3D space into two infinite regions,
/// the half-space is one of those regions and excludes the bisecting plane.
///
/// Each instance of this type is characterized by:
/// - the bisecting plane's unit normal, normalized and pointing "inside" the half-space,
/// - the signed distance along the normal from the bisecting plane to the origin of 3D space.
///
/// The distance can also be seen as:
/// - the distance along the inverse of the normal from the origin of 3D space to the bisecting plane,
/// - the opposite of the distance along the normal from the origin of 3D space to the bisecting plane.
///
/// Any point `p` is considered to be within the `HalfSpace` when the length of the projection
/// of p on the normal is greater or equal than the opposite of the distance,
/// meaning: if the equation `normal.dot(p) + distance > 0.` is satisfied.
///
/// For example, the half-space containing all the points with a z-coordinate lesser
/// or equal than `8.0` would be defined by: `HalfSpace::new(Vec3::NEG_Z.extend(-8.0))`.
/// It includes all the points from the bisecting plane towards `NEG_Z`, and the distance
/// from the plane to the origin is `-8.0` along `NEG_Z`.
///
/// It is used to define a [`Frustum`], but is also a useful mathematical primitive for rendering tasks such as  light computation.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_ecs", derive(Component))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
#[cfg_attr(
    all(feature = "bevy_ecs", feature = "bevy_reflect"),
    reflect(Component)
)]
pub struct HalfSpace {
    normal_d: Vec4,
}

impl HalfSpace {
    /// Constructs a `HalfSpace` from a 4D vector whose first 3 components
    /// represent the bisecting plane's unit normal, and the last component is
    /// the signed distance along the normal from the plane to the origin.
    /// The constructor ensures the normal vector is normalized and the distance is appropriately scaled.
    #[inline]
    pub fn new(normal_d: Vec4) -> Self {
        Self {
            normal_d: normal_d * normal_d.xyz().length_recip(),
        }
    }

    /// Returns the unit normal vector of the bisecting plane that characterizes the `HalfSpace`.
    #[inline]
    pub fn normal(&self) -> Vec3A {
        Vec3A::from(self.normal_d)
    }

    /// Returns the signed distance from the bisecting plane to the origin along
    /// the plane's unit normal vector.
    #[inline]
    pub fn d(&self) -> f32 {
        self.normal_d.w
    }

    /// Returns the bisecting plane's unit normal vector and the signed distance
    /// from the plane to the origin.
    #[inline]
    pub fn normal_d(&self) -> Vec4 {
        self.normal_d
    }
}
