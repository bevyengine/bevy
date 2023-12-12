use super::BoundingVolume;
use crate::prelude::{Quat, Vec3};

/// A trait with methods that return 3d bounded volumes for a shape
pub trait Bounded3d {
    /// Get an axis-aligned bounding box for the shape with the given translation and rotation
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d;
    /// Get a bounding sphere for the shape
    fn bounding_sphere(&self, translation: Vec3) -> BoundingSphere;
}

/// A 3D axis-aligned bounding box
pub struct Aabb3d {
    /// The minimum point of the box
    min: Vec3,
    /// The maximum point of the box
    max: Vec3,
}

impl BoundingVolume for Aabb3d {
    type Position = Vec3;
    type Padding = (Vec3, Vec3);

    #[inline(always)]
    fn center(&self) -> Self::Position {
        (self.min + self.max) / 2.
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
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y && b.min.z <= b.max.z);
        b
    }

    #[inline(always)]
    fn shrunk(&self, amount: Self::Padding) -> Self {
        let b = Self {
            min: self.min + amount.0,
            max: self.max - amount.1,
        };
        debug_assert!(b.min.x <= b.max.x && b.min.y <= b.max.y && b.min.z <= b.max.z);
        b
    }
}

#[test]
fn test_aabb3d_center() {
    let aabb = Aabb3d {
        min: Vec3::new(-0.5, -1., -0.5),
        max: Vec3::new(1., 1., 2.),
    };
    assert!((aabb.center() - Vec3::new(0.25, 0., 0.75)).length() < 0.0001);
    let aabb = Aabb3d {
        min: Vec3::new(5., 5., -10.),
        max: Vec3::new(10., 10., -5.),
    };
    assert!((aabb.center() - Vec3::new(7.5, 7.5, -7.5)).length() < 0.0001);
}

#[test]
fn test_aabb3d_area() {
    let aabb = Aabb3d {
        min: Vec3::new(-1., -1., -1.),
        max: Vec3::new(1., 1., 1.),
    };
    assert!((aabb.visible_area() - 12.).abs() < 0.0001);
    let aabb = Aabb3d {
        min: Vec3::new(0., 0., 0.),
        max: Vec3::new(1., 0.5, 0.25),
    };
    assert!((aabb.visible_area() - 0.875).abs() < 0.0001);
}

#[test]
fn test_aabb3d_contains() {
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
fn test_aabb3d_merged() {
    let a = Aabb3d {
        min: Vec3::new(-1., -1., -1.),
        max: Vec3::new(1., 0.5, 1.),
    };
    let b = Aabb3d {
        min: Vec3::new(-2., -0.5, -0.),
        max: Vec3::new(0.75, 1., 2.),
    };
    let merged = a.merged(&b);
    assert!((merged.min - Vec3::new(-2., -1., -1.)).length() < 0.0001);
    assert!((merged.max - Vec3::new(1., 1., 2.)).length() < 0.0001);
}

#[test]
fn test_aabb3d_padded() {
    let a = Aabb3d {
        min: Vec3::new(-1., -1., -1.),
        max: Vec3::new(1., 1., 1.),
    };
    let padded = a.padded((Vec3::ONE, Vec3::Y));
    assert!((padded.min - Vec3::new(-2., -2., -2.)).length() < 0.0001);
    assert!((padded.max - Vec3::new(1., 2., 1.)).length() < 0.0001);
}

#[test]
fn test_aabb3d_shrunk() {
    let a = Aabb3d {
        min: Vec3::new(-1., -1., -1.),
        max: Vec3::new(1., 1., 1.),
    };
    let shrunk = a.shrunk((Vec3::ONE, Vec3::Y));
    assert!((shrunk.min - Vec3::new(-0., -0., -0.)).length() < 0.0001);
    assert!((shrunk.max - Vec3::new(1., 0., 1.)).length() < 0.0001);
}

/// A bounding sphere
pub struct BoundingSphere {
    /// The center of the bounding sphere
    center: Vec3,
    /// The radius of the bounding sphere
    radius: f32,
}

impl BoundingVolume for BoundingSphere {
    type Position = Vec3;
    type Padding = f32;

    #[inline(always)]
    fn center(&self) -> Self::Position {
        self.center
    }

    #[inline(always)]
    fn visible_area(&self) -> f32 {
        2. * std::f32::consts::PI * self.radius * self.radius
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

#[test]
fn test_bounding_sphere_area() {
    let sphere = BoundingSphere {
        center: Vec3::ONE,
        radius: 5.,
    };
    assert!((sphere.visible_area() - 157.0796).abs() < 0.001);
}

#[test]
fn test_bounding_sphere_contains() {
    let a = BoundingSphere {
        center: Vec3::ONE,
        radius: 5.,
    };
    let b = BoundingSphere {
        center: Vec3::new(5.5, 1., 1.),
        radius: 1.,
    };
    assert!(!a.contains(&b));
    let b = BoundingSphere {
        center: Vec3::new(1., -3.5, 1.),
        radius: 0.5,
    };
    assert!(a.contains(&b));
}

#[test]
fn test_bounding_sphere_merged() {
    let a = BoundingSphere {
        center: Vec3::ONE,
        radius: 5.,
    };
    let b = BoundingSphere {
        center: Vec3::new(1., 1., -4.),
        radius: 1.,
    };
    let merged = a.merged(&b);
    assert!((merged.center - Vec3::new(1., 1., 0.5)).length() < 0.0001);
    assert!((merged.radius - 5.5).abs() < 0.0001);
}

#[test]
fn test_bounding_sphere_padded() {
    let a = BoundingSphere {
        center: Vec3::ONE,
        radius: 5.,
    };
    let padded = a.padded(1.25);
    assert!((padded.radius - 6.25).abs() < 0.0001);
}

#[test]
fn test_bounding_sphere_shrunk() {
    let a = BoundingSphere {
        center: Vec3::ONE,
        radius: 5.,
    };
    let shrunk = a.shrunk(0.5);
    assert!((shrunk.radius - 4.5).abs() < 0.0001);
}
