use super::BoundingVolume;
use crate::prelude::Vec2;

/// A trait with methods that return 2d bounded volumes for a shape
pub trait Bounded2d {
    /// Get an axis-aligned bounding box for the shape with the given translation and rotation
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d;
    /// Get a bounding circle for the shape
    fn bounding_circle(&self, translation: Vec2) -> BoundingCircle;
}

/// A 2D axis-aligned bounding box, or bounding rectangle
pub struct Aabb2d {
    /// The minimum point of the box
    pub min: Vec2,
    /// The maximum point of the box
    pub max: Vec2,
}

impl BoundingVolume for Aabb2d {
    type Position = Vec2;
    type Padding = (Vec2, Vec2);

    #[inline(always)]
    fn center(&self) -> Self::Position {
        (self.min + self.max) / 2.
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
    fn merged(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline(always)]
    fn padded(&self, amount: Self::Padding) -> Self {
        let b = Self {
            min: self.min - amount.0,
            max: self.max + amount.1,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y);
        b
    }

    #[inline(always)]
    fn shrunk(&self, amount: Self::Padding) -> Self {
        let b = Self {
            min: self.min + amount.0,
            max: self.max - amount.1,
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
    fn merged() {
        let a = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 0.5),
        };
        let b = Aabb2d {
            min: Vec2::new(-2., -0.5),
            max: Vec2::new(0.75, 1.),
        };
        let merged = a.merged(&b);
        assert!((merged.min - Vec2::new(-2., -1.)).length() < std::f32::EPSILON);
        assert!((merged.max - Vec2::new(1., 1.)).length() < std::f32::EPSILON);
    }

    #[test]
    fn padded() {
        let a = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 1.),
        };
        let padded = a.padded((Vec2::ONE, Vec2::Y));
        assert!((padded.min - Vec2::new(-2., -2.)).length() < std::f32::EPSILON);
        assert!((padded.max - Vec2::new(1., 2.)).length() < std::f32::EPSILON);
    }

    #[test]
    fn shrunk() {
        let a = Aabb2d {
            min: Vec2::new(-1., -1.),
            max: Vec2::new(1., 1.),
        };
        let shrunk = a.shrunk((Vec2::ONE, Vec2::Y));
        assert!((shrunk.min - Vec2::new(-0., -0.)).length() < std::f32::EPSILON);
        assert!((shrunk.max - Vec2::new(1., 0.)).length() < std::f32::EPSILON);
    }
}

/// A bounding circle
pub struct BoundingCircle {
    /// The center of the bounding circle
    pub center: Vec2,
    /// The radius of the bounding circle
    pub radius: f32,
}

impl BoundingVolume for BoundingCircle {
    type Position = Vec2;
    type Padding = f32;

    #[inline(always)]
    fn center(&self) -> Self::Position {
        self.center
    }

    #[inline(always)]
    fn visible_area(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius
    }

    #[inline(always)]
    fn contains(&self, other: &Self) -> bool {
        if other.center == self.center {
            other.radius <= self.radius
        } else {
            let furthest_point = (other.center - self.center).length() + other.radius;
            furthest_point <= self.radius
        }
    }

    #[inline(always)]
    fn merged(&self, other: &Self) -> Self {
        if other.center == self.center {
            Self {
                center: self.center,
                radius: self.radius.max(other.radius),
            }
        } else {
            let diff = other.center - self.center;
            let length = diff.length();
            let dir = diff / length;

            Self {
                center: self.center + dir * ((length + other.radius - self.radius) / 2.),
                radius: (length + self.radius + other.radius) / 2.,
            }
        }
    }

    #[inline(always)]
    fn padded(&self, amount: Self::Padding) -> Self {
        Self {
            center: self.center,
            radius: self.radius + amount,
        }
    }

    #[inline(always)]
    fn shrunk(&self, amount: Self::Padding) -> Self {
        debug_assert!(self.radius >= amount);
        Self {
            center: self.center,
            radius: self.radius - amount,
        }
    }
}

#[cfg(test)]
mod bounding_circle_tests {
    use super::BoundingCircle;
    use crate::{bounding::BoundingVolume, Vec2};

    #[test]
    fn area() {
        let circle = BoundingCircle {
            center: Vec2::ONE,
            radius: 5.,
        };
        // Since this number is messy we check it with a higher threshold
        assert!((circle.visible_area() - 78.5398).abs() < 0.001);
    }

    #[test]
    fn contains() {
        let a = BoundingCircle {
            center: Vec2::ONE,
            radius: 5.,
        };
        let b = BoundingCircle {
            center: Vec2::new(5.5, 1.),
            radius: 1.,
        };
        assert!(!a.contains(&b));
        let b = BoundingCircle {
            center: Vec2::new(1., -3.5),
            radius: 0.5,
        };
        assert!(a.contains(&b));
    }

    #[test]
    fn merged() {
        let a = BoundingCircle {
            center: Vec2::ONE,
            radius: 5.,
        };
        let b = BoundingCircle {
            center: Vec2::new(1., -4.),
            radius: 1.,
        };
        let merged = a.merged(&b);
        assert!((merged.center - Vec2::new(1., 0.5)).length() < std::f32::EPSILON);
        assert!((merged.radius - 5.5).abs() < std::f32::EPSILON);
    }

    #[test]
    fn padded() {
        let a = BoundingCircle {
            center: Vec2::ONE,
            radius: 5.,
        };
        let padded = a.padded(1.25);
        assert!((padded.radius - 6.25).abs() < std::f32::EPSILON);
    }

    #[test]
    fn shrunk() {
        let a = BoundingCircle {
            center: Vec2::ONE,
            radius: 5.,
        };
        let shrunk = a.shrunk(0.5);
        assert!((shrunk.radius - 4.5).abs() < std::f32::EPSILON);
    }
}
