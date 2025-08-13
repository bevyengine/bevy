use core::f32::consts::FRAC_PI_2;

use glam::{Vec2, Vec3A, Vec3Swizzles};

use crate::{
    bounding::{BoundingCircle, BoundingVolume},
    ops,
    primitives::{
        Capsule2d, Cuboid, Cylinder, Ellipse, Extrusion, Line2d, Primitive2d, Rectangle,
        RegularPolygon, Segment2d, Triangle2d,
    },
    Isometry2d, Isometry3d, Quat, Rot2,
};

#[cfg(feature = "alloc")]
use crate::primitives::{Polygon, Polyline2d};

use crate::{bounding::Bounded2d, primitives::Circle};

use super::{Aabb3d, Bounded3d, BoundingSphere};

impl BoundedExtrusion for Circle {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let isometry = isometry.into();

        let segment_dir = isometry.rotation * Vec3A::Z;
        let top = (segment_dir * half_depth).abs();

        let e = (Vec3A::ONE - segment_dir * segment_dir).max(Vec3A::ZERO);
        let half_size = self.radius * Vec3A::new(ops::sqrt(e.x), ops::sqrt(e.y), ops::sqrt(e.z));

        Aabb3d {
            min: isometry.translation - half_size - top,
            max: isometry.translation + half_size + top,
        }
    }
}

impl BoundedExtrusion for Ellipse {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let Vec2 { x: a, y: b } = self.half_size;
        let normal = isometry.rotation * Vec3A::Z;
        let conjugate_rot = isometry.rotation.conjugate();

        let [max_x, max_y, max_z] = Vec3A::AXES.map(|axis| {
            let Some(axis) = (conjugate_rot * axis.reject_from(normal))
                .xy()
                .try_normalize()
            else {
                return Vec3A::ZERO;
            };

            if axis.element_product() == 0. {
                return isometry.rotation * Vec3A::new(a * axis.y, b * axis.x, 0.);
            }
            let m = -axis.x / axis.y;
            let signum = axis.signum();

            let y = signum.y * b * b / ops::sqrt(b * b + m * m * a * a);
            let x = signum.x * a * ops::sqrt(1. - y * y / b / b);
            isometry.rotation * Vec3A::new(x, y, 0.)
        });

        let half_size = Vec3A::new(max_x.x, max_y.y, max_z.z).abs() + (normal * half_depth).abs();
        Aabb3d::new(isometry.translation, half_size)
    }
}

impl BoundedExtrusion for Line2d {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let dir = isometry.rotation * Vec3A::from(self.direction.extend(0.));
        let half_depth = (isometry.rotation * Vec3A::new(0., 0., half_depth)).abs();

        let max = f32::MAX / 2.;
        let half_size = Vec3A::new(
            if dir.x == 0. { half_depth.x } else { max },
            if dir.y == 0. { half_depth.y } else { max },
            if dir.z == 0. { half_depth.z } else { max },
        );

        Aabb3d::new(isometry.translation, half_size)
    }
}

impl BoundedExtrusion for Segment2d {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let half_size = isometry.rotation * Vec3A::from(self.point1().extend(0.));
        let depth = isometry.rotation * Vec3A::new(0., 0., half_depth);

        Aabb3d::new(isometry.translation, half_size.abs() + depth.abs())
    }
}

#[cfg(feature = "alloc")]
impl BoundedExtrusion for Polyline2d {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let aabb = Aabb3d::from_point_cloud(isometry, self.vertices.iter().map(|v| v.extend(0.)));
        let depth = isometry.rotation * Vec3A::new(0., 0., half_depth);

        aabb.grow(depth.abs())
    }
}

impl BoundedExtrusion for Triangle2d {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let aabb = Aabb3d::from_point_cloud(isometry, self.vertices.iter().map(|v| v.extend(0.)));
        let depth = isometry.rotation * Vec3A::new(0., 0., half_depth);

        aabb.grow(depth.abs())
    }
}

impl BoundedExtrusion for Rectangle {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        Cuboid {
            half_size: self.half_size.extend(half_depth),
        }
        .aabb_3d(isometry)
    }
}

impl BoundedExtrusion for Polygon {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let aabb = Aabb3d::from_point_cloud(isometry, self.vertices.iter().map(|v| v.extend(0.)));
        let depth = isometry.rotation * Vec3A::new(0., 0., half_depth);

        aabb.grow(depth.abs())
    }
}

impl BoundedExtrusion for RegularPolygon {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let aabb = Aabb3d::from_point_cloud(
            isometry,
            self.vertices(0.).into_iter().map(|v| v.extend(0.)),
        );
        let depth = isometry.rotation * Vec3A::new(0., 0., half_depth);

        aabb.grow(depth.abs())
    }
}

impl BoundedExtrusion for Capsule2d {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let aabb = Cylinder {
            half_height: half_depth,
            radius: self.radius,
        }
        .aabb_3d(isometry.rotation * Quat::from_rotation_x(FRAC_PI_2));

        let up = isometry.rotation * Vec3A::new(0., self.half_length, 0.);
        let half_size = aabb.max + up.abs();
        Aabb3d::new(isometry.translation, half_size)
    }
}

impl<T: BoundedExtrusion> Bounded3d for Extrusion<T> {
    fn aabb_3d(&self, isometry: impl Into<Isometry3d>) -> Aabb3d {
        self.base_shape.extrusion_aabb_3d(self.half_depth, isometry)
    }

    fn bounding_sphere(&self, isometry: impl Into<Isometry3d>) -> BoundingSphere {
        self.base_shape
            .extrusion_bounding_sphere(self.half_depth, isometry)
    }
}

/// A trait implemented on 2D shapes which determines the 3D bounding volumes of their extrusions.
///
/// Since default implementations can be inferred from 2D bounding volumes, this allows a `Bounded2d`
/// implementation on some shape `MyShape` to be extrapolated to a `Bounded3d` implementation on
/// `Extrusion<MyShape>` without supplying any additional data; e.g.:
/// `impl BoundedExtrusion for MyShape {}`
pub trait BoundedExtrusion: Primitive2d + Bounded2d {
    /// Get an axis-aligned bounding box for an extrusion with this shape as a base and the given `half_depth`, transformed by the given `translation` and `rotation`.
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        let isometry = isometry.into();
        let cap_normal = isometry.rotation * Vec3A::Z;
        let conjugate_rot = isometry.rotation.conjugate();

        // The `(halfsize, offset)` for each axis
        let axis_values = Vec3A::AXES.map(|ax| {
            // This is the direction of the line of intersection of a plane with the `ax` normal and the plane containing the cap of the extrusion.
            let intersect_line = ax.cross(cap_normal);
            if intersect_line.length_squared() <= f32::EPSILON {
                return (0., 0.);
            };

            // This is the normal vector of the intersection line rotated to be in the XY-plane
            let line_normal = (conjugate_rot * intersect_line).yx();
            let angle = line_normal.to_angle();

            // Since the plane containing the caps of the extrusion is not guaranteed to be orthogonal to the `ax` plane, only a certain "scale" factor
            // of the `Aabb2d` will actually go towards the dimensions of the `Aabb3d`
            let scale = cap_normal.reject_from(ax).length();

            // Calculate the `Aabb2d` of the base shape. The shape is rotated so that the line of intersection is parallel to the Y axis in the `Aabb2d` calculations.
            // This guarantees that the X value of the `Aabb2d` is closest to the `ax` plane
            let aabb2d = self.aabb_2d(Rot2::radians(angle));
            (aabb2d.half_size().x * scale, aabb2d.center().x * scale)
        });

        let offset = Vec3A::from_array(axis_values.map(|(_, offset)| offset));
        let cap_size = Vec3A::from_array(axis_values.map(|(max_val, _)| max_val)).abs();
        let depth = isometry.rotation * Vec3A::new(0., 0., half_depth);

        Aabb3d::new(isometry.translation - offset, cap_size + depth.abs())
    }

    /// Get a bounding sphere for an extrusion of the `base_shape` with the given `half_depth` with the given translation and rotation
    fn extrusion_bounding_sphere(
        &self,
        half_depth: f32,
        isometry: impl Into<Isometry3d>,
    ) -> BoundingSphere {
        let isometry = isometry.into();

        // We calculate the bounding circle of the base shape.
        // Since each of the extrusions bases will have the same distance from its center,
        // and they are just shifted along the Z-axis, the minimum bounding sphere will be the bounding sphere
        // of the cylinder defined by the two bounding circles of the bases for any base shape
        let BoundingCircle {
            center,
            circle: Circle { radius },
        } = self.bounding_circle(Isometry2d::IDENTITY);
        let radius = ops::hypot(radius, half_depth);
        let center = isometry * Vec3A::from(center.extend(0.));

        BoundingSphere::new(center, radius)
    }
}

#[cfg(test)]
mod tests {
    use core::f32::consts::FRAC_PI_4;

    use glam::{EulerRot, Quat, Vec2, Vec3, Vec3A};

    use crate::{
        bounding::{Bounded3d, BoundingVolume},
        ops,
        primitives::{
            Capsule2d, Circle, Ellipse, Extrusion, Line2d, Polygon, Polyline2d, Rectangle,
            RegularPolygon, Segment2d, Triangle2d,
        },
        Dir2, Isometry3d,
    };

    #[test]
    fn circle() {
        let cylinder = Extrusion::new(Circle::new(0.5), 2.0);
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = cylinder.aabb_3d(translation);
        assert_eq!(aabb.center(), Vec3A::from(translation));
        assert_eq!(aabb.half_size(), Vec3A::new(0.5, 0.5, 1.0));

        let bounding_sphere = cylinder.bounding_sphere(translation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), ops::hypot(1.0, 0.5));
    }

    #[test]
    fn ellipse() {
        let extrusion = Extrusion::new(Ellipse::new(2.0, 0.5), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_euler(EulerRot::ZYX, FRAC_PI_4, FRAC_PI_4, FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), Vec3A::from(translation));
        assert_eq!(aabb.half_size(), Vec3A::new(2.709784, 1.3801551, 2.436141));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), ops::sqrt(8f32));
    }

    #[test]
    fn line() {
        let extrusion = Extrusion::new(
            Line2d {
                direction: Dir2::new_unchecked(Vec2::Y),
            },
            4.,
        );
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_y(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.min, Vec3A::new(1.5857864, f32::MIN / 2., 3.5857865));
        assert_eq!(aabb.max, Vec3A::new(4.4142136, f32::MAX / 2., 6.414213));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center(), translation.into());
        assert_eq!(bounding_sphere.radius(), f32::MAX / 2.);
    }

    #[test]
    fn rectangle() {
        let extrusion = Extrusion::new(Rectangle::new(2.0, 1.0), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_z(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(1.0606602, 1.0606602, 2.));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.291288);
    }

    #[test]
    fn segment() {
        let extrusion = Extrusion::new(
            Segment2d::new(Vec2::new(0.0, -1.5), Vec2::new(0.0, 1.5)),
            4.0,
        );
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(0., 2.4748735, 2.4748735));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.5);
    }

    #[test]
    fn polyline() {
        let polyline = Polyline2d::new([
            Vec2::ONE,
            Vec2::new(-1.0, 1.0),
            Vec2::NEG_ONE,
            Vec2::new(1.0, -1.0),
        ]);
        let extrusion = Extrusion::new(polyline, 3.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(1., 1.7677668, 1.7677668));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.0615528);
    }

    #[test]
    fn triangle() {
        let triangle = Triangle2d::new(
            Vec2::new(0.0, 1.0),
            Vec2::new(-10.0, -1.0),
            Vec2::new(10.0, -1.0),
        );
        let extrusion = Extrusion::new(triangle, 3.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(10., 1.7677668, 1.7677668));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(
            bounding_sphere.center,
            Vec3A::new(3.0, 3.2928934, 4.2928934)
        );
        assert_eq!(bounding_sphere.radius(), 10.111875);
    }

    #[test]
    fn polygon() {
        let polygon = Polygon::new([
            Vec2::ONE,
            Vec2::new(-1.0, 1.0),
            Vec2::NEG_ONE,
            Vec2::new(1.0, -1.0),
        ]);
        let extrusion = Extrusion::new(polygon, 3.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(1., 1.7677668, 1.7677668));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.0615528);
    }

    #[test]
    fn regular_polygon() {
        let extrusion = Extrusion::new(RegularPolygon::new(2.0, 7), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(
            aabb.center(),
            Vec3A::from(translation) + Vec3A::new(0., 0.0700254, 0.0700254)
        );
        assert_eq!(
            aabb.half_size(),
            Vec3A::new(1.9498558, 2.7584014, 2.7584019)
        );

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), ops::sqrt(8f32));
    }

    #[test]
    fn capsule() {
        let extrusion = Extrusion::new(Capsule2d::new(0.5, 2.0), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);
        let isometry = Isometry3d::new(translation, rotation);

        let aabb = extrusion.aabb_3d(isometry);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(0.5, 2.4748735, 2.4748735));

        let bounding_sphere = extrusion.bounding_sphere(isometry);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.5);
    }
}
