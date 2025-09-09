//! Contains [`Bounded3d`] implementations for [geometric primitives](crate::primitives).

use crate::{
    bounding::{Bounded2d, BoundingCircle, BoundingVolume},
    ops,
    primitives::{
        Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, InfinitePlane3d, Line3d, Segment3d,
        Sphere, Torus, Triangle2d, Triangle3d,
    },
    Isometry2d, Isometry3d, Mat3, Vec2, Vec3, Vec3A,
};

#[cfg(feature = "alloc")]
use crate::primitives::Polyline3d;

use super::{Aabb3d, Bounded3d, BoundingSphere};

impl Bounded3d for Sphere {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        Aabb3d::new(isometry.translation, Vec3::splat(self.radius))
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        BoundingSphere::new(isometry.translation, self.radius)
    }
}

impl Bounded3d for InfinitePlane3d {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();

        let normal = isometry.rotation * *self.normal;
        let facing_x = normal == Vec3::X || normal == Vec3::NEG_X;
        let facing_y = normal == Vec3::Y || normal == Vec3::NEG_Y;
        let facing_z = normal == Vec3::Z || normal == Vec3::NEG_Z;

        // Dividing `f32::MAX` by 2.0 is helpful so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let half_width = if facing_x { 0.0 } else { f32::MAX / 2.0 };
        let half_height = if facing_y { 0.0 } else { f32::MAX / 2.0 };
        let half_depth = if facing_z { 0.0 } else { f32::MAX / 2.0 };
        let half_size = Vec3A::new(half_width, half_height, half_depth);

        Aabb3d::new(isometry.translation, half_size)
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        BoundingSphere::new(isometry.translation, f32::MAX / 2.0)
    }
}

impl Bounded3d for Line3d {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let direction = isometry.rotation * *self.direction;

        // Dividing `f32::MAX` by 2.0 is helpful so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let max = f32::MAX / 2.0;
        let half_width = if direction.x == 0.0 { 0.0 } else { max };
        let half_height = if direction.y == 0.0 { 0.0 } else { max };
        let half_depth = if direction.z == 0.0 { 0.0 } else { max };
        let half_size = Vec3A::new(half_width, half_height, half_depth);

        Aabb3d::new(isometry.translation, half_size)
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        BoundingSphere::new(isometry.translation, f32::MAX / 2.0)
    }
}

impl Bounded3d for Segment3d {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        Aabb3d::from_point_cloud(isometry, [self.point1(), self.point2()].iter().copied())
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        let local_sphere = BoundingSphere::new(self.center(), self.length() / 2.);
        local_sphere.transformed_by(isometry.translation, isometry.rotation)
    }
}

#[cfg(feature = "alloc")]
impl Bounded3d for Polyline3d {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        Aabb3d::from_point_cloud(isometry, self.vertices.iter().copied())
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        BoundingSphere::from_point_cloud(isometry, &self.vertices)
    }
}

impl Bounded3d for Cuboid {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();

        // Compute the AABB of the rotated cuboid by transforming the half-size
        // by an absolute rotation matrix.
        let rot_mat = Mat3::from_quat(isometry.rotation);
        let abs_rot_mat = Mat3::from_cols(
            rot_mat.x_axis.abs(),
            rot_mat.y_axis.abs(),
            rot_mat.z_axis.abs(),
        );
        let half_size = abs_rot_mat * self.half_size;

        Aabb3d::new(isometry.translation, half_size)
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        BoundingSphere::new(isometry.translation, self.half_size.length())
    }
}

impl Bounded3d for Cylinder {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let isometry = isometry.into();

        let segment_dir = isometry.rotation * Vec3A::Y;
        let top = segment_dir * self.half_height;
        let bottom = -top;

        let e = (Vec3A::ONE - segment_dir * segment_dir).max(Vec3A::ZERO);
        let half_size = self.radius * Vec3A::new(ops::sqrt(e.x), ops::sqrt(e.y), ops::sqrt(e.z));

        Aabb3d {
            min: isometry.translation + (top - half_size).min(bottom - half_size),
            max: isometry.translation + (top + half_size).max(bottom + half_size),
        }
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        let radius = ops::hypot(self.radius, self.half_height);
        BoundingSphere::new(isometry.translation, radius)
    }
}

impl Bounded3d for Capsule3d {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();

        // Get the line segment between the hemispheres of the rotated capsule
        let segment_dir = isometry.rotation * Vec3A::Y;
        let top = segment_dir * self.half_length;
        let bottom = -top;

        // Expand the line segment by the capsule radius to get the capsule half-extents
        let min = bottom.min(top) - Vec3A::splat(self.radius);
        let max = bottom.max(top) + Vec3A::splat(self.radius);

        Aabb3d {
            min: min + isometry.translation,
            max: max + isometry.translation,
        }
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        BoundingSphere::new(isometry.translation, self.radius + self.half_length)
    }
}

impl Bounded3d for Cone {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let isometry = isometry.into();

        let segment_dir = isometry.rotation * Vec3A::Y;
        let top = segment_dir * 0.5 * self.height;
        let bottom = -top;

        let e = (Vec3A::ONE - segment_dir * segment_dir).max(Vec3A::ZERO);
        let half_extents = Vec3A::new(ops::sqrt(e.x), ops::sqrt(e.y), ops::sqrt(e.z));

        Aabb3d {
            min: isometry.translation + top.min(bottom - self.radius * half_extents),
            max: isometry.translation + top.max(bottom + self.radius * half_extents),
        }
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();

        // Get the triangular cross-section of the cone.
        let half_height = 0.5 * self.height;
        let triangle = Triangle2d::new(
            half_height * Vec2::Y,
            Vec2::new(-self.radius, -half_height),
            Vec2::new(self.radius, -half_height),
        );

        // Because of circular symmetry, we can use the bounding circle of the triangle
        // for the bounding sphere of the cone.
        let BoundingCircle { circle, center } = triangle.bounding_circle(Isometry2d::IDENTITY);

        BoundingSphere::new(
            isometry.rotation * Vec3A::from(center.extend(0.0)) + isometry.translation,
            circle.radius,
        )
    }
}

impl Bounded3d for ConicalFrustum {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let isometry = isometry.into();

        let segment_dir = isometry.rotation * Vec3A::Y;
        let top = segment_dir * 0.5 * self.height;
        let bottom = -top;

        let e = (Vec3A::ONE - segment_dir * segment_dir).max(Vec3A::ZERO);
        let half_extents = Vec3A::new(ops::sqrt(e.x), ops::sqrt(e.y), ops::sqrt(e.z));

        Aabb3d {
            min: isometry.translation
                + (top - self.radius_top * half_extents)
                    .min(bottom - self.radius_bottom * half_extents),
            max: isometry.translation
                + (top + self.radius_top * half_extents)
                    .max(bottom + self.radius_bottom * half_extents),
        }
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        let half_height = 0.5 * self.height;

        // To compute the bounding sphere, we'll get the center and radius of the circumcircle
        // passing through all four vertices of the trapezoidal cross-section of the conical frustum.
        //
        // If the circumcenter is inside the trapezoid, we can use that for the bounding sphere.
        // Otherwise, we clamp it to the longer parallel side to get a more tightly fitting bounding sphere.
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
        let bisector = ab.perp();

        // Compute intersection between bisector and vertical line at x = 0.
        //
        // x = ab_midpoint.x + t * bisector.x = 0
        // y = ab_midpoint.y + t * bisector.y = ?
        //
        // Because ab_midpoint.y = 0 for our conical frustum, we get:
        // y = t * bisector.y
        //
        // Solve x for t:
        // t = -ab_midpoint.x / bisector.x
        //
        // Substitute t to solve for y:
        // y = -ab_midpoint.x / bisector.x * bisector.y
        let circumcenter_y = -ab_midpoint.x / bisector.x * bisector.y;

        // If the circumcenter is outside the trapezoid, the bounding circle is too large.
        // In those cases, we clamp it to the longer parallel side.
        let (center, radius) = if circumcenter_y <= -half_height {
            (Vec2::new(0.0, -half_height), self.radius_bottom)
        } else if circumcenter_y >= half_height {
            (Vec2::new(0.0, half_height), self.radius_top)
        } else {
            let circumcenter = Vec2::new(0.0, circumcenter_y);
            // We can use the distance from an arbitrary vertex because they all lie on the circumcircle.
            (circumcenter, a.distance(circumcenter))
        };

        BoundingSphere::new(
            isometry.translation + isometry.rotation * Vec3A::from(center.extend(0.0)),
            radius,
        )
    }
}

impl Bounded3d for Torus {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();

        // Compute the AABB of a flat disc with the major radius of the torus.
        // Reference: http://iquilezles.org/articles/diskbbox/
        let normal = isometry.rotation * Vec3A::Y;
        let e = (Vec3A::ONE - normal * normal).max(Vec3A::ZERO);
        let disc_half_size =
            self.major_radius * Vec3A::new(ops::sqrt(e.x), ops::sqrt(e.y), ops::sqrt(e.z));

        // Expand the disc by the minor radius to get the torus half-size
        let half_size = disc_half_size + Vec3A::splat(self.minor_radius);

        Aabb3d::new(isometry.translation, half_size)
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();
        BoundingSphere::new(isometry.translation, self.outer_radius())
    }
}

impl Bounded3d for Triangle3d {
    /// Get the bounding box of the triangle.
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let [a, b, c] = self.vertices;

        let a = isometry.rotation * a;
        let b = isometry.rotation * b;
        let c = isometry.rotation * c;

        let min = Vec3A::from(a.min(b).min(c));
        let max = Vec3A::from(a.max(b).max(c));

        let bounding_center = (max + min) / 2.0 + isometry.translation;
        let half_extents = (max - min) / 2.0;

        Aabb3d::new(bounding_center, half_extents)
    }

    /// Get the bounding sphere of the triangle.
    ///
    /// The [`Triangle3d`] implements the minimal bounding sphere calculation. For acute triangles, the circumcenter is used as
    /// the center of the sphere. For the others, the bounding sphere is the minimal sphere
    /// that contains the largest side of the triangle.
    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        let isometry = isometry.into();

        if self.is_degenerate() || self.is_obtuse() {
            let (p1, p2) = self.largest_side();
            let (p1, p2) = (Vec3A::from(p1), Vec3A::from(p2));
            let mid_point = (p1 + p2) / 2.0;
            let radius = mid_point.distance(p1);
            BoundingSphere::new(mid_point + isometry.translation, radius)
        } else {
            let [a, _, _] = self.vertices;

            let circumcenter = self.circumcenter();
            let radius = circumcenter.distance(a);
            BoundingSphere::new(Vec3A::from(circumcenter) + isometry.translation, radius)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{bounding::BoundingVolume, ops, Isometry3d};
    use glam::{Quat, Vec3, Vec3A};

    use crate::{
        bounding::Bounded3d,
        primitives::{
            Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, InfinitePlane3d, Line3d, Polyline3d,
            Segment3d, Sphere, Torus, Triangle3d,
        },
        Dir3,
    };

    #[test]
    fn sphere() {
        let sphere = Sphere { radius: 1.0 };
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = sphere.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(1.0, 0.0, -1.0));
        assert_eq!(aabb.max, Vec3A::new(3.0, 2.0, 1.0));

        let bounding_sphere = sphere.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 1.0);
    }

    #[test]
    fn plane() {
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb1 = InfinitePlane3d::new(Vec3::X).aabb_3d(translation);
        assert_eq!(aabb1.min, Vec3A::new(2.0, -f32::MAX / 2.0, -f32::MAX / 2.0));
        assert_eq!(aabb1.max, Vec3A::new(2.0, f32::MAX / 2.0, f32::MAX / 2.0));

        let aabb2 = InfinitePlane3d::new(Vec3::Y).aabb_3d(translation);
        assert_eq!(aabb2.min, Vec3A::new(-f32::MAX / 2.0, 1.0, -f32::MAX / 2.0));
        assert_eq!(aabb2.max, Vec3A::new(f32::MAX / 2.0, 1.0, f32::MAX / 2.0));

        let aabb3 = InfinitePlane3d::new(Vec3::Z).aabb_3d(translation);
        assert_eq!(aabb3.min, Vec3A::new(-f32::MAX / 2.0, -f32::MAX / 2.0, 0.0));
        assert_eq!(aabb3.max, Vec3A::new(f32::MAX / 2.0, f32::MAX / 2.0, 0.0));

        let aabb4 = InfinitePlane3d::new(Vec3::ONE).aabb_3d(translation);
        assert_eq!(aabb4.min, Vec3A::splat(-f32::MAX / 2.0));
        assert_eq!(aabb4.max, Vec3A::splat(f32::MAX / 2.0));

        let bounding_sphere = InfinitePlane3d::new(Vec3::Y).bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), f32::MAX / 2.0);
    }

    #[test]
    fn line() {
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb1 = Line3d { direction: Dir3::Y }.aabb_3d(translation);
        assert_eq!(aabb1.min, Vec3A::new(2.0, -f32::MAX / 2.0, 0.0));
        assert_eq!(aabb1.max, Vec3A::new(2.0, f32::MAX / 2.0, 0.0));

        let aabb2 = Line3d { direction: Dir3::X }.aabb_3d(translation);
        assert_eq!(aabb2.min, Vec3A::new(-f32::MAX / 2.0, 1.0, 0.0));
        assert_eq!(aabb2.max, Vec3A::new(f32::MAX / 2.0, 1.0, 0.0));

        let aabb3 = Line3d { direction: Dir3::Z }.aabb_3d(translation);
        assert_eq!(aabb3.min, Vec3A::new(2.0, 1.0, -f32::MAX / 2.0));
        assert_eq!(aabb3.max, Vec3A::new(2.0, 1.0, f32::MAX / 2.0));

        let aabb4 = Line3d {
            direction: Dir3::from_xyz(1.0, 1.0, 1.0).unwrap(),
        }
        .aabb_3d(translation);
        assert_eq!(aabb4.min, Vec3A::splat(-f32::MAX / 2.0));
        assert_eq!(aabb4.max, Vec3A::splat(f32::MAX / 2.0));

        let bounding_sphere = Line3d { direction: Dir3::Y }.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), f32::MAX / 2.0);
    }

    #[test]
    fn segment() {
        let segment = Segment3d::new(Vec3::new(-1.0, -0.5, 0.0), Vec3::new(1.0, 0.5, 0.0));
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = segment.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(1.0, 0.5, 0.0));
        assert_eq!(aabb.max, Vec3A::new(3.0, 1.5, 0.0));

        let bounding_sphere = segment.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), ops::hypot(1.0, 0.5));
    }

    #[test]
    fn polyline() {
        let polyline = Polyline3d::new([
            Vec3::ONE,
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::NEG_ONE,
            Vec3::new(1.0, -1.0, -1.0),
        ]);
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = polyline.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(1.0, 0.0, -1.0));
        assert_eq!(aabb.max, Vec3A::new(3.0, 2.0, 1.0));

        let bounding_sphere = polyline.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(
            bounding_sphere.radius(),
            ops::hypot(ops::hypot(1.0, 1.0), 1.0)
        );
    }

    #[test]
    fn cuboid() {
        let cuboid = Cuboid::new(2.0, 1.0, 1.0);
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = cuboid.aabb_3d(Isometry3d::new(
            translation,
            Quat::from_rotation_z(core::f32::consts::FRAC_PI_4),
        ));
        let expected_half_size = Vec3A::new(1.0606601, 1.0606601, 0.5);
        assert_eq!(aabb.min, Vec3A::from(translation) - expected_half_size);
        assert_eq!(aabb.max, Vec3A::from(translation) + expected_half_size);

        let bounding_sphere = cuboid.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(
            bounding_sphere.radius(),
            ops::hypot(ops::hypot(1.0, 0.5), 0.5)
        );
    }

    #[test]
    fn cylinder() {
        let cylinder = Cylinder::new(0.5, 2.0);
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = cylinder.aabb_3d(translation);
        assert_eq!(
            aabb.min,
            Vec3A::from(translation) - Vec3A::new(0.5, 1.0, 0.5)
        );
        assert_eq!(
            aabb.max,
            Vec3A::from(translation) + Vec3A::new(0.5, 1.0, 0.5)
        );

        let bounding_sphere = cylinder.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), ops::hypot(1.0, 0.5));
    }

    #[test]
    fn capsule() {
        let capsule = Capsule3d::new(0.5, 2.0);
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = capsule.aabb_3d(translation);
        assert_eq!(
            aabb.min,
            Vec3A::from(translation) - Vec3A::new(0.5, 1.5, 0.5)
        );
        assert_eq!(
            aabb.max,
            Vec3A::from(translation) + Vec3A::new(0.5, 1.5, 0.5)
        );

        let bounding_sphere = capsule.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 1.5);
    }

    #[test]
    fn cone() {
        let cone = Cone {
            radius: 1.0,
            height: 2.0,
        };
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = cone.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(1.0, 0.0, -1.0));
        assert_eq!(aabb.max, Vec3A::new(3.0, 2.0, 1.0));

        let bounding_sphere = cone.bounding_sphere(translation);
        assert_eq!(
            bounding_sphere.center,
            Vec3A::from(translation) + Vec3A::NEG_Y * 0.25
        );
        assert_eq!(bounding_sphere.radius(), 1.25);
    }

    #[test]
    fn conical_frustum() {
        let conical_frustum = ConicalFrustum {
            radius_top: 0.5,
            radius_bottom: 1.0,
            height: 2.0,
        };
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = conical_frustum.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(1.0, 0.0, -1.0));
        assert_eq!(aabb.max, Vec3A::new(3.0, 2.0, 1.0));

        let bounding_sphere = conical_frustum.bounding_sphere(translation);
        assert_eq!(
            bounding_sphere.center,
            Vec3A::from(translation) + Vec3A::NEG_Y * 0.1875
        );
        assert_eq!(bounding_sphere.radius(), 1.2884705);
    }

    #[test]
    fn wide_conical_frustum() {
        let conical_frustum = ConicalFrustum {
            radius_top: 0.5,
            radius_bottom: 5.0,
            height: 1.0,
        };
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = conical_frustum.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(-3.0, 0.5, -5.0));
        assert_eq!(aabb.max, Vec3A::new(7.0, 1.5, 5.0));

        // For wide conical frusta like this, the circumcenter can be outside the frustum,
        // so the center and radius should be clamped to the longest side.
        let bounding_sphere = conical_frustum.bounding_sphere(translation);
        assert_eq!(
            bounding_sphere.center,
            Vec3A::from(translation) + Vec3A::NEG_Y * 0.5
        );
        assert_eq!(bounding_sphere.radius(), 5.0);
    }

    #[test]
    fn torus() {
        let torus = Torus {
            minor_radius: 0.5,
            major_radius: 1.0,
        };
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = torus.aabb_3d(translation);
        assert_eq!(aabb.min, Vec3A::new(0.5, 0.5, -1.5));
        assert_eq!(aabb.max, Vec3A::new(3.5, 1.5, 1.5));

        let bounding_sphere = torus.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 1.5);
    }

    #[test]
    fn triangle3d() {
        let zero_degenerate_triangle = Triangle3d::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);

        let br = zero_degenerate_triangle.aabb_3d(Isometry3d::IDENTITY);
        assert_eq!(
            br.center(),
            Vec3::ZERO.into(),
            "incorrect bounding box center"
        );
        assert_eq!(
            br.half_size(),
            Vec3::ZERO.into(),
            "incorrect bounding box half extents"
        );

        let bs = zero_degenerate_triangle.bounding_sphere(Isometry3d::IDENTITY);
        assert_eq!(
            bs.center,
            Vec3::ZERO.into(),
            "incorrect bounding sphere center"
        );
        assert_eq!(bs.sphere.radius, 0.0, "incorrect bounding sphere radius");

        let dup_degenerate_triangle = Triangle3d::new(Vec3::ZERO, Vec3::X, Vec3::X);
        let bs = dup_degenerate_triangle.bounding_sphere(Isometry3d::IDENTITY);
        assert_eq!(
            bs.center,
            Vec3::new(0.5, 0.0, 0.0).into(),
            "incorrect bounding sphere center"
        );
        assert_eq!(bs.sphere.radius, 0.5, "incorrect bounding sphere radius");
        let br = dup_degenerate_triangle.aabb_3d(Isometry3d::IDENTITY);
        assert_eq!(
            br.center(),
            Vec3::new(0.5, 0.0, 0.0).into(),
            "incorrect bounding box center"
        );
        assert_eq!(
            br.half_size(),
            Vec3::new(0.5, 0.0, 0.0).into(),
            "incorrect bounding box half extents"
        );

        let collinear_degenerate_triangle = Triangle3d::new(Vec3::NEG_X, Vec3::ZERO, Vec3::X);
        let bs = collinear_degenerate_triangle.bounding_sphere(Isometry3d::IDENTITY);
        assert_eq!(
            bs.center,
            Vec3::ZERO.into(),
            "incorrect bounding sphere center"
        );
        assert_eq!(bs.sphere.radius, 1.0, "incorrect bounding sphere radius");
        let br = collinear_degenerate_triangle.aabb_3d(Isometry3d::IDENTITY);
        assert_eq!(
            br.center(),
            Vec3::ZERO.into(),
            "incorrect bounding box center"
        );
        assert_eq!(
            br.half_size(),
            Vec3::new(1.0, 0.0, 0.0).into(),
            "incorrect bounding box half extents"
        );
    }
}
