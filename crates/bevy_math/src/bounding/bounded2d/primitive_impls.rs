//! Contains [`Bounded2d`](super::Bounded2d) implementations for [geometric primitives](crate::primitives).

use glam::{Mat2, Vec2};

use crate::primitives::{
    BoxedPolygon, BoxedPolyline2d, Circle, Direction2d, Ellipse, Line2d, Plane2d, Polygon,
    Polyline2d, Rectangle, RegularPolygon, Segment2d, Triangle2d,
};

use super::{rotate_vec2, Aabb2d, Bounded2d, BoundingCircle};

impl Bounded2d for Circle {
    fn aabb_2d(&self, translation: Vec2, _rotation: f32) -> Aabb2d {
        Aabb2d {
            min: translation - Vec2::splat(self.radius),
            max: translation + Vec2::splat(self.radius),
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.radius)
    }
}

impl Bounded2d for Ellipse {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        //           V = (hh * cos(beta), hh * sin(beta))
        //      #####*#####
        //   ###     |     ###
        //  #     hh |        #
        // #         *---------* U = (hw * cos(alpha), hw * sin(alpha))
        //  #            hw   #
        //   ###           ###
        //      ###########

        let (alpha, beta) = (rotation, rotation + std::f32::consts::FRAC_PI_2);
        let (hw, hh) = (self.half_width, self.half_height);

        let (ux, uy) = (hw * alpha.cos(), hw * alpha.sin());
        let (vx, vy) = (hh * beta.cos(), hh * beta.sin());

        let half_extents = Vec2::new(ux.hypot(vx), uy.hypot(vy));

        Aabb2d {
            min: translation - half_extents,
            max: translation + half_extents,
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.half_width.max(self.half_height))
    }
}

impl Bounded2d for Plane2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // Add or subtract pi/2 from the rotation to get a direction in the right side of the plane
        let direction = rotate_vec2(
            *self.normal,
            rotation - std::f32::consts::FRAC_PI_2 * self.normal.y.signum(),
        );
        let x_parallel = direction == Vec2::X || direction == Vec2::NEG_X;
        let y_parallel = direction == Vec2::Y || direction == Vec2::NEG_Y;

        // Dividing `f32::MAX` by 2.0 can actually be good so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let half_width = if y_parallel { 0.0 } else { f32::MAX / 2.0 };
        let half_height = if x_parallel { 0.0 } else { f32::MAX / 2.0 };
        let half_size = Vec2::new(half_width, half_height);

        Aabb2d {
            min: translation - half_size,
            max: translation + half_size,
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, f32::MAX / 2.0)
    }
}

impl Bounded2d for Line2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let direction = rotate_vec2(*self.direction, rotation);
        let x_parallel = direction == Vec2::X || direction == Vec2::NEG_X;
        let y_parallel = direction == Vec2::Y || direction == Vec2::NEG_Y;

        // Dividing `f32::MAX` by 2.0 can actually be good so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let half_width = if y_parallel { 0.0 } else { f32::MAX / 2.0 };
        let half_height = if x_parallel { 0.0 } else { f32::MAX / 2.0 };
        let half_size = Vec2::new(half_width, half_height);

        Aabb2d {
            min: translation - half_size,
            max: translation + half_size,
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, f32::MAX / 2.0)
    }
}

impl Bounded2d for Segment2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // Rotate the segment by `rotation`
        let direction = Direction2d::from_normalized(rotate_vec2(*self.direction, rotation));
        let segment = Self { direction, ..*self };
        let (point1, point2) = (segment.point1(), segment.point2());

        Aabb2d {
            min: translation + point1.min(point2),
            max: translation + point1.max(point2),
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.half_length)
    }
}

impl<const N: usize> Bounded2d for Polyline2d<N> {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, self.vertices)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for BoxedPolyline2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, self.vertices.to_vec())
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for Triangle2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let [a, b, c] = self.vertices.map(|vtx| rotate_vec2(vtx, rotation));

        let min = Vec2::new(a.x.min(b.x).min(c.x), a.y.min(b.y).min(c.y));
        let max = Vec2::new(a.x.max(b.x).max(c.x), a.y.max(b.y).max(c.y));

        Aabb2d {
            min: min + translation,
            max: max + translation,
        }
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        self.aabb_2d(translation, rotation).bounding_circle()
    }
}

impl Bounded2d for Rectangle {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let half_size = Vec2::new(self.half_width, self.half_height);

        // Compute the AABB of the rotated rectangle by transforming the half-extents
        // by an absolute rotation matrix.
        let (sin, cos) = rotation.sin_cos();
        let abs_rot_mat = Mat2::from_cols_array(&[cos.abs(), sin.abs(), sin.abs(), cos.abs()]);
        let half_extents = abs_rot_mat * half_size;

        Aabb2d {
            min: translation - half_extents,
            max: translation + half_extents,
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        let radius = self.half_width.hypot(self.half_height);
        BoundingCircle::new(translation, radius)
    }
}

impl<const N: usize> Bounded2d for Polygon<N> {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, self.vertices)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for BoxedPolygon {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, self.vertices.to_vec())
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for RegularPolygon {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let mut min = Vec2::ZERO;
        let mut max = Vec2::ZERO;

        for vertex in self.vertices(rotation) {
            min = min.min(vertex);
            max = max.max(vertex);
        }

        Aabb2d {
            min: min + translation,
            max: max + translation,
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.circumcircle.radius)
    }
}
