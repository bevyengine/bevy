use std::f32::consts::FRAC_PI_2;

use glam::{Vec2, Vec3A, Vec3Swizzles};

use crate::bounding::{BoundingCircle, BoundingVolume};
use crate::primitives::{
    BoxedPolygon, BoxedPolyline2d, Capsule2d, Cuboid, Cylinder, Ellipse, Extrusion, Polygon,
    Polyline2d, Primitive2d, Rectangle, RegularPolygon, Segment2d, Triangle2d,
};
use crate::{Quat, Vec3};

use crate::{bounding::Bounded2d, primitives::Circle};

use super::{Aabb3d, Bounded3d, BoundingSphere};

impl Bounded3d for Extrusion<Circle> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        // Reference: http://iquilezles.org/articles/diskbbox/

        let segment_dir = rotation * Vec3::Z;
        let top = segment_dir * self.half_depth;
        let bottom = -top;

        let e = Vec3::ONE - segment_dir * segment_dir;
        let half_size = self.base_shape.radius * Vec3::new(e.x.sqrt(), e.y.sqrt(), e.z.sqrt());

        Aabb3d {
            min: (translation + (top - half_size).min(bottom - half_size)).into(),
            max: (translation + (top + half_size).max(bottom + half_size)).into(),
        }
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<Ellipse> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let Vec2 { x: a, y: b } = self.base_shape.half_size;
        let normal = rotation * Vec3::Z;
        let conjugate_rot = rotation.conjugate();

        let [max_x, max_y, max_z] = Vec3::AXES.map(|axis: Vec3| {
            let Some(axis) = (conjugate_rot * axis.reject_from(normal))
                .xy()
                .try_normalize()
            else {
                return Vec3::ZERO;
            };

            if axis.element_product() == 0. {
                return rotation * Vec3::new(a * axis.y, b * axis.x, 0.);
            }
            let m = -axis.x / axis.y;
            let signum = axis.signum();

            let y = signum.y * b * b / (b * b + m * m * a * a).sqrt();
            let x = signum.x * a * (1. - y * y / b / b).sqrt();
            rotation * Vec3::new(x, y, 0.)
        });

        let half_size =
            Vec3::new(max_x.x, max_y.y, max_z.z).abs() + (normal * self.half_depth).abs();
        Aabb3d::new(translation, half_size)
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<Segment2d> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let half_size = rotation * self.base_shape.point1().extend(0.);
        let depth = rotation * Vec3::new(0., 0., self.half_depth);

        Aabb3d::new(translation, half_size.abs() + depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl<const N: usize> Bounded3d for Extrusion<Polyline2d<N>> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Aabb3d::from_point_cloud(
            translation,
            rotation,
            self.base_shape.vertices.map(|v| v.extend(0.)).into_iter(),
        );
        let depth = rotation * Vec3A::new(0., 0., self.half_depth);

        aabb.grow(depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<BoxedPolyline2d> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Aabb3d::from_point_cloud(
            translation,
            rotation,
            self.base_shape.vertices.iter().map(|v| v.extend(0.)),
        );
        let depth = rotation * Vec3A::new(0., 0., self.half_depth);

        aabb.grow(depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<Triangle2d> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Aabb3d::from_point_cloud(
            translation,
            rotation,
            self.base_shape.vertices.iter().map(|v| v.extend(0.)),
        );
        let depth = rotation * Vec3A::new(0., 0., self.half_depth);

        aabb.grow(depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<Rectangle> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        Cuboid {
            half_size: self.base_shape.half_size.extend(self.half_depth),
        }
        .aabb_3d(translation, rotation)
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl<const N: usize> Bounded3d for Extrusion<Polygon<N>> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Aabb3d::from_point_cloud(
            translation,
            rotation,
            self.base_shape.vertices.map(|v| v.extend(0.)).into_iter(),
        );
        let depth = rotation * Vec3A::new(0., 0., self.half_depth);

        aabb.grow(depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<BoxedPolygon> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Aabb3d::from_point_cloud(
            translation,
            rotation,
            self.base_shape.vertices.iter().map(|v| v.extend(0.)),
        );
        let depth = rotation * Vec3A::new(0., 0., self.half_depth);

        aabb.grow(depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<RegularPolygon> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Aabb3d::from_point_cloud(
            translation,
            rotation,
            self.base_shape
                .vertices(0.)
                .into_iter()
                .map(|v| v.extend(0.)),
        );
        let depth = rotation * Vec3A::new(0., 0., self.half_depth);

        aabb.grow(depth.abs())
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<Capsule2d> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let aabb = Cylinder {
            half_height: self.half_depth,
            radius: self.base_shape.radius,
        }
        .aabb_3d(Vec3::ZERO, rotation * Quat::from_rotation_x(FRAC_PI_2));

        let up = rotation * Vec3::new(0., self.base_shape.half_length, 0.);
        let half_size = Into::<Vec3>::into(aabb.max) + up.abs();
        Aabb3d::new(translation, half_size)
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

fn bounding_sphere<T: Primitive2d + Bounded2d>(
    extrusion: &Extrusion<T>,
    translation: Vec3,
    rotation: Quat,
) -> BoundingSphere {
    // We calculate the bounding circle of the base shape.
    // Since each of the extrusions bases will have the same distance from its center,
    // and they are just shifted along the Z-axis, the minimum bounding sphere will be the bounding sphere
    // of the cylinder defined by the two bounding circles of the bases for any base shape
    let BoundingCircle {
        center,
        circle: Circle { radius },
    } = extrusion.base_shape.bounding_circle(Vec2::ZERO, 0.);
    let radius = radius.hypot(extrusion.half_depth);
    let center = translation + rotation * center.extend(0.);

    BoundingSphere::new(center, radius)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use glam::{EulerRot, Quat, Vec2, Vec3, Vec3A};

    use crate::{
        bounding::{Bounded3d, BoundingVolume},
        primitives::{
            Capsule2d, Circle, Ellipse, Extrusion, Polygon, Polyline2d, Rectangle, RegularPolygon,
            Segment2d, Triangle2d,
        },
        Dir2,
    };

    #[test]
    fn circle() {
        let cylinder = Extrusion::new(Circle::new(0.5), 2.0);
        let translation = Vec3::new(2.0, 1.0, 0.0);

        let aabb = cylinder.aabb_3d(translation, Quat::IDENTITY);
        assert_eq!(aabb.center(), Vec3A::from(translation));
        assert_eq!(aabb.half_size(), Vec3A::new(0.5, 0.5, 1.0));

        let bounding_sphere = cylinder.bounding_sphere(translation, Quat::IDENTITY);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 1f32.hypot(0.5));
    }

    #[test]
    fn ellipse() {
        let extrusion = Extrusion::new(Ellipse::new(2.0, 0.5), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_euler(EulerRot::ZYX, FRAC_PI_4, FRAC_PI_4, FRAC_PI_4);

        for _ in 0..1_000_000 {
            let _aabb = extrusion.aabb_3d(translation, rotation);
        }
        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), Vec3A::from(translation));
        assert_eq!(aabb.half_size(), Vec3A::new(2.709784, 1.3801551, 2.436141));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 8f32.sqrt());
    }

    #[test]
    fn rectangle() {
        let extrusion = Extrusion::new(Rectangle::new(2.0, 1.0), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(1.0606602, 1.0606602, 2.));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.2912878);
    }

    #[test]
    fn segment() {
        let extrusion = Extrusion::new(Segment2d::new(Dir2::new_unchecked(Vec2::NEG_Y), 3.), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(0., 2.4748735, 2.4748735));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.5);
    }

    #[test]
    fn polyline() {
        let polyline = Polyline2d::<4>::new([
            Vec2::ONE,
            Vec2::new(-1.0, 1.0),
            Vec2::NEG_ONE,
            Vec2::new(1.0, -1.0),
        ]);
        let extrusion = Extrusion::new(polyline, 3.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(1., 1.7677668, 1.7677668));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
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

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(10., 1.7677668, 1.7677668));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(
            bounding_sphere.center,
            Vec3A::new(3.0, 3.2928934, 4.2928934)
        );
        assert_eq!(bounding_sphere.radius(), 10.111875);
    }

    #[test]
    fn polygon() {
        let polygon = Polygon::<4>::new([
            Vec2::ONE,
            Vec2::new(-1.0, 1.0),
            Vec2::NEG_ONE,
            Vec2::new(1.0, -1.0),
        ]);
        let extrusion = Extrusion::new(polygon, 3.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(1., 1.7677668, 1.7677668));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.0615528);
    }

    #[test]
    fn regular_polygon() {
        let extrusion = Extrusion::new(RegularPolygon::new(2.0, 7), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(
            aabb.center(),
            Vec3A::from(translation) + Vec3A::new(0., 0.0700254, 0.0700254)
        );
        assert_eq!(
            aabb.half_size(),
            Vec3A::new(1.9498558, 2.7584014, 2.7584019)
        );

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 8f32.sqrt());
    }

    #[test]
    fn capsule() {
        let extrusion = Extrusion::new(Capsule2d::new(0.5, 2.0), 4.0);
        let translation = Vec3::new(3., 4., 5.);
        let rotation = Quat::from_rotation_x(FRAC_PI_4);

        let aabb = extrusion.aabb_3d(translation, rotation);
        assert_eq!(aabb.center(), translation.into());
        assert_eq!(aabb.half_size(), Vec3A::new(0.5, 2.4748735, 2.4748735));

        let bounding_sphere = extrusion.bounding_sphere(translation, rotation);
        assert_eq!(bounding_sphere.center, translation.into());
        assert_eq!(bounding_sphere.radius(), 2.5);
    }
}
