use super::BoundingVolume;
use crate::prelude::{Quat, Vec3};

/// A trait with methods that return 3D bounded volumes for a shape
pub trait Bounded3d {
    /// Get an axis-aligned bounding box for the shape with the given translation and rotation
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d;
    /// Get a bounding sphere for the shape with the given translation and rotation
    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere;
}

/// A 3D axis-aligned bounding box
#[derive(Clone, Debug)]
pub struct Aabb3d {
    /// The minimum point of the box
    pub min: Vec3,
    /// The maximum point of the box
    pub max: Vec3,
}

impl BoundingVolume for Aabb3d {
    type Position = Vec3;
    type HalfSize = Vec3;

    #[inline(always)]
    fn center(&self) -> Self::Position {
        (self.min + self.max) / 2.
    }

    #[inline(always)]
    fn half_size(&self) -> Self::HalfSize {
        (self.max - self.min) / 2.
    }

    #[inline(always)]
    fn visible_area(&self) -> f32 {
        let b = self.max - self.min;
        b.x * (b.y + b.z) + b.y * b.z
    }

    #[inline(always)]
    fn contains(&self, other: &Self) -> bool {
        other.min.x >= self.min.x
            && other.min.y >= self.min.y
            && other.min.z >= self.min.z
            && other.max.x <= self.max.x
            && other.max.y <= self.max.y
            && other.max.z <= self.max.z
    }

    #[inline(always)]
    fn merge(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline(always)]
    fn grow(&self, amount: Self::HalfSize) -> Self {
        let b = Self {
            min: self.min - amount,
            max: self.max + amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y && b.min.z <= b.max.z);
        b
    }

    #[inline(always)]
    fn shrink(&self, amount: Self::HalfSize) -> Self {
        let b = Self {
            min: self.min + amount,
            max: self.max - amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y && b.min.z <= b.max.z);
        b
    }
}

#[cfg(test)]
mod aabb3d_tests {
    use super::Aabb3d;
    use crate::{bounding::BoundingVolume, Vec3};

    #[test]
    fn center() {
        let aabb = Aabb3d {
            min: Vec3::new(-0.5, -1., -0.5),
            max: Vec3::new(1., 1., 2.),
        };
        assert!((aabb.center() - Vec3::new(0.25, 0., 0.75)).length() < std::f32::EPSILON);
        let aabb = Aabb3d {
            min: Vec3::new(5., 5., -10.),
            max: Vec3::new(10., 10., -5.),
        };
        assert!((aabb.center() - Vec3::new(7.5, 7.5, -7.5)).length() < std::f32::EPSILON);
    }

    #[test]
    fn half_size() {
        let aabb = Aabb3d {
            min: Vec3::new(-0.5, -1., -0.5),
            max: Vec3::new(1., 1., 2.),
        };
        assert!((aabb.half_size() - Vec3::new(0.75, 1., 1.25)).length() < std::f32::EPSILON);
    }

    #[test]
    fn area() {
        let aabb = Aabb3d {
            min: Vec3::new(-1., -1., -1.),
            max: Vec3::new(1., 1., 1.),
        };
        assert!((aabb.visible_area() - 12.).abs() < std::f32::EPSILON);
        let aabb = Aabb3d {
            min: Vec3::new(0., 0., 0.),
            max: Vec3::new(1., 0.5, 0.25),
        };
        assert!((aabb.visible_area() - 0.875).abs() < std::f32::EPSILON);
    }

    #[test]
    fn contains() {
        let a = Aabb3d {
            min: Vec3::new(-1., -1., -1.),
            max: Vec3::new(1., 1., 1.),
        };
        let b = Aabb3d {
            min: Vec3::new(-2., -1., -1.),
            max: Vec3::new(1., 1., 1.),
        };
        assert!(!a.contains(&b));
        let b = Aabb3d {
            min: Vec3::new(-0.25, -0.8, -0.9),
            max: Vec3::new(1., 1., 0.9),
        };
        assert!(a.contains(&b));
    }

    #[test]
    fn merge() {
        let a = Aabb3d {
            min: Vec3::new(-1., -1., -1.),
            max: Vec3::new(1., 0.5, 1.),
        };
        let b = Aabb3d {
            min: Vec3::new(-2., -0.5, -0.),
            max: Vec3::new(0.75, 1., 2.),
        };
        let merged = a.merge(&b);
        assert!((merged.min - Vec3::new(-2., -1., -1.)).length() < std::f32::EPSILON);
        assert!((merged.max - Vec3::new(1., 1., 2.)).length() < std::f32::EPSILON);
        assert!(merged.contains(&a));
        assert!(merged.contains(&b));
        assert!(!a.contains(&merged));
        assert!(!b.contains(&merged));
    }

    #[test]
    fn grow() {
        let a = Aabb3d {
            min: Vec3::new(-1., -1., -1.),
            max: Vec3::new(1., 1., 1.),
        };
        let padded = a.grow(Vec3::ONE);
        assert!((padded.min - Vec3::new(-2., -2., -2.)).length() < std::f32::EPSILON);
        assert!((padded.max - Vec3::new(2., 2., 2.)).length() < std::f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = Aabb3d {
            min: Vec3::new(-2., -2., -2.),
            max: Vec3::new(2., 2., 2.),
        };
        let shrunk = a.shrink(Vec3::ONE);
        assert!((shrunk.min - Vec3::new(-1., -1., -1.)).length() < std::f32::EPSILON);
        assert!((shrunk.max - Vec3::new(1., 1., 1.)).length() < std::f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }
}

use crate::primitives::Sphere;

/// A bounding sphere
#[derive(Clone, Debug)]
pub struct BoundingSphere {
    /// The center of the bounding sphere
    pub center: Vec3,
    /// The sphere
    pub sphere: Sphere,
}

impl BoundingSphere {
    /// Construct a bounding sphere from its center and radius.
    pub fn new(center: Vec3, radius: f32) -> Self {
        debug_assert!(radius >= 0.);
        Self {
            center,
            sphere: Sphere { radius },
        }
    }
}

impl BoundingVolume for BoundingSphere {
    type Position = Vec3;
    type HalfSize = f32;

    #[inline(always)]
    fn center(&self) -> Self::Position {
        self.center
    }

    #[inline(always)]
    fn half_size(&self) -> Self::HalfSize {
        self.sphere.radius
    }

    #[inline(always)]
    fn visible_area(&self) -> f32 {
        2. * std::f32::consts::PI * self.half_size() * self.half_size()
    }

    #[inline(always)]
    fn contains(&self, other: &Self) -> bool {
        let diff = self.half_size() - other.half_size();
        self.center.distance_squared(other.center) <= diff.powi(2).copysign(diff)
    }

    #[inline(always)]
    fn merge(&self, other: &Self) -> Self {
        let diff = other.center - self.center;
        let length = diff.length();
        if self.half_size() >= length + other.half_size() {
            return self.clone();
        }
        if other.half_size() >= length + self.half_size() {
            return other.clone();
        }
        let dir = diff / length;
        Self::new(
            (self.center + other.center) / 2. + dir * ((other.half_size() - self.half_size()) / 2.),
            (length + self.half_size() + other.half_size()) / 2.,
        )
    }

    #[inline(always)]
    fn grow(&self, amount: Self::HalfSize) -> Self {
        debug_assert!(amount >= 0.);
        Self {
            center: self.center,
            sphere: Sphere {
                radius: self.half_size() + amount,
            },
        }
    }

    #[inline(always)]
    fn shrink(&self, amount: Self::HalfSize) -> Self {
        debug_assert!(amount >= 0.);
        debug_assert!(self.half_size() >= amount);
        Self {
            center: self.center,
            sphere: Sphere {
                radius: self.half_size() - amount,
            },
        }
    }
}

#[cfg(test)]
mod bounding_sphere_tests {
    use super::BoundingSphere;
    use crate::{bounding::BoundingVolume, Vec3};

    #[test]
    fn area() {
        let sphere = BoundingSphere::new(Vec3::ONE, 5.);
        // Since this number is messy we check it with a higher threshold
        assert!((sphere.visible_area() - 157.0796).abs() < 0.001);
    }

    #[test]
    fn contains() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let b = BoundingSphere::new(Vec3::new(5.5, 1., 1.), 1.);
        assert!(!a.contains(&b));
        let b = BoundingSphere::new(Vec3::new(1., -3.5, 1.), 0.5);
        assert!(a.contains(&b));
    }

    #[test]
    fn contains_identical() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        assert!(a.contains(&a));
    }

    #[test]
    fn merge() {
        // When merging two circles that don't contain each other, we find a center position that
        // contains both
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let b = BoundingSphere::new(Vec3::new(1., 1., -4.), 1.);
        let merged = a.merge(&b);
        assert!((merged.center - Vec3::new(1., 1., 0.5)).length() < std::f32::EPSILON);
        assert!((merged.half_size() - 5.5).abs() < std::f32::EPSILON);
        assert!(merged.contains(&a));
        assert!(merged.contains(&b));
        assert!(!a.contains(&merged));
        assert!(!b.contains(&merged));

        // When one circle contains the other circle, we use the bigger circle
        let b = BoundingSphere::new(Vec3::ZERO, 3.);
        assert!(a.contains(&b));
        let merged = a.merge(&b);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.half_size(), a.half_size());

        // When two circles are at the same point, we use the bigger radius
        let b = BoundingSphere::new(Vec3::ONE, 6.);
        let merged = a.merge(&b);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.half_size(), b.half_size());
    }

    #[test]
    fn merge_identical() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let merged = a.merge(&a);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.half_size(), a.half_size());
    }

    #[test]
    fn grow() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let padded = a.grow(1.25);
        assert!((padded.half_size() - 6.25).abs() < std::f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let shrunk = a.shrink(0.5);
        assert!((shrunk.half_size() - 4.5).abs() < std::f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }
}
