mod primitive_impls;

use super::{BoundingVolume, IntersectsVolume};
use crate::prelude::{Mat2, Rot2, Vec2};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// Computes the geometric center of the given set of points.
#[inline(always)]
fn point_cloud_2d_center(points: &[Vec2]) -> Vec2 {
    assert!(
        !points.is_empty(),
        "cannot compute the center of an empty set of points"
    );

    let denom = 1.0 / points.len() as f32;
    points.iter().fold(Vec2::ZERO, |acc, point| acc + *point) * denom
}

/// A trait with methods that return 2D bounded volumes for a shape
pub trait Bounded2d {
    /// Get an axis-aligned bounding box for the shape with the given translation and rotation.
    /// The rotation is in radians, counterclockwise, with 0 meaning no rotation.
    fn aabb_2d(&self, translation: Vec2, rotation: impl Into<Rot2>) -> Aabb2d;
    /// Get a bounding circle for the shape
    /// The rotation is in radians, counterclockwise, with 0 meaning no rotation.
    fn bounding_circle(&self, translation: Vec2, rotation: impl Into<Rot2>) -> BoundingCircle;
}

/// A 2D axis-aligned bounding box, or bounding rectangle
#[doc(alias = "BoundingRectangle")]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct Aabb2d {
    /// The minimum, conventionally bottom-left, point of the box
    pub min: Vec2,
    /// The maximum, conventionally top-right, point of the box
    pub max: Vec2,
}

impl Aabb2d {
    /// Constructs an AABB from its center and half-size.
    #[inline(always)]
    pub fn new(center: Vec2, half_size: Vec2) -> Self {
        debug_assert!(half_size.x >= 0.0 && half_size.y >= 0.0);
        Self {
            min: center - half_size,
            max: center + half_size,
        }
    }

    /// Computes the smallest [`Aabb2d`] containing the given set of points,
    /// transformed by `translation` and `rotation`.
    ///
    /// # Panics
    ///
    /// Panics if the given set of points is empty.
    #[inline(always)]
    pub fn from_point_cloud(
        translation: Vec2,
        rotation: impl Into<Rot2>,
        points: &[Vec2],
    ) -> Aabb2d {
        // Transform all points by rotation
        let rotation: Rot2 = rotation.into();
        let mut iter = points.iter().map(|point| rotation * *point);

        let first = iter
            .next()
            .expect("point cloud must contain at least one point for Aabb2d construction");

        let (min, max) = iter.fold((first, first), |(prev_min, prev_max), point| {
            (point.min(prev_min), point.max(prev_max))
        });

        Aabb2d {
            min: min + translation,
            max: max + translation,
        }
    }

    /// Computes the smallest [`BoundingCircle`] containing this [`Aabb2d`].
    #[inline(always)]
    pub fn bounding_circle(&self) -> BoundingCircle {
        let radius = self.min.distance(self.max) / 2.0;
        BoundingCircle::new(self.center(), radius)
    }

    /// Finds the point on the AABB that is closest to the given `point`.
    ///
    /// If the point is outside the AABB, the returned point will be on the perimeter of the AABB.
    /// Otherwise, it will be inside the AABB and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        // Clamp point coordinates to the AABB
        point.clamp(self.min, self.max)
    }
}

impl BoundingVolume for Aabb2d {
    type Translation = Vec2;
    type Rotation = Rot2;
    type HalfSize = Vec2;

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
        b.x * b.y
    }

    #[inline(always)]
    fn contains(&self, other: &Self) -> bool {
        other.min.x >= self.min.x
            && other.min.y >= self.min.y
            && other.max.x <= self.max.x
            && other.max.y <= self.max.y
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
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    #[inline(always)]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        let b = Self {
            min: self.min + amount,
            max: self.max - amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    #[inline(always)]
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
        let rotation: Rot2 = rotation.into();
        let abs_rot_mat = Mat2::from_cols(
            Vec2::new(rotation.cos, rotation.sin),
            Vec2::new(rotation.sin, rotation.cos),
        );
        let half_size = abs_rot_mat * self.half_size();
        *self = Self::new(rotation * self.center(), half_size);
    }
}

impl IntersectsVolume<Self> for Aabb2d {
    #[inline(always)]
    fn intersects(&self, other: &Self) -> bool {
        let x_overlaps = self.min.x <= other.max.x && self.max.x >= other.min.x;
        let y_overlaps = self.min.y <= other.max.y && self.max.y >= other.min.y;
        x_overlaps && y_overlaps
    }
}

impl IntersectsVolume<BoundingCircle> for Aabb2d {
    #[inline(always)]
    fn intersects(&self, circle: &BoundingCircle) -> bool {
        let closest_point = self.closest_point(circle.center);
        let distance_squared = circle.center.distance_squared(closest_point);
        let radius_squared = circle.radius().powi(2);
        distance_squared <= radius_squared
    }
}

#[cfg(test)]
mod aabb2d_tests {
    use super::Aabb2d;
    use crate::{
        bounding::{BoundingCircle, BoundingVolume, IntersectsVolume},
        Vec2,
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
        assert!((aabb.visible_area() - 4.).abs() < f32::EPSILON);
        let aabb = Aabb2d {
            min: Vec2::new(0., 0.),
            max: Vec2::new(1., 0.5),
        };
        assert!((aabb.visible_area() - 0.5).abs() < f32::EPSILON);
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
    fn transform() {
        let a = Aabb2d {
            min: Vec2::new(-2.0, -2.0),
            max: Vec2::new(2.0, 2.0),
        };
        let transformed = a.transformed_by(Vec2::new(2.0, -2.0), std::f32::consts::FRAC_PI_4);
        let half_length = 2_f32.hypot(2.0);
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
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct BoundingCircle {
    /// The center of the bounding circle
    pub center: Vec2,
    /// The circle
    pub circle: Circle,
}

impl BoundingCircle {
    /// Constructs a bounding circle from its center and radius.
    #[inline(always)]
    pub fn new(center: Vec2, radius: f32) -> Self {
        debug_assert!(radius >= 0.);
        Self {
            center,
            circle: Circle { radius },
        }
    }

    /// Computes a [`BoundingCircle`] containing the given set of points,
    /// transformed by `translation` and `rotation`.
    ///
    /// The bounding circle is not guaranteed to be the smallest possible.
    #[inline(always)]
    pub fn from_point_cloud(
        translation: Vec2,
        rotation: impl Into<Rot2>,
        points: &[Vec2],
    ) -> BoundingCircle {
        let rotation: Rot2 = rotation.into();
        let center = point_cloud_2d_center(points);
        let mut radius_squared = 0.0;

        for point in points {
            // Get squared version to avoid unnecessary sqrt calls
            let distance_squared = point.distance_squared(center);
            if distance_squared > radius_squared {
                radius_squared = distance_squared;
            }
        }

        BoundingCircle::new(rotation * center + translation, radius_squared.sqrt())
    }

    /// Get the radius of the bounding circle
    #[inline(always)]
    pub fn radius(&self) -> f32 {
        self.circle.radius
    }

    /// Computes the smallest [`Aabb2d`] containing this [`BoundingCircle`].
    #[inline(always)]
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
    #[inline(always)]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        self.circle.closest_point(point - self.center) + self.center
    }
}

impl BoundingVolume for BoundingCircle {
    type Translation = Vec2;
    type Rotation = Rot2;
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
        std::f32::consts::PI * self.radius() * self.radius()
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
        Self::new(self.center, self.radius() + amount)
    }

    #[inline(always)]
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self {
        let amount = amount.into();
        debug_assert!(amount >= 0.);
        debug_assert!(self.radius() >= amount);
        Self::new(self.center, self.radius() - amount)
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
        let rotation: Rot2 = rotation.into();
        self.center = rotation * self.center;
    }
}

impl IntersectsVolume<Self> for BoundingCircle {
    #[inline(always)]
    fn intersects(&self, other: &Self) -> bool {
        let center_distance_squared = self.center.distance_squared(other.center);
        let radius_sum_squared = (self.radius() + other.radius()).powi(2);
        center_distance_squared <= radius_sum_squared
    }
}

impl IntersectsVolume<Aabb2d> for BoundingCircle {
    #[inline(always)]
    fn intersects(&self, aabb: &Aabb2d) -> bool {
        aabb.intersects(self)
    }
}

#[cfg(test)]
mod bounding_circle_tests {
    use super::BoundingCircle;
    use crate::{
        bounding::{BoundingVolume, IntersectsVolume},
        Vec2,
    };

    #[test]
    fn area() {
        let circle = BoundingCircle::new(Vec2::ONE, 5.);
        // Since this number is messy we check it with a higher threshold
        assert!((circle.visible_area() - 78.5398).abs() < 0.001);
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
        assert!((merged.radius() - 5.5).abs() < f32::EPSILON);
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
        assert!((padded.radius() - 6.25).abs() < f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let shrunk = a.shrink(0.5);
        assert!((shrunk.radius() - 4.5).abs() < f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }

    #[test]
    fn scale_around_center() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let scaled = a.scale_around_center(2.);
        assert!((scaled.radius() - 10.).abs() < f32::EPSILON);
        assert!(!a.contains(&scaled));
        assert!(scaled.contains(&a));
    }

    #[test]
    fn transform() {
        let a = BoundingCircle::new(Vec2::ONE, 5.0);
        let transformed = a.transformed_by(Vec2::new(2.0, -2.0), std::f32::consts::FRAC_PI_4);
        assert_eq!(
            transformed.center,
            Vec2::new(2.0, std::f32::consts::SQRT_2 - 2.0)
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
