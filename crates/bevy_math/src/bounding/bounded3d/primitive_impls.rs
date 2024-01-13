//! Contains [`Bounded3d`](super::Bounded3d) implementations for [geometric primitives](crate::primitives).

use glam::{Mat3, Quat, Vec2, Vec3};

use crate::primitives::{
    BoxedPolyline3d, Capsule, Cone, ConicalFrustum, Cuboid, Cylinder, Direction3d, Line3d, Plane3d,
    Polyline3d, Segment3d, Sphere, Torus,
};

use super::{Aabb3d, Bounded3d, BoundingSphere};

impl Bounded3d for Sphere {
    fn aabb_3d(&self, translation: Vec3, _rotation: Quat) -> Aabb3d {
        Aabb3d {
            min: translation - Vec3::splat(self.radius),
            max: translation + Vec3::splat(self.radius),
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere::new(translation, self.radius)
    }
}

impl Bounded3d for Plane3d {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Get a direction along the plane rotated by `rotation`
        let direction = (rotation * *self.normal).any_orthonormal_vector();
        let facing_x = direction == Vec3::X || direction == Vec3::NEG_X;
        let facing_y = direction == Vec3::Y || direction == Vec3::NEG_Y;
        let facing_z = direction == Vec3::Z || direction == Vec3::NEG_Z;

        // Dividing `f32::MAX` by 2.0 can actually be good so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let max = f32::MAX / 2.0;
        let half_width = if facing_y && facing_z { 0.0 } else { max };
        let half_height = if facing_x && facing_z { 0.0 } else { max };
        let half_depth = if facing_x && facing_y { 0.0 } else { max };
        let half_size = Vec3::new(half_width, half_height, half_depth);

        Aabb3d {
            min: translation - half_size,
            max: translation + half_size,
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere::new(translation, f32::MAX / 2.0)
    }
}

impl Bounded3d for Line3d {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Get the line direction along the plane rotated by `rotation`
        let direction = rotation * *self.direction;
        let x_parallel = direction == Vec3::X || direction == Vec3::NEG_X;
        let y_parallel = direction == Vec3::Y || direction == Vec3::NEG_Y;
        let z_parallel = direction == Vec3::Z || direction == Vec3::NEG_Z;

        // Dividing `f32::MAX` by 2.0 can actually be good so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let max = f32::MAX / 2.0;
        let half_width = if y_parallel && z_parallel { 0.0 } else { max };
        let half_height = if x_parallel && z_parallel { 0.0 } else { max };
        let half_depth = if x_parallel && y_parallel { 0.0 } else { max };
        let half_size = Vec3::new(half_width, half_height, half_depth);

        Aabb3d {
            min: translation - half_size,
            max: translation + half_size,
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere::new(translation, f32::MAX / 2.0)
    }
}

impl Bounded3d for Segment3d {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Rotate the segment by `rotation`
        let direction = Direction3d::from_normalized(rotation * *self.direction);
        let segment = Self { direction, ..*self };
        let (point1, point2) = (segment.point1(), segment.point2());

        Aabb3d {
            min: translation + point1.min(point2),
            max: translation + point1.max(point2),
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere::new(translation, self.half_length)
    }
}

impl<const N: usize> Bounded3d for Polyline3d<N> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        Aabb3d::from_point_cloud(translation, rotation, self.vertices)
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        BoundingSphere::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded3d for BoxedPolyline3d {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        Aabb3d::from_point_cloud(translation, rotation, self.vertices.to_vec())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        BoundingSphere::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded3d for Cuboid {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Compute the AABB of the rotated cuboid by transforming the half-extents
        // by an absolute rotation matrix.
        let rot_mat = Mat3::from_quat(rotation);
        let abs_rot_mat = Mat3::from_cols(
            rot_mat.x_axis.abs(),
            rot_mat.y_axis.abs(),
            rot_mat.z_axis.abs(),
        );

        let half_extents = abs_rot_mat * self.half_extents;

        Aabb3d {
            min: translation - half_extents,
            max: translation + half_extents,
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere {
            center: translation,
            sphere: Sphere {
                radius: self.half_extents.length(),
            },
        }
    }
}

impl Bounded3d for Cylinder {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let top = rotation * Vec3::Y * self.half_height;
        let bottom = -top;
        let segment = bottom - top;

        let e = 1.0 - segment * segment / segment.length_squared();
        let half_extents = self.radius * Vec3::new(e.x.sqrt(), e.y.sqrt(), e.z.sqrt());

        Aabb3d {
            min: translation + (top - half_extents).min(bottom - half_extents),
            max: translation + (top + half_extents).max(bottom + half_extents),
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        let radius = (self.radius.powi(2) + self.half_height.powi(2)).sqrt();
        BoundingSphere::new(translation, radius)
    }
}

impl Bounded3d for Capsule {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Get the line segment between the hemispheres of the rotated capsule
        let segment = Segment3d {
            direction: Direction3d::from_normalized(rotation * Vec3::Y),
            half_length: self.half_length,
        };
        let (a, b) = (segment.point1(), segment.point2());

        // Expand the line segment by the capsule radius to get the capsule half-extents
        let min = a.min(b) - Vec3::splat(self.radius);
        let max = a.max(b) + Vec3::splat(self.radius);

        Aabb3d {
            min: min + translation,
            max: max + translation,
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere::new(translation, self.radius + self.half_length)
    }
}

impl Bounded3d for Cone {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let top = rotation * Vec3::Y * 0.5 * self.height;
        let bottom = -top;
        let segment = bottom - top;

        let e = 1.0 - segment * segment / segment.length_squared();
        let half_extents = Vec3::new(e.x.sqrt(), e.y.sqrt(), e.z.sqrt());

        Aabb3d {
            min: translation + top.min(bottom - self.radius * half_extents),
            max: translation + top.max(bottom + self.radius * half_extents),
        }
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        // To compute the bounding sphere, we'll get the circumcenter U and circumradius R
        // of the circumcircle passing through all three vertices of the triangular
        // cross-section of the cone.
        //
        // Here, we assume the tip A is translated to the origin, which simplifies calculations.
        //
        //     A = (0, 0)
        //         *
        //        / \
        //       /   \
        //      /     \
        //     /       \
        //    /         \
        //   /     U     \
        //  /             \
        // *---------------*
        // B                C

        let b = Vec2::new(-self.radius, -self.height);
        let c = Vec2::new(self.radius, -self.height);
        let b_length_sq = b.length_squared();
        let c_length_sq = c.length_squared();

        // Reference: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2
        let inv_d = (2.0 * (b.x * c.y - b.y * c.x)).recip();
        let ux = inv_d * (c.y * b_length_sq - b.y * c_length_sq);
        let uy = inv_d * (b.x * c_length_sq - c.x * b_length_sq);
        let u = Vec2::new(ux, uy);

        // Compute true circumcenter and circumradius, adding the tip coordinate so that
        // A is translated back to its actual coordinate.
        let circumcenter = u + 0.5 * self.height * Vec2::Y;
        let circumradius = u.length();

        BoundingSphere::new(
            translation + rotation * circumcenter.extend(0.0),
            circumradius,
        )
    }
}

impl Bounded3d for ConicalFrustum {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let top = rotation * Vec3::Y * 0.5 * self.height;
        let bottom = -top;
        let segment = bottom - top;

        let e = 1.0 - segment * segment / segment.length_squared();
        let half_extents = Vec3::new(e.x.sqrt(), e.y.sqrt(), e.z.sqrt());

        Aabb3d {
            min: translation
                + (top - self.radius_top * half_extents)
                    .min(bottom - self.radius_bottom * half_extents),
            max: translation
                + (top + self.radius_top * half_extents)
                    .max(bottom + self.radius_bottom * half_extents),
        }
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        let half_height = 0.5 * self.height;

        // To compute the bounding sphere, we'll get the center and radius of the circumcircle
        // passing through all four vertices of the trapezoidal cross-section of the conical frustum.
        //
        // The circumcenter is at the intersection of the bisectors perpendicular to the sides.
        // For the isosceles trapezoid, the X coordinate is zero at the center, so a single bisector is enough.
        //
        //       A
        //       *-------*
        //      /    |    \
        //     /     |     \
        // AB / \    |    / \
        //   /     \ | /     \
        //  /        C        \
        // *-------------------*
        // B

        let a = Vec2::new(-self.radius_top, half_height);
        let b = Vec2::new(-self.radius_bottom, -half_height);
        let ab = a - b;
        let ab_midpoint = b + 0.5 * ab;

        // The direction towards the circumcenter is the bisector perpendicular to the side
        let bisector = -ab.perp().normalize_or_zero();

        // Here we divide the bisector by its X coordinate so that its X becomes 1
        // and we can reach the center by multiplying by the X distance to the midpoint of AB.
        let circumcenter = ab_midpoint + ab_midpoint.x.abs() * bisector / bisector.x.abs();

        // The circumcircle passes through all four vertices, so we can pick any of them
        let circumradius = a.distance(circumcenter);

        BoundingSphere::new(
            translation + rotation * circumcenter.extend(0.0),
            circumradius,
        )
    }
}

impl Bounded3d for Torus {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Compute the AABB of a flat disc with the major radius of the torus.
        // Reference: http://iquilezles.org/articles/diskbbox/
        let normal = rotation * Vec3::Y;
        let e = 1.0 - normal * normal;
        let disc_half_extents = self.major_radius * Vec3::new(e.x.sqrt(), e.y.sqrt(), e.z.sqrt());

        // Expand the disc by the minor radius to get the torus half extents
        let half_extents = disc_half_extents + Vec3::splat(self.minor_radius);

        Aabb3d {
            min: translation - half_extents,
            max: translation + half_extents,
        }
    }

    fn bounding_sphere(&self, translation: Vec3, _rotation: Quat) -> BoundingSphere {
        BoundingSphere::new(translation, self.outer_radius())
    }
}
