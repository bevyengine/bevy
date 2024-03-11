mod primitive_impls;

use glam::Mat3;

use super::{BoundingVolume, IntersectsVolume};
use crate::prelude::{Quat, Vec3};

/// Computes the geometric center of the given set of points.
#[inline(always)]
fn point_cloud_3d_center(points: &[Vec3]) -> Vec3 {
    assert!(
        !points.is_empty(),
        "cannot compute the center of an empty set of points"
    );

    let denom = 1.0 / points.len() as f32;
    points.iter().fold(Vec3::ZERO, |acc, point| acc + *point) * denom
}

/// A trait with methods that return 3D bounded volumes for a shape
pub trait Bounded3d {
    /// Get an axis-aligned bounding box for the shape with the given translation and rotation
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d;
    /// Get a bounding sphere for the shape with the given translation and rotation
    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere;
}

/// A 3D axis-aligned bounding box
#[derive(Clone, Copy, Debug)]
pub struct Aabb3d {
    /// The minimum point of the box
    pub min: Vec3,
    /// The maximum point of the box
    pub max: Vec3,
}

impl Aabb3d {
    /// Constructs an AABB from its center and half-size.
    #[inline(always)]
    pub fn new(center: Vec3, half_size: Vec3) -> Self {
        debug_assert!(half_size.x >= 0.0 && half_size.y >= 0.0 && half_size.z >= 0.0);
        Self {
            min: center - half_size,
            max: center + half_size,
        }
    }

    /// Computes the smallest [`Aabb3d`] containing the given set of points,
    /// transformed by `translation` and `rotation`.
    ///
    /// # Panics
    ///
    /// Panics if the given set of points is empty.
    #[inline(always)]
    pub fn from_point_cloud(translation: Vec3, rotation: Quat, points: &[Vec3]) -> Aabb3d {
        // Transform all points by rotation
        let mut iter = points.iter().map(|point| rotation * *point);

        let first = iter
            .next()
            .expect("point cloud must contain at least one point for Aabb3d construction");

        let (min, max) = iter.fold((first, first), |(prev_min, prev_max), point| {
            (point.min(prev_min), point.max(prev_max))
        });

        Aabb3d {
            min: min + translation,
            max: max + translation,
        }
    }

    /// Computes the smallest [`BoundingSphere`] containing this [`Aabb3d`].
    #[inline(always)]
    pub fn bounding_sphere(&self) -> BoundingSphere {
        let radius = self.min.distance(self.max) / 2.0;
        BoundingSphere::new(self.center(), radius)
    }

    /// Finds the point on the AABB that is closest to the given `point`.
    ///
    /// If the point is outside the AABB, the returned point will be on the surface of the AABB.
    /// Otherwise, it will be inside the AABB and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        // Clamp point coordinates to the AABB
        point.clamp(self.min, self.max)
    }
}

impl BoundingVolume for Aabb3d {
    type Translation = Vec3;
    type Rotation = Quat;
    type HalfSize = Vec3;

    #[inline(always)]
    fn center(&self) -> Self::Translation {
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

    #[inline(always)]
    fn scale_around_center(&self, scale: Self::HalfSize) -> Self {
        let b = Self {
            min: self.center() - (self.half_size() * scale),
            max: self.center() + (self.half_size() * scale),
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    /// Transforms the bounding volume by first rotating it around the origin and then applying a translation.
    ///
    /// The result is an Axis-Aligned Bounding Box that encompasses the rotated shape.
    ///
    /// Note that the result may not be as tightly fitting as the original, and repeated rotations
    /// can cause the AABB to grow indefinitely. Avoid applying multiple rotations to the same AABB,
    /// and consider storing the original AABB and rotating that every time instead.
    #[inline(always)]
    fn transformed_by(
        mut self,
        translation: Self::Translation,
        rotation: impl Into<Self::Rotation>,
    ) -> Self {
        self.transform_by(translation, rotation);
        self
    }

    /// Transforms the bounding volume by first rotating it around the origin and then applying a translation.
    ///
    /// The result is an Axis-Aligned Bounding Box that encompasses the rotated shape.
    ///
    /// Note that the result may not be as tightly fitting as the original, and repeated rotations
    /// can cause the AABB to grow indefinitely. Avoid applying multiple rotations to the same AABB,
    /// and consider storing the original AABB and rotating that every time instead.
    #[inline(always)]
    fn transform_by(
        &mut self,
        translation: Self::Translation,
        rotation: impl Into<Self::Rotation>,
    ) {
        self.rotate_by(rotation);
        self.translate_by(translation);
    }

    #[inline(always)]
    fn translate_by(&mut self, translation: Self::Translation) {
        self.min += translation;
        self.max += translation;
    }

    /// Rotates the bounding volume around the origin by the given rotation.
    ///
    /// The result is an Axis-Aligned Bounding Box that encompasses the rotated shape.
    ///
    /// Note that the result may not be as tightly fitting as the original, and repeated rotations
    /// can cause the AABB to grow indefinitely. Avoid applying multiple rotations to the same AABB,
    /// and consider storing the original AABB and rotating that every time instead.
    #[inline(always)]
    fn rotated_by(mut self, rotation: impl Into<Self::Rotation>) -> Self {
        self.rotate_by(rotation);
        self
    }

    /// Rotates the bounding volume around the origin by the given rotation.
    ///
    /// The result is an Axis-Aligned Bounding Box that encompasses the rotated shape.
    ///
    /// Note that the result may not be as tightly fitting as the original, and repeated rotations
    /// can cause the AABB to grow indefinitely. Avoid applying multiple rotations to the same AABB,
    /// and consider storing the original AABB and rotating that every time instead.
    #[inline(always)]
    fn rotate_by(&mut self, rotation: impl Into<Self::Rotation>) {
        let rot_mat = Mat3::from_quat(rotation.into());
        let abs_rot_mat = Mat3::from_cols(
            rot_mat.x_axis.abs(),
            rot_mat.y_axis.abs(),
            rot_mat.z_axis.abs(),
        );
        let half_size = abs_rot_mat * self.half_size();
        *self = Self::new(rot_mat * self.center(), half_size);
    }
}

impl IntersectsVolume<Self> for Aabb3d {
    #[inline(always)]
    fn intersects(&self, other: &Self) -> bool {
        let x_overlaps = self.min.x <= other.max.x && self.max.x >= other.min.x;
        let y_overlaps = self.min.y <= other.max.y && self.max.y >= other.min.y;
        let z_overlaps = self.min.z <= other.max.z && self.max.z >= other.min.z;
        x_overlaps && y_overlaps && z_overlaps
    }
}

impl IntersectsVolume<BoundingSphere> for Aabb3d {
    #[inline(always)]
    fn intersects(&self, sphere: &BoundingSphere) -> bool {
        let closest_point = self.closest_point(sphere.center);
        let distance_squared = sphere.center.distance_squared(closest_point);
        let radius_squared = sphere.radius().powi(2);
        distance_squared <= radius_squared
    }
}

#[cfg(test)]
mod aabb3d_tests {
    use super::Aabb3d;
    use crate::{
        bounding::{BoundingSphere, BoundingVolume, IntersectsVolume},
        Quat, Vec3,
    };

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

    #[test]
    fn scale_around_center() {
        let a = Aabb3d {
            min: Vec3::NEG_ONE,
            max: Vec3::ONE,
        };
        let scaled = a.scale_around_center(Vec3::splat(2.));
        assert!((scaled.min - Vec3::splat(-2.)).length() < std::f32::EPSILON);
        assert!((scaled.max - Vec3::splat(2.)).length() < std::f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn transform() {
        let a = Aabb3d {
            min: Vec3::new(-2.0, -2.0, -2.0),
            max: Vec3::new(2.0, 2.0, 2.0),
        };
        let transformed = a.transformed_by(
            Vec3::new(2.0, -2.0, 4.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
        );
        let half_length = 2_f32.hypot(2.0);
        assert_eq!(
            transformed.min,
            Vec3::new(2.0 - half_length, -half_length - 2.0, 2.0)
        );
        assert_eq!(
            transformed.max,
            Vec3::new(2.0 + half_length, half_length - 2.0, 6.0)
        );
    }

    #[test]
    fn closest_point() {
        let aabb = Aabb3d {
            min: Vec3::NEG_ONE,
            max: Vec3::ONE,
        };
        assert_eq!(aabb.closest_point(Vec3::X * 10.0), Vec3::X);
        assert_eq!(aabb.closest_point(Vec3::NEG_ONE * 10.0), Vec3::NEG_ONE);
        assert_eq!(
            aabb.closest_point(Vec3::new(0.25, 0.1, 0.3)),
            Vec3::new(0.25, 0.1, 0.3)
        );
    }

    #[test]
    fn intersect_aabb() {
        let aabb = Aabb3d {
            min: Vec3::NEG_ONE,
            max: Vec3::ONE,
        };
        assert!(aabb.intersects(&aabb));
        assert!(aabb.intersects(&Aabb3d {
            min: Vec3::splat(0.5),
            max: Vec3::splat(2.0),
        }));
        assert!(aabb.intersects(&Aabb3d {
            min: Vec3::splat(-2.0),
            max: Vec3::splat(-0.5),
        }));
        assert!(!aabb.intersects(&Aabb3d {
            min: Vec3::new(1.1, 0.0, 0.0),
            max: Vec3::new(2.0, 0.5, 0.25),
        }));
    }

    #[test]
    fn intersect_bounding_sphere() {
        let aabb = Aabb3d {
            min: Vec3::NEG_ONE,
            max: Vec3::ONE,
        };
        assert!(aabb.intersects(&BoundingSphere::new(Vec3::ZERO, 1.0)));
        assert!(aabb.intersects(&BoundingSphere::new(Vec3::ONE * 1.5, 1.0)));
        assert!(aabb.intersects(&BoundingSphere::new(Vec3::NEG_ONE * 1.5, 1.0)));
        assert!(!aabb.intersects(&BoundingSphere::new(Vec3::ONE * 1.75, 1.0)));
    }
}

use crate::primitives::Sphere;

/// A bounding sphere
#[derive(Clone, Copy, Debug)]
pub struct BoundingSphere {
    /// The center of the bounding sphere
    pub center: Vec3,
    /// The sphere
    pub sphere: Sphere,
}

impl BoundingSphere {
    /// Constructs a bounding sphere from its center and radius.
    pub fn new(center: Vec3, radius: f32) -> Self {
        debug_assert!(radius >= 0.);
        Self {
            center,
            sphere: Sphere { radius },
        }
    }

    /// Computes a [`BoundingSphere`] containing the given set of points,
    /// transformed by `translation` and `rotation`.
    ///
    /// The bounding sphere is not guaranteed to be the smallest possible.
    #[inline(always)]
    pub fn from_point_cloud(translation: Vec3, rotation: Quat, points: &[Vec3]) -> BoundingSphere {
        let center = point_cloud_3d_center(points);
        let mut radius_squared = 0.0;

        for point in points {
            // Get squared version to avoid unnecessary sqrt calls
            let distance_squared = point.distance_squared(center);
            if distance_squared > radius_squared {
                radius_squared = distance_squared;
            }
        }

        BoundingSphere::new(rotation * center + translation, radius_squared.sqrt())
    }

    /// Get the radius of the bounding sphere
    #[inline(always)]
    pub fn radius(&self) -> f32 {
        self.sphere.radius
    }

    /// Computes the smallest [`Aabb3d`] containing this [`BoundingSphere`].
    #[inline(always)]
    pub fn aabb_3d(&self) -> Aabb3d {
        Aabb3d {
            min: self.center - Vec3::splat(self.radius()),
            max: self.center + Vec3::splat(self.radius()),
        }
    }

    /// Finds the point on the bounding sphere that is closest to the given `point`.
    ///
    /// If the point is outside the sphere, the returned point will be on the surface of the sphere.
    /// Otherwise, it will be inside the sphere and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        self.sphere.closest_point(point - self.center) + self.center
    }
}

impl BoundingVolume for BoundingSphere {
    type Translation = Vec3;
    type Rotation = Quat;
    type HalfSize = f32;

    #[inline(always)]
    fn center(&self) -> Self::Translation {
        self.center
    }

    #[inline(always)]
    fn half_size(&self) -> Self::HalfSize {
        self.radius()
    }

    #[inline(always)]
    fn visible_area(&self) -> f32 {
        2. * std::f32::consts::PI * self.radius() * self.radius()
    }

    #[inline(always)]
    fn contains(&self, other: &Self) -> bool {
        let diff = self.radius() - other.radius();
        self.center.distance_squared(other.center) <= diff.powi(2).copysign(diff)
    }

    #[inline(always)]
    fn merge(&self, other: &Self) -> Self {
        let diff = other.center - self.center;
        let length = diff.length();
        if self.radius() >= length + other.radius() {
            return *self;
        }
        if other.radius() >= length + self.radius() {
            return *other;
        }
        let dir = diff / length;
        Self::new(
            (self.center + other.center) / 2. + dir * ((other.radius() - self.radius()) / 2.),
            (length + self.radius() + other.radius()) / 2.,
        )
    }

    #[inline(always)]
    fn grow(&self, amount: Self::HalfSize) -> Self {
        debug_assert!(amount >= 0.);
        Self {
            center: self.center,
            sphere: Sphere {
                radius: self.radius() + amount,
            },
        }
    }

    #[inline(always)]
    fn shrink(&self, amount: Self::HalfSize) -> Self {
        debug_assert!(amount >= 0.);
        debug_assert!(self.radius() >= amount);
        Self {
            center: self.center,
            sphere: Sphere {
                radius: self.radius() - amount,
            },
        }
    }

    #[inline(always)]
    fn scale_around_center(&self, scale: Self::HalfSize) -> Self {
        debug_assert!(scale >= 0.);
        Self::new(self.center, self.radius() * scale)
    }

    #[inline(always)]
    fn translate_by(&mut self, translation: Self::Translation) {
        self.center += translation;
    }

    #[inline(always)]
    fn rotate_by(&mut self, rotation: impl Into<Self::Rotation>) {
        let rotation: Quat = rotation.into();
        self.center = rotation * self.center;
    }
}

impl IntersectsVolume<Self> for BoundingSphere {
    #[inline(always)]
    fn intersects(&self, other: &Self) -> bool {
        let center_distance_squared = self.center.distance_squared(other.center);
        let radius_sum_squared = (self.radius() + other.radius()).powi(2);
        center_distance_squared <= radius_sum_squared
    }
}

impl IntersectsVolume<Aabb3d> for BoundingSphere {
    #[inline(always)]
    fn intersects(&self, aabb: &Aabb3d) -> bool {
        aabb.intersects(self)
    }
}

#[cfg(test)]
mod bounding_sphere_tests {
    use approx::assert_relative_eq;

    use super::BoundingSphere;
    use crate::{
        bounding::{BoundingVolume, IntersectsVolume},
        Quat, Vec3,
    };

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
        assert!((merged.radius() - 5.5).abs() < std::f32::EPSILON);
        assert!(merged.contains(&a));
        assert!(merged.contains(&b));
        assert!(!a.contains(&merged));
        assert!(!b.contains(&merged));

        // When one circle contains the other circle, we use the bigger circle
        let b = BoundingSphere::new(Vec3::ZERO, 3.);
        assert!(a.contains(&b));
        let merged = a.merge(&b);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.radius(), a.radius());

        // When two circles are at the same point, we use the bigger radius
        let b = BoundingSphere::new(Vec3::ONE, 6.);
        let merged = a.merge(&b);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.radius(), b.radius());
    }

    #[test]
    fn merge_identical() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let merged = a.merge(&a);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.radius(), a.radius());
    }

    #[test]
    fn grow() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let padded = a.grow(1.25);
        assert!((padded.radius() - 6.25).abs() < std::f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let shrunk = a.shrink(0.5);
        assert!((shrunk.radius() - 4.5).abs() < std::f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let scaled = a.scale_around_center(2.);
        assert!((scaled.radius() - 10.).abs() < std::f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn transform() {
        let a = BoundingSphere::new(Vec3::ONE, 5.0);
        let transformed = a.transformed_by(
            Vec3::new(2.0, -2.0, 4.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
        );
        assert_relative_eq!(
            transformed.center,
            Vec3::new(2.0, std::f32::consts::SQRT_2 - 2.0, 5.0)
        );
        assert_eq!(transformed.radius(), 5.0);
    }

    #[test]
    fn closest_point() {
        let sphere = BoundingSphere::new(Vec3::ZERO, 1.0);
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
    fn intersect_bounding_sphere() {
        let sphere = BoundingSphere::new(Vec3::ZERO, 1.0);
        assert!(sphere.intersects(&BoundingSphere::new(Vec3::ZERO, 1.0)));
        assert!(sphere.intersects(&BoundingSphere::new(Vec3::ONE * 1.1, 1.0)));
        assert!(sphere.intersects(&BoundingSphere::new(Vec3::NEG_ONE * 1.1, 1.0)));
        assert!(!sphere.intersects(&BoundingSphere::new(Vec3::ONE * 1.2, 1.0)));
    }
}
