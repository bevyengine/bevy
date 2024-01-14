//! Contains [`Bounded2d`] implementations for [geometric primitives](crate::primitives).

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
        let normal = rotate_vec2(*self.normal, rotation);
        let facing_x = normal == Vec2::X || normal == Vec2::NEG_X;
        let facing_y = normal == Vec2::Y || normal == Vec2::NEG_Y;

        // Dividing `f32::MAX` by 2.0 can actually be good so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let half_width = if facing_x { 0.0 } else { f32::MAX / 2.0 };
        let half_height = if facing_y { 0.0 } else { f32::MAX / 2.0 };
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

        // Dividing `f32::MAX` by 2.0 can actually be good so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let max = f32::MAX / 2.0;
        let half_width = if direction.x == 0.0 { 0.0 } else { max };
        let half_height = if direction.y == 0.0 { 0.0 } else { max };
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
        let (Circle { radius }, circumcenter) = self.circumcircle();
        BoundingCircle::new(rotate_vec2(circumcenter, rotation) + translation, radius)
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

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::{
        bounding::Bounded2d,
        primitives::{
            Circle, Direction2d, Ellipse, Line2d, Plane2d, Polygon, Polyline2d, Rectangle,
            RegularPolygon, Segment2d, Triangle2d,
        },
    };

    #[test]
    fn circle() {
        let circle = Circle { radius: 1.0 };
        let translation = Vec2::new(2.0, 1.0);

        let aabb = circle.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(3.0, 2.0));

        let bounding_circle = circle.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), 1.0);
    }

    #[test]
    fn ellipse() {
        let ellipse = Ellipse {
            half_width: 1.0,
            half_height: 0.5,
        };
        let translation = Vec2::new(2.0, 1.0);

        let aabb = ellipse.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.5));
        assert_eq!(aabb.max, Vec2::new(3.0, 1.5));

        let bounding_circle = ellipse.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), 1.0);
    }

    #[test]
    fn plane() {
        let translation = Vec2::new(2.0, 1.0);

        let aabb1 = Plane2d::new(Vec2::X).aabb_2d(translation, 0.0);
        assert_eq!(aabb1.min, Vec2::new(2.0, -f32::MAX / 2.0));
        assert_eq!(aabb1.max, Vec2::new(2.0, f32::MAX / 2.0));

        let aabb2 = Plane2d::new(Vec2::Y).aabb_2d(translation, 0.0);
        assert_eq!(aabb2.min, Vec2::new(-f32::MAX / 2.0, 1.0));
        assert_eq!(aabb2.max, Vec2::new(f32::MAX / 2.0, 1.0));

        let aabb3 = Plane2d::new(Vec2::ONE).aabb_2d(translation, 0.0);
        assert_eq!(aabb3.min, Vec2::new(-f32::MAX / 2.0, -f32::MAX / 2.0));
        assert_eq!(aabb3.max, Vec2::new(f32::MAX / 2.0, f32::MAX / 2.0));

        let bounding_circle = Plane2d::new(Vec2::Y).bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), f32::MAX / 2.0);
    }

    #[test]
    fn line() {
        let translation = Vec2::new(2.0, 1.0);

        let aabb1 = Line2d {
            direction: Direction2d::Y,
        }
        .aabb_2d(translation, 0.0);
        assert_eq!(aabb1.min, Vec2::new(2.0, -f32::MAX / 2.0));
        assert_eq!(aabb1.max, Vec2::new(2.0, f32::MAX / 2.0));

        let aabb2 = Line2d {
            direction: Direction2d::X,
        }
        .aabb_2d(translation, 0.0);
        assert_eq!(aabb2.min, Vec2::new(-f32::MAX / 2.0, 1.0));
        assert_eq!(aabb2.max, Vec2::new(f32::MAX / 2.0, 1.0));

        let aabb3 = Line2d {
            direction: Direction2d::from_xy(1.0, 1.0).unwrap(),
        }
        .aabb_2d(translation, 0.0);
        assert_eq!(aabb3.min, Vec2::new(-f32::MAX / 2.0, -f32::MAX / 2.0));
        assert_eq!(aabb3.max, Vec2::new(f32::MAX / 2.0, f32::MAX / 2.0));

        let bounding_circle = Line2d {
            direction: Direction2d::Y,
        }
        .bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), f32::MAX / 2.0);
    }

    #[test]
    fn segment() {
        let translation = Vec2::new(2.0, 1.0);
        let segment = Segment2d::from_points(Vec2::new(-1.0, -0.5), Vec2::new(1.0, 0.5)).0;

        let aabb = segment.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.5));
        assert_eq!(aabb.max, Vec2::new(3.0, 1.5));

        let bounding_circle = segment.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), 1.0_f32.hypot(0.5));
    }

    #[test]
    fn polyline() {
        let polyline = Polyline2d::<4>::new([
            Vec2::ONE,
            Vec2::new(-1.0, 1.0),
            Vec2::NEG_ONE,
            Vec2::new(1.0, -1.0),
        ]);
        let translation = Vec2::new(2.0, 1.0);

        let aabb = polyline.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(3.0, 2.0));

        let bounding_circle = polyline.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), std::f32::consts::SQRT_2);
    }

    #[test]
    fn triangle() {
        let triangle = Triangle2d::new(Vec2::new(0.0, 1.0), Vec2::NEG_ONE, Vec2::new(1.0, -1.0));
        let translation = Vec2::new(2.0, 1.0);

        let aabb = triangle.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(3.0, 2.0));

        let bounding_circle = triangle.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation + Vec2::NEG_Y * 0.25);
        assert_eq!(bounding_circle.radius(), 1.25);
    }

    #[test]
    fn rectangle() {
        let rectangle = Rectangle::new(2.0, 1.0);
        let translation = Vec2::new(2.0, 1.0);

        let aabb = rectangle.aabb_2d(translation, std::f32::consts::FRAC_PI_4);
        let expected_half_size = Vec2::splat(1.0606601);
        assert_eq!(aabb.min, translation - expected_half_size);
        assert_eq!(aabb.max, translation + expected_half_size);

        let bounding_circle = rectangle.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), 1.0_f32.hypot(0.5));
    }

    #[test]
    fn polygon() {
        let polygon = Polygon::<4>::new([
            Vec2::ONE,
            Vec2::new(-1.0, 1.0),
            Vec2::NEG_ONE,
            Vec2::new(1.0, -1.0),
        ]);
        let translation = Vec2::new(2.0, 1.0);

        let aabb = polygon.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(3.0, 2.0));

        let bounding_circle = polygon.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), std::f32::consts::SQRT_2);
    }

    #[test]
    fn regular_polygon() {
        let regular_polygon = RegularPolygon::new(1.0, 5);
        let translation = Vec2::new(2.0, 1.0);

        let aabb = regular_polygon.aabb_2d(translation, 0.0);
        assert!((aabb.min - (translation - Vec2::new(0.9510565, 0.8090169))).length() < 1e-6);
        assert!((aabb.max - (translation + Vec2::new(0.9510565, 1.0))).length() < 1e-6);

        let bounding_circle = regular_polygon.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), 1.0);
    }
}
