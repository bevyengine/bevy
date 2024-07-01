#![allow(missing_docs)]

use std::borrow::Borrow;

#[cfg(feature = "bevy_ecs")]
use bevy_ecs::component::Component;
#[cfg(all(feature = "bevy_reflect", feature = "bevy_ecs"))]
use bevy_ecs::prelude::ReflectComponent;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use glam::{Affine3A, Mat3A, Vec3, Vec3A};

/// An axis-aligned bounding box, defined by:
/// - a center,
/// - the distances from the center to each faces along the axis,
///     the faces are orthogonal to the axis.
///
/// It is typically used as a component on an entity to represent the local space
/// occupied by this entity, with faces orthogonal to its local axis.
///
/// This component is notably used during "frustum culling", a process to determine
/// if an entity should be rendered by a [`Camera`] if its bounding box intersects
/// with the camera's [`Frustum`].
///
/// It will be added automatically by the systems in [`CalculateBounds`] to entities that:
/// - could be subject to frustum culling, for example with a [`Handle<Mesh>`]
///     or `Sprite` component,
/// - don't have the [`NoFrustumCulling`] component.
///
/// It won't be updated automatically if the space occupied by the entity changes,
/// for example if the vertex positions of a [`Mesh`] inside a `Handle<Mesh>` are
/// updated.
///
/// [`Camera`]: crate::camera::Camera
/// [`NoFrustumCulling`]: crate::view::visibility::NoFrustumCulling
/// [`CalculateBounds`]: crate::view::visibility::VisibilitySystems::CalculateBounds
/// [`Mesh`]: crate::mesh::Mesh
/// [`Handle<Mesh>`]: crate::mesh::Mesh
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy_ecs", derive(Component))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
#[cfg_attr(
    all(feature = "bevy_ecs", feature = "bevy_reflect"),
    reflect(Component)
)]

pub struct Aabb {
    pub center: Vec3A,
    pub half_extents: Vec3A,
}

impl Aabb {
    #[inline]
    pub fn from_min_max(minimum: Vec3, maximum: Vec3) -> Self {
        let minimum = Vec3A::from(minimum);
        let maximum = Vec3A::from(maximum);
        let center = 0.5 * (maximum + minimum);
        let half_extents = 0.5 * (maximum - minimum);
        Self {
            center,
            half_extents,
        }
    }

    /// Returns a bounding box enclosing the specified set of points.
    ///
    /// Returns `None` if the iterator is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Vec3, Vec3A, primitives::legacy_bevy_render::Aabb};
    /// let bb = Aabb::enclosing([Vec3::X, Vec3::Z * 2.0, Vec3::Y * -0.5]).unwrap();
    /// assert_eq!(bb.min(), Vec3A::new(0.0, -0.5, 0.0));
    /// assert_eq!(bb.max(), Vec3A::new(1.0, 0.0, 2.0));
    /// ```
    pub fn enclosing<T: Borrow<Vec3>>(iter: impl IntoIterator<Item = T>) -> Option<Self> {
        let mut iter = iter.into_iter().map(|p| *p.borrow());
        let mut min = iter.next()?;
        let mut max = min;
        for v in iter {
            min = Vec3::min(min, v);
            max = Vec3::max(max, v);
        }
        Some(Self::from_min_max(min, max))
    }

    /// Calculate the relative radius of the AABB with respect to a plane
    #[inline]
    pub fn relative_radius(&self, p_normal: &Vec3A, world_from_local: &Mat3A) -> f32 {
        // NOTE: dot products on Vec3A use SIMD and even with the overhead of conversion are net faster than Vec3
        let half_extents = self.half_extents;
        Vec3A::new(
            p_normal.dot(world_from_local.x_axis),
            p_normal.dot(world_from_local.y_axis),
            p_normal.dot(world_from_local.z_axis),
        )
        .abs()
        .dot(half_extents)
    }

    #[inline]
    pub fn min(&self) -> Vec3A {
        self.center - self.half_extents
    }

    #[inline]
    pub fn max(&self) -> Vec3A {
        self.center + self.half_extents
    }
}

impl From<Sphere> for Aabb {
    #[inline]
    fn from(sphere: Sphere) -> Self {
        Self {
            center: sphere.center,
            half_extents: Vec3A::splat(sphere.radius),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Sphere {
    pub center: Vec3A,
    pub radius: f32,
}

impl Sphere {
    #[inline]
    pub fn intersects_obb(&self, aabb: &Aabb, world_from_local: &Affine3A) -> bool {
        let aabb_center_world = world_from_local.transform_point3a(aabb.center);
        let v = aabb_center_world - self.center;
        let d = v.length();
        let relative_radius = aabb.relative_radius(&(v / d), &world_from_local.matrix3);
        d < self.radius + relative_radius
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec4;

    use crate::primitives::{
        legacy_bevy_render::{Aabb, Sphere},
        Frustum, HalfSpace,
    };

    use super::*;

    // A big, offset frustum
    fn big_frustum() -> Frustum {
        Frustum {
            half_spaces: [
                HalfSpace::new(Vec4::new(-0.9701, -0.2425, -0.0000, 7.7611)),
                HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 4.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, -0.9701, 2.9104)),
                HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 4.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, 0.9701, 2.9104)),
                HalfSpace::new(Vec4::new(0.9701, -0.2425, -0.0000, -1.9403)),
            ],
        }
    }

    #[test]
    fn intersects_sphere_big_frustum_outside() {
        // Sphere outside frustum
        let frustum = big_frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.9167, 0.0000, 0.0000),
            radius: 0.7500,
        };
        assert!(!frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_big_frustum_intersect() {
        // Sphere intersects frustum boundary
        let frustum = big_frustum();
        let sphere = Sphere {
            center: Vec3A::new(7.9288, 0.0000, 2.9728),
            radius: 2.0000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    // A frustum
    fn frustum() -> Frustum {
        Frustum {
            half_spaces: [
                HalfSpace::new(Vec4::new(-0.9701, -0.2425, -0.0000, 0.7276)),
                HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 1.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, -0.9701, 0.7276)),
                HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 1.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, 0.9701, 0.7276)),
                HalfSpace::new(Vec4::new(0.9701, -0.2425, -0.0000, 0.7276)),
            ],
        }
    }

    #[test]
    fn intersects_sphere_frustum_surrounding() {
        // Sphere surrounds frustum
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.0000, 0.0000, 0.0000),
            radius: 3.0000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_contained() {
        // Sphere is contained in frustum
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.0000, 0.0000, 0.0000),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_intersects_plane() {
        // Sphere intersects a plane
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.0000, 0.0000, 0.9695),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_intersects_2_planes() {
        // Sphere intersects 2 planes
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(1.2037, 0.0000, 0.9695),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_intersects_3_planes() {
        // Sphere intersects 3 planes
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(1.2037, -1.0988, 0.9695),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_dodges_1_plane() {
        // Sphere avoids intersecting the frustum by 1 plane
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(-1.7020, 0.0000, 0.0000),
            radius: 0.7000,
        };
        assert!(!frustum.intersects_sphere(&sphere, true));
    }

    // A long frustum.
    fn long_frustum() -> Frustum {
        Frustum {
            half_spaces: [
                HalfSpace::new(Vec4::new(-0.9998, -0.0222, -0.0000, -1.9543)),
                HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 45.1249)),
                HalfSpace::new(Vec4::new(-0.0000, -0.0168, -0.9999, 2.2718)),
                HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 45.1249)),
                HalfSpace::new(Vec4::new(-0.0000, -0.0168, 0.9999, 2.2718)),
                HalfSpace::new(Vec4::new(0.9998, -0.0222, -0.0000, 7.9528)),
            ],
        }
    }

    #[test]
    fn intersects_sphere_long_frustum_outside() {
        // Sphere outside frustum
        let frustum = long_frustum();
        let sphere = Sphere {
            center: Vec3A::new(-4.4889, 46.9021, 0.0000),
            radius: 0.7500,
        };
        assert!(!frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_long_frustum_intersect() {
        // Sphere intersects frustum boundary
        let frustum = long_frustum();
        let sphere = Sphere {
            center: Vec3A::new(-4.9957, 0.0000, -0.7396),
            radius: 4.4094,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn aabb_enclosing() {
        assert_eq!(Aabb::enclosing(<[Vec3; 0]>::default()), None);
        assert_eq!(
            Aabb::enclosing(vec![Vec3::ONE]).unwrap(),
            Aabb::from_min_max(Vec3::ONE, Vec3::ONE)
        );
        assert_eq!(
            Aabb::enclosing(&[Vec3::Y, Vec3::X, Vec3::Z][..]).unwrap(),
            Aabb::from_min_max(Vec3::ZERO, Vec3::ONE)
        );
        assert_eq!(
            Aabb::enclosing([
                Vec3::NEG_X,
                Vec3::X * 2.0,
                Vec3::NEG_Y * 5.0,
                Vec3::Z,
                Vec3::ZERO
            ])
            .unwrap(),
            Aabb::from_min_max(Vec3::new(-1.0, -5.0, 0.0), Vec3::new(2.0, 0.0, 1.0))
        );
    }
}
