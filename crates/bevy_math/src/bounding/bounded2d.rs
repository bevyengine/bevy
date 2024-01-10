use super::BoundingVolume;
use crate::prelude::Vec2;

/// A trait with methods that return 2D bounded volumes for a shape
pub trait Bounded2d {
    /// Get an axis-aligned bounding box for the shape with the given translation and rotation.
    /// The rotation is in radians, counterclockwise, with 0 meaning no rotation.
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d;
    /// Get a bounding circle for the shape
    /// The rotation is in radians, counterclockwise, with 0 meaning no rotation.
    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle;
}

/// A 2D axis-aligned bounding box, or bounding rectangle
#[doc(alias = "BoundingRectangle")]
#[derive(Clone, Debug)]
pub struct Aabb2d {
    /// The minimum, conventionally bottom-left, point of the box
    pub min: Vec2,
    /// The maximum, conventionally top-right, point of the box
    pub max: Vec2,
}

impl BoundingVolume for Aabb2d {
    type Position = Vec2;
    type HalfSize = Vec2;

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
    fn grow(&self, amount: Self::HalfSize) -> Self {
        let b = Self {
            min: self.min - amount,
            max: self.max + amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    #[inline(always)]
    fn shrink(&self, amount: Self::HalfSize) -> Self {
        let b = Self {
            min: self.min + amount,
            max: self.max - amount,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }
}

#[cfg(test)]
mod aabb2d_tests {
    use super::Aabb2d;
    use crate::{bounding::BoundingVolume, Vec2};

    #[test]
    fn center() {
        let aabb = Aabb2d {
            min: Vec2::new(-0.5, -1.),
            max: Vec2::new(1., 1.),
        };
        assert!((aabb.center() - Vec2::new(0.25, 0.)).length() < std::f32::EPSILON);
        let aabb = Aabb2d {
            min: Vec2::new(5., -10.),
            max: Vec2::new(10., -5.),
        };
        assert!((aabb.center() - Vec2::new(7.5, -7.5)).length() < std::f32::EPSILON);
    }

    #[test]
    fn half_size() {
        let aabb = Aabb2d {
            min: Vec2::new(-0.5, -1.),
            max: Vec2::new(1., 1.),
        };
        let half_size = aabb.half_size();
        assert!((half_size - Vec2::new(0.75, 1.)).length() < std::f32::EPSILON);
    }

    #[test]
    fn area() {
        let aabb = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 1.),
        };
        assert!((aabb.visible_area() - 4.).abs() < std::f32::EPSILON);
        let aabb = Aabb2d {
            min: Vec2::new(0., 0.),
            max: Vec2::new(1., 0.5),
        };
        assert!((aabb.visible_area() - 0.5).abs() < std::f32::EPSILON);
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
        assert!((merged.min - Vec2::new(-2., -1.)).length() < std::f32::EPSILON);
        assert!((merged.max - Vec2::new(1., 1.)).length() < std::f32::EPSILON);
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
        assert!((padded.min - Vec2::new(-2., -2.)).length() < std::f32::EPSILON);
        assert!((padded.max - Vec2::new(2., 2.)).length() < std::f32::EPSILON);
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
        assert!((shrunk.min - Vec2::new(-1., -1.)).length() < std::f32::EPSILON);
        assert!((shrunk.max - Vec2::new(1., 1.)).length() < std::f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }
}

use crate::primitives::Circle;

/// A bounding circle
#[derive(Clone, Debug)]
pub struct BoundingCircle {
    /// The center of the bounding circle
    pub center: Vec2,
    /// The circle
    pub circle: Circle,
}

impl BoundingCircle {
    /// Construct a bounding circle from its center and radius
    #[inline(always)]
    pub fn new(center: Vec2, radius: f32) -> Self {
        debug_assert!(radius >= 0.);
        Self {
            center,
            circle: Circle { radius },
        }
    }

    /// Get the radius of the bounding circle
    #[inline(always)]
    pub fn radius(&self) -> f32 {
        self.circle.radius
    }
}

impl BoundingVolume for BoundingCircle {
    type Position = Vec2;
    type HalfSize = f32;

    #[inline(always)]
    fn center(&self) -> Self::Position {
        self.center
    }

    #[inline(always)]
    fn half_size(&self) -> Self::HalfSize {
        self.circle.radius
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
            return self.clone();
        }
        if other.radius() >= length + self.radius() {
            return other.clone();
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
        Self::new(self.center, self.radius() + amount)
    }

    #[inline(always)]
    fn shrink(&self, amount: Self::HalfSize) -> Self {
        debug_assert!(amount >= 0.);
        debug_assert!(self.radius() >= amount);
        Self::new(self.center, self.radius() - amount)
    }
}

#[cfg(test)]
mod bounding_circle_tests {
    use super::BoundingCircle;
    use crate::{bounding::BoundingVolume, Vec2};

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
        assert!((merged.center - Vec2::new(1., 0.5)).length() < std::f32::EPSILON);
        assert!((merged.radius() - 5.5).abs() < std::f32::EPSILON);
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
        assert!((padded.radius() - 6.25).abs() < std::f32::EPSILON);
        assert!(padded.contains(&a));
        assert!(!a.contains(&padded));
    }

    #[test]
    fn shrink() {
        let a = BoundingCircle::new(Vec2::ONE, 5.);
        let shrunk = a.shrink(0.5);
        assert!((shrunk.radius() - 4.5).abs() < std::f32::EPSILON);
        assert!(a.contains(&shrunk));
        assert!(!shrunk.contains(&a));
    }
}
