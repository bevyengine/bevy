use std::f32::consts::FRAC_PI_2;

use glam::{Vec2, Vec3A, Vec3Swizzles};

use crate::bounding::{BoundingCircle, BoundingVolume};
use crate::primitives::{
    BoxedPolygon, BoxedPolyline2d, Capsule2d, Cuboid, Cylinder, Ellipse, Extrusion, Line2d,
    Plane2d, Polygon, Polyline2d, Primitive2d, Rectangle, RegularPolygon, Segment2d, Triangle2d,
};
use crate::{Quat, Vec3};

use crate::{bounding::Bounded2d, primitives::Circle};

use super::{Aabb3d, Bounded3d, BoundingSphere};

impl Bounded3d for Extrusion<Circle> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        Cylinder {
            half_height: self.half_depth,
            radius: self.base_shape.radius,
        }
        .aabb_3d(translation, rotation * Quat::from_rotation_x(FRAC_PI_2))
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

impl Bounded3d for Extrusion<Plane2d> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let direction =
            rotation * Vec3::new(self.base_shape.normal.y, self.base_shape.normal.x, 0.);
        let half_size = Vec3::new(
            if direction.x == 0. { 0. } else { f32::MAX / 2. },
            if direction.y == 0. { 0. } else { f32::MAX / 2. },
            if direction.z == 0. { 0. } else { f32::MAX / 2. },
        );
        Aabb3d::new(translation, half_size)
    }

    fn bounding_sphere(&self, translation: Vec3, rotation: Quat) -> BoundingSphere {
        bounding_sphere(self, translation, rotation)
    }
}

impl Bounded3d for Extrusion<Line2d> {
    fn aabb_3d(&self, translation: Vec3, rotation: Quat) -> Aabb3d {
        let direction = rotation * self.base_shape.direction.extend(0.);
        let half_size = Vec3::new(
            if direction.x == 0. { 0. } else { f32::MAX / 2. },
            if direction.y == 0. { 0. } else { f32::MAX / 2. },
            if direction.z == 0. { 0. } else { f32::MAX / 2. },
        );
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
