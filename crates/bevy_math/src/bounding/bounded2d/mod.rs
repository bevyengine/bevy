mod primitive_impls;

use super::{BoundingVolume, IntersectsVolume};
use crate::{
    ops,
    prelude::{Mat2, Rot2, Vec2},
    FloatPow, Isometry2d,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use itertools::Itertools;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// Computes the geometric center of the given set of points.
#[inline]
fn point_cloud_2d_center(points: &[Vec2]) -> Vec2 {
    assert!(
        !points.is_empty(),
        "cannot compute the center of an empty set of points"
    );

    let denom = 1.0 / points.len() as f32;
    points.iter().fold(Vec2::ZERO, |acc, point| acc + *point) * denom
}

/// A trait with methods that return 2D bounding volumes for a shape.
pub trait Bounded2d {
    /// Get an axis-aligned bounding box for the shape translated and rotated by the given isometry.
    fn aabb_2d(&self, isometry: impl Into<Isometry2d>) -> Aabb2d;
    /// Get a bounding circle for the shape translated and rotated by the given isometry.
    fn bounding_circle(&self, isometry: impl Into<Isometry2d>) -> BoundingCircle;
}

/// A 2D axis-aligned bounding box, or bounding rectangle
#[doc(alias = "BoundingRectangle")]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(Serialize), derive(Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Aabb2d {
    /// The minimum, conventionally bottom-left, point of the box
    pub min: Vec2,
    /// The maximum, conventionally top-right, point of the box
    pub max: Vec2,
}

impl Aabb2d {
    /// Constructs an AABB from its center and half-size.
    #[inline]
    pub fn new(center: Vec2, half_size: Vec2) -> Self {
        debug_assert!(half_size.x >= 0.0 && half_size.y >= 0.0);
        Self {
            min: center - half_size,
            max: center + half_size,
        }
    }

    /// Computes the smallest [`Aabb2d`] containing the given set of points,
    /// transformed by the rotation and translation of the given isometry.
    ///
    /// # Panics
    ///
    /// Panics if the given set of points is empty.
    #[inline]
    pub fn from_point_cloud(isometry: impl Into<Isometry2d>, points: &[Vec2]) -> Aabb2d {
        let isometry = isometry.into();

        // Transform all points by rotation
        let mut iter = points.iter().map(|point| isometry.rotation * *point);

        let first = iter
            .next()
            .expect("point cloud must contain at least one point for Aabb2d construction");

        let (min, max) = iter.fold((first, first), |(prev_min, prev_max), point| {
            (point.min(prev_min), point.max(prev_max))
        });

        Aabb2d {
            min: min + isometry.translation,
            max: max + isometry.translation,
        }
    }

    /// Computes the smallest [`BoundingCircle`] containing this [`Aabb2d`].
    #[inline]
    pub fn bounding_circle(&self) -> BoundingCircle {
        let radius = self.min.distance(self.max) / 2.0;
        BoundingCircle::new(self.center(), radius)
    }

    /// Finds the point on the AABB that is closest to the given `point`.
    ///
    /// If the point is outside the AABB, the returned point will be on the perimeter of the AABB.
    /// Otherwise, it will be inside the AABB and returned as is.
    #[inline]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        // Clamp point coordinates to the AABB
        point.clamp(self.min, self.max)
    }
}

impl BoundingVolume for Aabb2d {
    type Translation = Vec2;
    type Rotation = Rot2;
    type HalfSize = Vec2;

    #[inline]
    fn center(&self) -> Self::Translation {
        (self.min + self.max) / 2.
    }

    #[inline]
    fn half_size(&self) -> Self::HalfSize {
        (self.max - self.min) / 2.
    }

    #[inline]
    fn visible_area(&self) -> f32 {
        let b = (self.max - self.min).max(Vec2::ZERO);
        b.x * b.y
    }

    #[inline]
    fn contains(&self, other: &Self) -> bool {
        other.min.x >= self.min.x
            && other.min.y >= self.min.y
            && other.max.x <= self.max.x
            && other.max.y <= self.max.y
    }

    #[inline]
    fn merge(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline]
    fn grow(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        let b = Self {
            min: self.min - amount,
            max: self.max + amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    #[inline]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        let b = Self {
            min: self.min + amount,
            max: self.max - amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    #[inline]
    fn scale_around_center(&self, scale: impl Into<Self::HalfSize>) -> Self {
        let scale = scale.into();
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
    #[inline]
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
    #[inline]
    fn transform_by(
        &mut self,
        translation: impl Into<Self::Translation>,
        rotation: impl Into<Self::Rotation>,
    ) {
        self.rotate_by(rotation);
        self.translate_by(translation);
    }

    #[inline]
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
    #[inline]
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
    #[inline]
    fn rotate_by(&mut self, rotation: impl Into<Self::Rotation>) {
        let rot_mat = Mat2::from(rotation.into());
        let half_size = rot_mat.abs() * self.half_size();
        *self = Self::new(rot_mat * self.center(), half_size);
    }
}

impl IntersectsVolume<Self> for Aabb2d {
    #[inline]
    fn intersects(&self, other: &Self) -> bool {
        let x_overlaps = self.min.x <= other.max.x && self.max.x >= other.min.x;
        let y_overlaps = self.min.y <= other.max.y && self.max.y >= other.min.y;
        x_overlaps && y_overlaps
    }
}

impl IntersectsVolume<BoundingCircle> for Aabb2d {
    #[inline]
    fn intersects(&self, circle: &BoundingCircle) -> bool {
        let closest_point = self.closest_point(circle.center);
        let distance_squared = circle.center.distance_squared(closest_point);
        let radius_squared = circle.radius().squared();
        distance_squared <= radius_squared
    }
}

#[cfg(test)]
mod aabb2d_tests {
    use approx::assert_relative_eq;

    use super::Aabb2d;
    use crate::{
        bounding::{BoundingCircle, BoundingVolume, IntersectsVolume},
        ops, Vec2,
    };

    #[test]
    fn center() {
        let aabb = Aabb2d {
            min: Vec2::new(-0.5, -1.),
            max: Vec2::new(1., 1.),
        };
        assert!((aabb.center() - Vec2::new(0.25, 0.)).length() < f32::EPSILON);
        let aabb = Aabb2d {
            min: Vec2::new(5., -10.),
            max: Vec2::new(10., -5.),
        };
        assert!((aabb.center() - Vec2::new(7.5, -7.5)).length() < f32::EPSILON);
    }

    #[test]
    fn half_size() {
        let aabb = Aabb2d {
            min: Vec2::new(-0.5, -1.),
            max: Vec2::new(1., 1.),
        };
        let half_size = aabb.half_size();
        assert!((half_size - Vec2::new(0.75, 1.)).length() < f32::EPSILON);
    }

    #[test]
    fn area() {
        let aabb = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 1.),
        };
        assert!(ops::abs(aabb.visible_area() - 4.) < f32::EPSILON);
        let aabb = Aabb2d {
            min: Vec2::new(0., 0.),
            max: Vec2::new(1., 0.5),
        };
        assert!(ops::abs(aabb.visible_area() - 0.5) < f32::EPSILON);
    }

    #[test]
    fn contains() {
        let a = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 1.),
        };
        let b = Aabb2d {
            min: Vec2::new(-2., -1.),
            max: Vec2::new(1., 1.),
        };
        assert!(!a.contains(&b));
        let b = Aabb2d {
            min: Vec2::new(-0.25, -0.8),
            max: Vec2::new(1., 1.),
        };
        assert!(a.contains(&b));
    }

    #[test]
    fn merge() {
        let a = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 0.5),
        };
        let b = Aabb2d {
            min: Vec2::new(-2., -0.5),
            max: Vec2::new(0.75, 1.),
        };
        let merged = a.merge(&b);
        assert!((merged.min - Vec2::new(-2., -1.)).length() < f32::EPSILON);
        assert!((merged.max - Vec2::new(1., 1.)).length() < f32::EPSILON);
        assert!(merged.contains(&a));
        assert!(merged.contains(&b));
        assert!(!a.contains(&merged));
        assert!(!b.contains(&merged));
    }

    #[test]
    fn grow() {
        let a = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 1.),
        };
        let padded = a.grow(Vec2::ONE);
        assert!((padded.min - Vec2::new(-2., -2.)).length() < f32::EPSILON);
        assert!((padded.max - Vec2::new(2., 2.)).length() < f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = Aabb2d {
            min: Vec2::new(-2., -2.),
            max: Vec2::new(2., 2.),
        };
        let shrunk = a.shrink(Vec2::ONE);
        assert!((shrunk.min - Vec2::new(-1., -1.)).length() < f32::EPSILON);
        assert!((shrunk.max - Vec2::new(1., 1.)).length() < f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = Aabb2d {
            min: Vec2::NEG_ONE,
            max: Vec2::ONE,
        };
        let scaled = a.scale_around_center(Vec2::splat(2.));
        assert!((scaled.min - Vec2::splat(-2.)).length() < f32::EPSILON);
        assert!((scaled.max - Vec2::splat(2.)).length() < f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn rotate() {
        let a = Aabb2d {
            min: Vec2::new(-2.0, -2.0),
            max: Vec2::new(2.0, 2.0),
        };
        let rotated = a.rotated_by(core::f32::consts::PI);
        assert_relative_eq!(rotated.min, a.min);
        assert_relative_eq!(rotated.max, a.max);
    }

    #[test]
    fn transform() {
        let a = Aabb2d {
            min: Vec2::new(-2.0, -2.0),
            max: Vec2::new(2.0, 2.0),
        };
        let transformed = a.transformed_by(Vec2::new(2.0, -2.0), core::f32::consts::FRAC_PI_4);
        let half_length = ops::hypot(2.0, 2.0);
        assert_eq!(
            transformed.min,
            Vec2::new(2.0 - half_length, -half_length - 2.0)
        );
        assert_eq!(
            transformed.max,
            Vec2::new(2.0 + half_length, half_length - 2.0)
        );
    }

    #[test]
    fn closest_point() {
        let aabb = Aabb2d {
            min: Vec2::NEG_ONE,
            max: Vec2::ONE,
        };
        assert_eq!(aabb.closest_point(Vec2::X * 10.0), Vec2::X);
        assert_eq!(aabb.closest_point(Vec2::NEG_ONE * 10.0), Vec2::NEG_ONE);
        assert_eq!(
            aabb.closest_point(Vec2::new(0.25, 0.1)),
            Vec2::new(0.25, 0.1)
        );
    }

    #[test]
    fn intersect_aabb() {
        let aabb = Aabb2d {
            min: Vec2::NEG_ONE,
            max: Vec2::ONE,
        };
        assert!(aabb.intersects(&aabb));
        assert!(aabb.intersects(&Aabb2d {
            min: Vec2::new(0.5, 0.5),
            max: Vec2::new(2.0, 2.0),
        }));
        assert!(aabb.intersects(&Aabb2d {
            min: Vec2::new(-2.0, -2.0),
            max: Vec2::new(-0.5, -0.5),
        }));
        assert!(!aabb.intersects(&Aabb2d {
            min: Vec2::new(1.1, 0.0),
            max: Vec2::new(2.0, 0.5),
        }));
    }

    #[test]
    fn intersect_bounding_circle() {
        let aabb = Aabb2d {
            min: Vec2::NEG_ONE,
            max: Vec2::ONE,
        };
        assert!(aabb.intersects(&BoundingCircle::new(Vec2::ZERO, 1.0)));
        assert!(aabb.intersects(&BoundingCircle::new(Vec2::ONE * 1.5, 1.0)));
        assert!(aabb.intersects(&BoundingCircle::new(Vec2::NEG_ONE * 1.5, 1.0)));
        assert!(!aabb.intersects(&BoundingCircle::new(Vec2::ONE * 1.75, 1.0)));
    }
}

use crate::primitives::Circle;

/// A bounding circle
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(Serialize), derive(Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct BoundingCircle {
    /// The center of the bounding circle
    pub center: Vec2,
    /// The circle
    pub circle: Circle,
}

impl BoundingCircle {
    /// Constructs a bounding circle from its center and radius.
    #[inline]
    pub fn new(center: Vec2, radius: f32) -> Self {
        debug_assert!(radius >= 0.);
        Self {
            center,
            circle: Circle { radius },
        }
    }

    /// Computes a [`BoundingCircle`] containing the given set of points,
    /// transformed by the rotation and translation of the given isometry.
    ///
    /// The bounding circle is not guaranteed to be the smallest possible.
    #[inline]
    pub fn from_point_cloud(isometry: impl Into<Isometry2d>, points: &[Vec2]) -> BoundingCircle {
        let isometry = isometry.into();

        let center = point_cloud_2d_center(points);
        let mut radius_squared = 0.0;

        for point in points {
            // Get squared version to avoid unnecessary sqrt calls
            let distance_squared = point.distance_squared(center);
            if distance_squared > radius_squared {
                radius_squared = distance_squared;
            }
        }

        BoundingCircle::new(isometry * center, ops::sqrt(radius_squared))
    }

    /// Get the radius of the bounding circle
    #[inline]
    pub fn radius(&self) -> f32 {
        self.circle.radius
    }

    /// Computes the smallest [`Aabb2d`] containing this [`BoundingCircle`].
    #[inline]
    pub fn aabb_2d(&self) -> Aabb2d {
        Aabb2d {
            min: self.center - Vec2::splat(self.radius()),
            max: self.center + Vec2::splat(self.radius()),
        }
    }

    /// Finds the point on the bounding circle that is closest to the given `point`.
    ///
    /// If the point is outside the circle, the returned point will be on the perimeter of the circle.
    /// Otherwise, it will be inside the circle and returned as is.
    #[inline]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        self.circle.closest_point(point - self.center) + self.center
    }
}

impl BoundingVolume for BoundingCircle {
    type Translation = Vec2;
    type Rotation = Rot2;
    type HalfSize = f32;

    #[inline]
    fn center(&self) -> Self::Translation {
        self.center
    }

    #[inline]
    fn half_size(&self) -> Self::HalfSize {
        self.radius()
    }

    #[inline]
    fn visible_area(&self) -> f32 {
        core::f32::consts::PI * self.radius() * self.radius()
    }

    #[inline]
    fn contains(&self, other: &Self) -> bool {
        let diff = self.radius() - other.radius();
        self.center.distance_squared(other.center) <= ops::copysign(diff.squared(), diff)
    }

    #[inline]
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

    #[inline]
    fn grow(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        debug_assert!(amount >= 0.);
        Self::new(self.center, self.radius() + amount)
    }

    #[inline]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        debug_assert!(amount >= 0.);
        debug_assert!(self.radius() >= amount);
        Self::new(self.center, self.radius() - amount)
    }

    #[inline]
    fn scale_around_center(&self, scale: impl Into<Self::HalfSize>) -> Self {
        let scale = scale.into();
        debug_assert!(scale >= 0.);
        Self::new(self.center, self.radius() * scale)
    }

    #[inline]
    fn translate_by(&mut self, translation: impl Into<Self::Translation>) {
        self.center += translation.into();
    }

    #[inline]
    fn rotate_by(&mut self, rotation: impl Into<Self::Rotation>) {
        let rotation: Rot2 = rotation.into();
        self.center = rotation * self.center;
    }
}

impl IntersectsVolume<Self> for BoundingCircle {
    #[inline]
    fn intersects(&self, other: &Self) -> bool {
        let center_distance_squared = self.center.distance_squared(other.center);
        let radius_sum_squared = (self.radius() + other.radius()).squared();
        center_distance_squared <= radius_sum_squared
    }
}

impl IntersectsVolume<Aabb2d> for BoundingCircle {
    #[inline]
    fn intersects(&self, aabb: &Aabb2d) -> bool {
        aabb.intersects(self)
    }
}

#[cfg(test)]
mod bounding_circle_tests {
    use super::BoundingCircle;
    use crate::{
        bounding::{BoundingVolume, IntersectsVolume},
        ops, Vec2,
    };

    #[test]
    fn area() {
        let circle = BoundingCircle::new(Vec2::ONE, 5.);
        // Since this number is messy we check it with a higher threshold
        assert!(ops::abs(circle.visible_area() - 78.5398) < 0.001);
    }

    #[test]
    fn contains() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let b = BoundingCircle::new(Vec2::new(5.5, 1.), 1.);
        assert!(!a.contains(&b));
        let b = BoundingCircle::new(Vec2::new(1., -3.5), 0.5);
        assert!(a.contains(&b));
    }

    #[test]
    fn contains_identical() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        assert!(a.contains(&a));
    }

    #[test]
    fn merge() {
        // When merging two circles that don't contain each other, we find a center position that
        // contains both
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let b = BoundingCircle::new(Vec2::new(1., -4.), 1.);
        let merged = a.merge(&b);
        assert!((merged.center - Vec2::new(1., 0.5)).length() < f32::EPSILON);
        assert!(ops::abs(merged.radius() - 5.5) < f32::EPSILON);
        assert!(merged.contains(&a));
        assert!(merged.contains(&b));
        assert!(!a.contains(&merged));
        assert!(!b.contains(&merged));

        // When one circle contains the other circle, we use the bigger circle
        let b = BoundingCircle::new(Vec2::ZERO, 3.);
        assert!(a.contains(&b));
        let merged = a.merge(&b);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.radius(), a.radius());

        // When two circles are at the same point, we use the bigger radius
        let b = BoundingCircle::new(Vec2::ONE, 6.);
        let merged = a.merge(&b);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.radius(), b.radius());
    }

    #[test]
    fn merge_identical() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let merged = a.merge(&a);
        assert_eq!(merged.center, a.center);
        assert_eq!(merged.radius(), a.radius());
    }

    #[test]
    fn grow() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let padded = a.grow(1.25);
        assert!(ops::abs(padded.radius() - 6.25) < f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let shrunk = a.shrink(0.5);
        assert!(ops::abs(shrunk.radius() - 4.5) < f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let scaled = a.scale_around_center(2.);
        assert!(ops::abs(scaled.radius() - 10.) < f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn transform() {
        let a = BoundingCircle::new(Vec2::ONE, 5.0);
        let transformed = a.transformed_by(Vec2::new(2.0, -2.0), core::f32::consts::FRAC_PI_4);
        assert_eq!(
            transformed.center,
            Vec2::new(2.0, core::f32::consts::SQRT_2 - 2.0)
        );
        assert_eq!(transformed.radius(), 5.0);
    }

    #[test]
    fn closest_point() {
        let circle = BoundingCircle::new(Vec2::ZERO, 1.0);
        assert_eq!(circle.closest_point(Vec2::X * 10.0), Vec2::X);
        assert_eq!(
            circle.closest_point(Vec2::NEG_ONE * 10.0),
            Vec2::NEG_ONE.normalize()
        );
        assert_eq!(
            circle.closest_point(Vec2::new(0.25, 0.1)),
            Vec2::new(0.25, 0.1)
        );
    }

    #[test]
    fn intersect_bounding_circle() {
        let circle = BoundingCircle::new(Vec2::ZERO, 1.0);
        assert!(circle.intersects(&BoundingCircle::new(Vec2::ZERO, 1.0)));
        assert!(circle.intersects(&BoundingCircle::new(Vec2::ONE * 1.25, 1.0)));
        assert!(circle.intersects(&BoundingCircle::new(Vec2::NEG_ONE * 1.25, 1.0)));
        assert!(!circle.intersects(&BoundingCircle::new(Vec2::ONE * 1.5, 1.0)));
    }
}

/// A 2D oriented bounding box.
///
/// An oriented bounding box contains its center in world coordinates, its rotation,
/// and its half size. In local coordinates, the OBB's center is at (0, 0).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(Serialize), derive(Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Obb2d {
    /// The isometrical transformation that converts local obb coordinates to world coordinates.
    /// `isometry.translation` is equal to the center of the obb in world coordinates.
    /// `let mat: Mat2 = isometry.rotation.into()` contains the local coordinate axes.
    pub isometry: Isometry2d,
    /// The half size of the oriented bounding box
    pub half_size: Vec2,
}

impl Obb2d {
    /// Create an Obb2d from an Aabb2d.
    ///
    /// An Aabb2d is just an Obb2d without a rotation.
    #[inline]
    pub fn from_aabb_2d(aabb: &Aabb2d) -> Self {
        Obb2d {
            isometry: Isometry2d::from_translation(aabb.center()),
            half_size: aabb.half_size(),
        }
    }

    /// Gets the axes of the box in world coordinate space
    ///
    /// Conveniently, the axes are just the rotation matrix.
    /// Each axis should already be normalized.
    #[inline]
    pub fn get_axes(&self) -> Mat2 {
        self.isometry.rotation.into()
    }

    /// Gets the four corners of the obb in world coordinate space
    ///
    /// The corners are returned with the following order:
    /// [local lower left, local top left, local top right, local bottom right].
    /// Note that "local lower left" does not necessarily mean that the point is
    /// "lower left" in world space due to rotation.
    /// There is a bounding edge between adjacent entries.
    #[inline]
    pub fn get_corners(&self) -> [Vec2; 4] {
        [
            self.isometry
                .transform_point(Vec2::new(-1., -1.) * self.half_size),
            self.isometry
                .transform_point(Vec2::new(-1., 1.) * self.half_size),
            self.isometry
                .transform_point(Vec2::new(1., 1.) * self.half_size),
            self.isometry
                .transform_point(Vec2::new(1., -1.) * self.half_size),
        ]
    }

    /// Finds the point on the OBB that is closest to the given `point` in world coordinates.
    ///
    /// If the point is outside the OBB, the returned point will be on the perimeter of the OBB.
    /// Otherwise, it will be inside the OBB and returned as is.
    #[inline]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        let local_point = self.isometry.inverse().transform_point(point);
        let local_closest = local_point.clamp(-self.half_size, self.half_size);
        self.isometry.transform_point(local_closest)
    }
}

impl BoundingVolume for Obb2d {
    type Translation = Vec2;
    type Rotation = Rot2;
    type HalfSize = Vec2;

    #[inline]
    fn center(&self) -> Self::Translation {
        self.isometry.translation
    }

    #[inline]
    fn half_size(&self) -> Self::HalfSize {
        self.half_size
    }

    #[inline]
    fn visible_area(&self) -> f32 {
        let b = self.half_size * 2.;
        b.x * b.y
    }

    #[inline]
    fn contains(&self, other: &Self) -> bool {
        // Convert the corners of `other` into this OBB's coordinate system.
        let other_corners = other.get_corners();
        // Check whether all corners are within the bounds of this OBB
        other_corners.iter().all(|&point| {
            let local_corner = self.isometry.inverse().transform_point(point);
            local_corner.x <= self.half_size.x
                && local_corner.x >= -self.half_size.x
                && local_corner.y <= self.half_size.y
                && local_corner.y >= -self.half_size.y
        })
    }

    #[inline]
    fn merge(&self, _other: &Self) -> Self {
        // TODO: implement
        // It should be the *smallest* bounding box that contains
        // both self and other, which makes this trickier.
        // Tentative algorithm (does not guarantee smallest):
        // Pick the new center as the center of the 8 corner points
        // Of the eight corners, find the farthest point from the center.
        // The farthest point should be the initial half extent in one direction.
        // Between the center and this farthest point, you now have one axes of direction / line segment
        // Find the orthogonal direction (e.g. by rotating the initial direction 90 degrees).
        // Find the farthest point along the orthogonal axes to get the half extent in the other direction
        // Note: This does not find the smallest bounding box in some special cases
        // e.g. two identical squares well aligned with each other with some distance apart.
        // the ideal obb in that case is a rectangle that fits snugly on 3 sides of each squares.
        // this could be remedied by testing whether any edge connected to the farthest point
        // should be an edge/direction in the merged obb.
        todo!();
    }

    #[inline]
    fn grow(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let isometry = self.isometry;
        Self {
            isometry,
            half_size: self.half_size() + amount.into(),
        }
    }

    #[inline]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let isometry = self.isometry;
        Self {
            isometry,
            half_size: self.half_size() - amount.into(),
        }
    }

    #[inline]
    fn scale_around_center(&self, scale: impl Into<Self::HalfSize>) -> Self {
        let isometry = self.isometry;
        Self {
            isometry,
            half_size: self.half_size() * scale.into(),
        }
    }

    /// Transforms the bounding volume by first rotating it around the origin and then applying a translation.
    ///
    /// The result is an Oriented Bounding Box that encompasses the rotated shape.
    ///
    /// If 'f(x)' is the isometry (the rotation followed by the translation) to be applied
    /// to an OBB with existing isometry 'g(x)', the new isometry for the OBB is f(g(x)).
    #[inline]
    fn transform_by(
        &mut self,
        translation: impl Into<Self::Translation>,
        rotation: impl Into<Self::Rotation>,
    ) {
        let translation = translation.into();
        let rotation = rotation.into();
        let isometry = Isometry2d {
            translation,
            rotation,
        };
        self.isometry = isometry * self.isometry;
    }

    /// Translates the bounding volume by the given translation.
    ///
    /// The result is an Oriented Bounding Box that encompasses the translated shape.
    ///
    /// The translation is applied *after* `self.isometry`
    #[inline]
    fn translate_by(&mut self, translation: impl Into<Self::Translation>) {
        let isometry = Isometry2d::from_translation(translation.into());
        self.isometry = isometry * self.isometry;
    }

    /// Rotates the bounding volume around the origin by the given rotation.
    ///
    /// The result is an Oriented Bounding Box that encompasses the rotated shape.
    ///
    /// The rotation is applied *after* `self.isometry`
    #[inline]
    fn rotate_by(&mut self, rotation: impl Into<Self::Rotation>) {
        let isometry = Isometry2d::from_rotation(rotation.into());
        self.isometry = isometry * self.isometry;
    }
}

impl From<&Aabb2d> for Obb2d {
    #[inline]
    fn from(aabb: &Aabb2d) -> Self {
        Obb2d::from_aabb_2d(aabb)
    }
}

impl IntersectsVolume<Self> for Obb2d {
    #[inline]
    fn intersects(&self, other: &Self) -> bool {
        let axes = self.get_axes();
        let other_axes = other.get_axes();
        let self_points = self.get_corners();
        let other_points = other.get_corners();

        // The separating axis theorem states that two convex sets do not intersect
        // if there is an axis where the projections of the sets onto the axis
        // do not overlap. The only necessary axes to check are the surface normals
        // of the rectangles, which are equivalent to these obb's x and y axes
        [
            axes.x_axis,
            axes.y_axis,
            other_axes.x_axis,
            other_axes.y_axis,
        ]
        .iter()
        .all(|axis: &Vec2| projections_have_overlap(axis, &self_points, &other_points))
    }
}

fn projections_have_overlap(
    normal: &Vec2,
    self_points: &[Vec2; 4],
    other_points: &[Vec2; 4],
) -> bool {
    let (self_min, self_max) = self_points
        .iter()
        .map(|point| point.dot(*normal))
        .minmax()
        .into_option()
        .expect("There should be a min/max because there are elements in self_points.");
    let (other_min, other_max) = other_points
        .iter()
        .map(|point| point.dot(*normal))
        .minmax()
        .into_option()
        .expect("There should be a min/max because there are elements in other_points.");
    self_max >= other_min && self_min <= other_max
}

impl IntersectsVolume<Aabb2d> for Obb2d {
    #[inline]
    fn intersects(&self, aabb: &Aabb2d) -> bool {
        self.intersects(&Obb2d::from_aabb_2d(aabb))
    }
}

impl IntersectsVolume<BoundingCircle> for Obb2d {
    #[inline]
    fn intersects(&self, circle: &BoundingCircle) -> bool {
        let closest_point = self.closest_point(circle.center);
        let distance_squared = circle.center.distance_squared(closest_point);
        let radius_squared = circle.radius().squared();
        distance_squared <= radius_squared
    }
}

impl IntersectsVolume<Obb2d> for Aabb2d {
    #[inline]
    fn intersects(&self, obb: &Obb2d) -> bool {
        obb.intersects(self)
    }
}

impl IntersectsVolume<Obb2d> for BoundingCircle {
    #[inline]
    fn intersects(&self, obb: &Obb2d) -> bool {
        obb.intersects(self)
    }
}

#[cfg(test)]
mod obb2d_tests {
    use core::f32::consts::{FRAC_1_SQRT_2, SQRT_2};

    use approx::assert_relative_eq;

    use super::Obb2d;
    use crate::{
        bounding::{Aabb2d, BoundingCircle, BoundingVolume, IntersectsVolume},
        ops, Isometry2d, Rot2, Vec2,
    };

    #[test]
    fn center() {
        let obb = Obb2d {
            isometry: Isometry2d::from_rotation(Rot2::FRAC_PI_4),
            half_size: Vec2::new(2., 5.),
        };
        assert!((obb.center() - Vec2::new(0., 0.)).length() < f32::EPSILON);
        let obb = Obb2d {
            isometry: Isometry2d::from_translation(Vec2::new(3., -5.)),
            half_size: Vec2::new(1., 8.),
        };
        assert!((obb.center() - Vec2::new(3., -5.)).length() < f32::EPSILON);
    }

    #[test]
    fn half_size() {
        let obb = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_8,
                translation: Vec2::new(2., -8.),
            },
            half_size: Vec2::new(3., 9.),
        };
        let half_size = obb.half_size();
        assert!((half_size - Vec2::new(3., 9.)).length() < f32::EPSILON);
    }

    #[test]
    fn area() {
        let obb = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_8,
                translation: Vec2::new(2., -8.),
            },
            half_size: Vec2::new(2., 5.),
        };
        assert!(ops::abs(obb.visible_area() - 40.) < f32::EPSILON);
        let obb = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_8,
                translation: Vec2::new(2., -8.),
            },
            half_size: Vec2::new(1., 0.25),
        };
        assert!(ops::abs(obb.visible_area() - 1.) < f32::EPSILON);
    }

    #[test]
    fn contains() {
        let a = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_2,
                translation: Vec2::new(2., -8.),
            },
            half_size: Vec2::new(2., 2.),
        };
        let b = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::IDENTITY,
                translation: Vec2::new(4., -10.),
            },
            half_size: Vec2::new(3., 3.),
        };
        assert!(!a.contains(&b));
        let b = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::IDENTITY,
                translation: Vec2::new(1., -9.),
            },
            half_size: Vec2::new(1., 1.),
        };
        assert!(a.contains(&b));
    }

    #[test]
    fn grow() {
        let a = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::IDENTITY,
                translation: Vec2::new(1., -9.),
            },
            half_size: Vec2::new(2., 1.),
        };
        let padded = a.grow(Vec2::ONE);
        assert!((padded.half_size - Vec2::new(3., 2.)).length() < f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::IDENTITY,
                translation: Vec2::new(1., -9.),
            },
            half_size: Vec2::new(3., 2.),
        };
        let shrunk = a.shrink(Vec2::ONE);
        assert!((shrunk.half_size - Vec2::new(2., 1.)).length() < f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::IDENTITY,
                translation: Vec2::new(1., -9.),
            },
            half_size: Vec2::new(1., 1.),
        };
        let scaled = a.scale_around_center(Vec2::splat(2.));
        assert!((scaled.half_size - Vec2::splat(2.)).length() < f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn rotate() {
        let a = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_4,
                translation: Vec2::new(3., -2.),
            },
            half_size: Vec2::new(1., 1.),
        };
        let rotated = a.rotated_by(core::f32::consts::FRAC_PI_2);
        assert_relative_eq!(rotated.center(), Vec2::new(2., 3.), epsilon = 2e-7);
    }

    #[test]
    fn transform() {
        let a = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_4,
                translation: Vec2::new(3., -2.),
            },
            half_size: Vec2::new(1., 1.),
        };
        let transformed = a.transformed_by(Vec2::new(2.0, -2.0), core::f32::consts::FRAC_PI_2);
        assert_relative_eq!(transformed.center(), Vec2::new(4.0, 1.0), epsilon = 2e-7);
    }

    #[test]
    fn closest_point() {
        let obb = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_4,
                translation: Vec2::new(2., -3.),
            },
            half_size: Vec2::new(1., 1.),
        };
        assert_relative_eq!(
            obb.closest_point(Vec2::new(20., -3.)),
            Vec2::new(2. + SQRT_2, -3.),
            epsilon = 2e-7
        );
        assert_relative_eq!(
            obb.closest_point(Vec2::new(
                2. + FRAC_1_SQRT_2 + 20.,
                -3. - FRAC_1_SQRT_2 - 20.
            )),
            Vec2::new(2. + FRAC_1_SQRT_2, -3. - FRAC_1_SQRT_2),
            epsilon = 2e-6
        );
        assert_relative_eq!(
            obb.closest_point(Vec2::new(2.25, -3.1)),
            Vec2::new(2.25, -3.1),
            epsilon = 2e-7
        );
    }

    #[test]
    fn intersect_obb() {
        let obb: Obb2d = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_4,
                translation: Vec2::new(2., -3.),
            },
            half_size: Vec2::new(1., 1.),
        };
        assert!(obb.intersects(&obb));
        // contains the other obb2d
        assert!(obb.intersects(&Obb2d {
            isometry: Isometry2d::from_translation(Vec2::new(2., -3.)),
            half_size: Vec2::new(0.5, 0.5)
        }));
        // has some area overlap
        assert!(obb.intersects(&Obb2d {
            isometry: Isometry2d::from_translation(Vec2::new(2., -2.)),
            half_size: Vec2::new(1., 1.)
        }));
        // touches a corner
        assert!(obb.intersects(&Obb2d {
            isometry: Isometry2d::from_translation(Vec2::new(3. + FRAC_1_SQRT_2, -3.)),
            half_size: Vec2::new(1., 1.)
        }));
        // does not intersect
        assert!(!obb.intersects(&Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_2,
                translation: Vec2::new(2., -6.)
            },
            half_size: Vec2::new(1., 1.)
        }));
    }

    #[test]
    fn intersect_abb() {
        let obb: Obb2d = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_4,
                translation: Vec2::new(2., -3.),
            },
            half_size: Vec2::new(1., 1.),
        };
        assert!(obb.intersects(&Aabb2d {
            min: Vec2::new(1.5, -3.5),
            max: Vec2::new(2.5, -2.5),
        }));
        assert!(obb.intersects(&Aabb2d {
            min: Vec2::new(3., -4.),
            max: Vec2::new(4., -2.),
        }));
        assert!(obb.intersects(&Aabb2d {
            min: Vec2::new(1., -5.),
            max: Vec2::new(3., -3. - FRAC_1_SQRT_2),
        }));
        assert!(!obb.intersects(&Aabb2d {
            min: Vec2::new(1.1, 0.0),
            max: Vec2::new(2.0, 0.5),
        }));
    }

    #[test]
    fn intersect_bounding_circle() {
        let obb: Obb2d = Obb2d {
            isometry: Isometry2d {
                rotation: Rot2::FRAC_PI_4,
                translation: Vec2::new(2., -3.),
            },
            half_size: Vec2::new(1., 1.),
        };
        // contains
        assert!(obb.intersects(&BoundingCircle::new(Vec2::new(2., -3.), 1.0)));
        // overlaps
        assert!(obb.intersects(&BoundingCircle::new(Vec2::new(1., -3.), 1.0)));
        // touching a corner
        assert!(obb.intersects(&BoundingCircle::new(
            Vec2::new(4. + FRAC_1_SQRT_2, -3.),
            2.0
        )));
        assert!(!obb.intersects(&BoundingCircle::new(Vec2::ZERO, 2.0)));
    }
}
