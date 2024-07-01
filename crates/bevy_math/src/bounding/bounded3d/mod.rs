mod extrusion;
mod primitive_impls;

use glam::Mat3;

use super::{BoundingVolume, IntersectsVolume};
use crate::{Quat, Vec3, Vec3A};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
pub use extrusion::BoundedExtrusion;

/// Computes the geometric center of the given set of points.
#[inline(always)]
fn point_cloud_3d_center(points: impl Iterator<Item = impl Into<Vec3A>>) -> Vec3A {
    let (acc, len) = points.fold((Vec3A::ZERO, 0), |(acc, len), point| {
        (acc + point.into(), len + 1)
    });

    assert!(
        len > 0,
        "cannot compute the center of an empty set of points"
    );
    acc / len as f32
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
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct Aabb3d {
    /// The minimum point of the box
    pub min: Vec3A,
    /// The maximum point of the box
    pub max: Vec3A,
}

impl Aabb3d {
    /// Constructs an AABB from its center and half-size.
    #[inline(always)]
    pub fn new(center: impl Into<Vec3A>, half_size: impl Into<Vec3A>) -> Self {
        let (center, half_size) = (center.into(), half_size.into());
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
    pub fn from_point_cloud(
        translation: impl Into<Vec3A>,
        rotation: Quat,
        points: impl Iterator<Item = impl Into<Vec3A>>,
    ) -> Aabb3d {
        // Transform all points by rotation
        let mut iter = points.map(|point| rotation * point.into());

        let first = iter
            .next()
            .expect("point cloud must contain at least one point for Aabb3d construction");

        let (min, max) = iter.fold((first, first), |(prev_min, prev_max), point| {
            (point.min(prev_min), point.max(prev_max))
        });

        let translation = translation.into();
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
    pub fn closest_point(&self, point: impl Into<Vec3A>) -> Vec3A {
        // Clamp point coordinates to the AABB
        point.into().clamp(self.min, self.max)
    }
}

impl BoundingVolume for Aabb3d {
    type Translation = Vec3A;
    type Rotation = Quat;
    type HalfSize = Vec3A;

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
        other.min.cmpge(self.min).all() && other.max.cmple(self.max).all()
    }

    #[inline(always)]
    fn merge(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline(always)]
    fn grow(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        let b = Self {
            min: self.min - amount,
            max: self.max + amount,
        };
        debug_assert!(b.min.cmple(b.max).all());
        b
    }

    #[inline(always)]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        let b = Self {
            min: self.min + amount,
            max: self.max - amount,
        };
        debug_assert!(b.min.cmple(b.max).all());
        b
    }

    #[inline(always)]
    fn scale_around_center(&self, scale: impl Into<Self::HalfSize>) -> Self {
        let scale = scale.into();
        let b = Self {
            min: self.center() - (self.half_size() * scale),
            max: self.center() + (self.half_size() * scale),
        };
        debug_assert!(b.min.cmple(b.max).all());
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
        translation: impl Into<Self::Translation>,
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
        translation: impl Into<Self::Translation>,
        rotation: impl Into<Self::Rotation>,
    ) {
        self.rotate_by(rotation);
        self.translate_by(translation);
    }

    #[inline(always)]
    fn translate_by(&mut self, translation: impl Into<Self::Translation>) {
        let translation = translation.into();
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
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
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
        Quat, Vec3, Vec3A,
    };

    #[test]
    fn center() {
        let aabb = Aabb3d {
            min: Vec3A::new(-0.5, -1., -0.5),
            max: Vec3A::new(1., 1., 2.),
        };
        assert!((aabb.center() - Vec3A::new(0.25, 0., 0.75)).length() < f32::EPSILON);
        let aabb = Aabb3d {
            min: Vec3A::new(5., 5., -10.),
            max: Vec3A::new(10., 10., -5.),
        };
        assert!((aabb.center() - Vec3A::new(7.5, 7.5, -7.5)).length() < f32::EPSILON);
    }

    #[test]
    fn half_size() {
        let aabb = Aabb3d {
            min: Vec3A::new(-0.5, -1., -0.5),
            max: Vec3A::new(1., 1., 2.),
        };
        assert!((aabb.half_size() - Vec3A::new(0.75, 1., 1.25)).length() < f32::EPSILON);
    }

    #[test]
    fn area() {
        let aabb = Aabb3d {
            min: Vec3A::new(-1., -1., -1.),
            max: Vec3A::new(1., 1., 1.),
        };
        assert!((aabb.visible_area() - 12.).abs() < f32::EPSILON);
        let aabb = Aabb3d {
            min: Vec3A::new(0., 0., 0.),
            max: Vec3A::new(1., 0.5, 0.25),
        };
        assert!((aabb.visible_area() - 0.875).abs() < f32::EPSILON);
    }

    #[test]
    fn contains() {
        let a = Aabb3d {
            min: Vec3A::new(-1., -1., -1.),
            max: Vec3A::new(1., 1., 1.),
        };
        let b = Aabb3d {
            min: Vec3A::new(-2., -1., -1.),
            max: Vec3A::new(1., 1., 1.),
        };
        assert!(!a.contains(&b));
        let b = Aabb3d {
            min: Vec3A::new(-0.25, -0.8, -0.9),
            max: Vec3A::new(1., 1., 0.9),
        };
        assert!(a.contains(&b));
    }

    #[test]
    fn merge() {
        let a = Aabb3d {
            min: Vec3A::new(-1., -1., -1.),
            max: Vec3A::new(1., 0.5, 1.),
        };
        let b = Aabb3d {
            min: Vec3A::new(-2., -0.5, -0.),
            max: Vec3A::new(0.75, 1., 2.),
        };
        let merged = a.merge(&b);
        assert!((merged.min - Vec3A::new(-2., -1., -1.)).length() < f32::EPSILON);
        assert!((merged.max - Vec3A::new(1., 1., 2.)).length() < f32::EPSILON);
        assert!(merged.contains(&a));
        assert!(merged.contains(&b));
        assert!(!a.contains(&merged));
        assert!(!b.contains(&merged));
    }

    #[test]
    fn grow() {
        let a = Aabb3d {
            min: Vec3A::new(-1., -1., -1.),
            max: Vec3A::new(1., 1., 1.),
        };
        let padded = a.grow(Vec3A::ONE);
        assert!((padded.min - Vec3A::new(-2., -2., -2.)).length() < f32::EPSILON);
        assert!((padded.max - Vec3A::new(2., 2., 2.)).length() < f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = Aabb3d {
            min: Vec3A::new(-2., -2., -2.),
            max: Vec3A::new(2., 2., 2.),
        };
        let shrunk = a.shrink(Vec3A::ONE);
        assert!((shrunk.min - Vec3A::new(-1., -1., -1.)).length() < f32::EPSILON);
        assert!((shrunk.max - Vec3A::new(1., 1., 1.)).length() < f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = Aabb3d {
            min: Vec3A::NEG_ONE,
            max: Vec3A::ONE,
        };
        let scaled = a.scale_around_center(Vec3A::splat(2.));
        assert!((scaled.min - Vec3A::splat(-2.)).length() < f32::EPSILON);
        assert!((scaled.max - Vec3A::splat(2.)).length() < f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn transform() {
        let a = Aabb3d {
            min: Vec3A::new(-2.0, -2.0, -2.0),
            max: Vec3A::new(2.0, 2.0, 2.0),
        };
        let transformed = a.transformed_by(
            Vec3A::new(2.0, -2.0, 4.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
        );
        let half_length = 2_f32.hypot(2.0);
        assert_eq!(
            transformed.min,
            Vec3A::new(2.0 - half_length, -half_length - 2.0, 2.0)
        );
        assert_eq!(
            transformed.max,
            Vec3A::new(2.0 + half_length, half_length - 2.0, 6.0)
        );
    }

    #[test]
    fn closest_point() {
        let aabb = Aabb3d {
            min: Vec3A::NEG_ONE,
            max: Vec3A::ONE,
        };
        assert_eq!(aabb.closest_point(Vec3A::X * 10.0), Vec3A::X);
        assert_eq!(aabb.closest_point(Vec3A::NEG_ONE * 10.0), Vec3A::NEG_ONE);
        assert_eq!(
            aabb.closest_point(Vec3A::new(0.25, 0.1, 0.3)),
            Vec3A::new(0.25, 0.1, 0.3)
        );
    }

    #[test]
    fn intersect_aabb() {
        let aabb = Aabb3d {
            min: Vec3A::NEG_ONE,
            max: Vec3A::ONE,
        };
        assert!(aabb.intersects(&aabb));
        assert!(aabb.intersects(&Aabb3d {
            min: Vec3A::splat(0.5),
            max: Vec3A::splat(2.0),
        }));
        assert!(aabb.intersects(&Aabb3d {
            min: Vec3A::splat(-2.0),
            max: Vec3A::splat(-0.5),
        }));
        assert!(!aabb.intersects(&Aabb3d {
            min: Vec3A::new(1.1, 0.0, 0.0),
            max: Vec3A::new(2.0, 0.5, 0.25),
        }));
    }

    #[test]
    fn intersect_bounding_sphere() {
        let aabb = Aabb3d {
            min: Vec3A::NEG_ONE,
            max: Vec3A::ONE,
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
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct BoundingSphere {
    /// The center of the bounding sphere
    pub center: Vec3A,
    /// The sphere
    pub sphere: Sphere,
}

impl BoundingSphere {
    /// Constructs a bounding sphere from its center and radius.
    pub fn new(center: impl Into<Vec3A>, radius: f32) -> Self {
        debug_assert!(radius >= 0.);
        Self {
            center: center.into(),
            sphere: Sphere { radius },
        }
    }

    /// Computes a [`BoundingSphere`] containing the given set of points,
    /// transformed by `translation` and `rotation`.
    ///
    /// The bounding sphere is not guaranteed to be the smallest possible.
    #[inline(always)]
    pub fn from_point_cloud(
        translation: impl Into<Vec3A>,
        rotation: Quat,
        points: &[impl Copy + Into<Vec3A>],
    ) -> BoundingSphere {
        let center = point_cloud_3d_center(points.iter().map(|v| Into::<Vec3A>::into(*v)));
        let mut radius_squared: f32 = 0.0;

        for point in points {
            // Get squared version to avoid unnecessary sqrt calls
            let distance_squared = Into::<Vec3A>::into(*point).distance_squared(center);
            if distance_squared > radius_squared {
                radius_squared = distance_squared;
            }
        }

        BoundingSphere::new(
            rotation * center + translation.into(),
            radius_squared.sqrt(),
        )
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
            min: self.center - self.radius(),
            max: self.center + self.radius(),
        }
    }

    /// Finds the point on the bounding sphere that is closest to the given `point`.
    ///
    /// If the point is outside the sphere, the returned point will be on the surface of the sphere.
    /// Otherwise, it will be inside the sphere and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: impl Into<Vec3A>) -> Vec3A {
        let point = point.into();
        let radius = self.radius();
        let distance_squared = (point - self.center).length_squared();

        if distance_squared <= radius.powi(2) {
            // The point is inside the sphere.
            point
        } else {
            // The point is outside the sphere.
            // Find the closest point on the surface of the sphere.
            let dir_to_point = point / distance_squared.sqrt();
            self.center + radius * dir_to_point
        }
    }
}

impl BoundingVolume for BoundingSphere {
    type Translation = Vec3A;
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
    fn grow(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        debug_assert!(amount >= 0.);
        Self {
            center: self.center,
            sphere: Sphere {
                radius: self.radius() + amount,
            },
        }
    }

    #[inline(always)]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
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
    fn scale_around_center(&self, scale: impl Into<Self::HalfSize>) -> Self {
        let scale = scale.into();
        debug_assert!(scale >= 0.);
        Self::new(self.center, self.radius() * scale)
    }

    #[inline(always)]
    fn translate_by(&mut self, translation: impl Into<Self::Translation>) {
        self.center += translation.into();
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
        Quat, Vec3, Vec3A,
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
        assert!((merged.center - Vec3A::new(1., 1., 0.5)).length() < f32::EPSILON);
        assert!((merged.radius() - 5.5).abs() < f32::EPSILON);
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
        assert!((padded.radius() - 6.25).abs() < f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let shrunk = a.shrink(0.5);
        assert!((shrunk.radius() - 4.5).abs() < f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = BoundingSphere::new(Vec3::ONE, 5.);
        let scaled = a.scale_around_center(2.);
        assert!((scaled.radius() - 10.).abs() < f32::EPSILON);
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
            Vec3A::new(2.0, std::f32::consts::SQRT_2 - 2.0, 5.0)
        );
        assert_eq!(transformed.radius(), 5.0);
    }

    #[test]
    fn closest_point() {
        let sphere = BoundingSphere::new(Vec3::ZERO, 1.0);
        assert_eq!(sphere.closest_point(Vec3::X * 10.0), Vec3A::X);
        assert_eq!(
            sphere.closest_point(Vec3::NEG_ONE * 10.0),
            Vec3A::NEG_ONE.normalize()
        );
        assert_eq!(
            sphere.closest_point(Vec3::new(0.25, 0.1, 0.3)),
            Vec3A::new(0.25, 0.1, 0.3)
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
