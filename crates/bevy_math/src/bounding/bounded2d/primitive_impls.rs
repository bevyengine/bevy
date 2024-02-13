//! Contains [`Bounded2d`] implementations for [geometric primitives](crate::primitives).

use glam::{Mat2, Vec2};

use crate::primitives::{
    Arc2d, BoxedPolygon, BoxedPolyline2d, Capsule2d, Circle, CircularSector, CircularSegment,
    Direction2d, Ellipse, Line2d, Plane2d, Polygon, Polyline2d, Rectangle, RegularPolygon,
    Segment2d, Triangle2d,
};

use super::{Aabb2d, Bounded2d, BoundingCircle};

impl Bounded2d for Circle {
    fn aabb_2d(&self, translation: Vec2, _rotation: f32) -> Aabb2d {
        Aabb2d::new(translation, Vec2::splat(self.radius))
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.radius)
    }
}

impl Bounded2d for Arc2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // Because our arcs are always symmetrical around Vec::Y, the uppermost point and the two endpoints will always be extrema.
        // For an arc that is greater than a semicircle, radii to the left and right will also be bounding points.
        // This gives five possible bounding points, so we will lay them out in an array and then
        // select the appropriate slice to compute the bounding box of.
        let all_bounds = [
            self.left_endpoint(),
            self.right_endpoint(),
            self.radius * Vec2::Y,
            self.radius * Vec2::X,
            self.radius * -Vec2::X,
        ];
        let bounds = if self.is_major() {
            &all_bounds[0..5]
        } else {
            &all_bounds[0..3]
        };
        Aabb2d::from_point_cloud(translation, rotation, bounds)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        // There are two possibilities for the bounding circle.
        if self.is_major() {
            // If the arc is major, then the widest distance between two points is a diameter of the arc's circle;
            // therefore, that circle is the bounding radius.
            BoundingCircle::new(translation, self.radius)
        } else {
            // Otherwise, the widest distance between two points is the chord,
            // so a circle of that diameter around the midpoint will contain the entire arc.
            let center = self.chord_midpoint().rotate(Vec2::from_angle(rotation));
            BoundingCircle::new(center + translation, self.half_chord_length())
        }
    }
}

impl Bounded2d for CircularSector {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // This is identical to the implementation for Arc2d, above, with the additional possibility of the
        // origin point, the center of the arc, acting as a bounding point when the arc is minor.
        //
        // See comments above for an explanation of the logic.
        let all_bounds = [
            Vec2::ZERO,
            self.arc.left_endpoint(),
            self.arc.right_endpoint(),
            self.arc.radius * Vec2::Y,
            self.arc.radius * Vec2::X,
            self.arc.radius * -Vec2::X,
        ];
        let bounds = if self.arc.is_major() {
            &all_bounds[1..6]
        } else {
            &all_bounds[0..4]
        };
        Aabb2d::from_point_cloud(translation, rotation, bounds)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        // There are three possibilities for the bounding circle.
        if self.arc.is_major() {
            // If the arc is major, then the widest distance between two points is a diameter of the arc's circle;
            // therefore, that circle is the bounding radius.
            BoundingCircle::new(translation, self.arc.radius)
        } else if self.arc.chord_length() < self.arc.radius {
            // If the chord length is smaller than the radius, then the radius is the widest distance between two points,
            // so the bounding circle is centered on the midpoint of the radius.
            let half_radius = self.arc.radius / 2.0;
            let center = half_radius * Vec2::Y;
            BoundingCircle::new(center + translation, half_radius)
        } else {
            // Otherwise, the widest distance between two points is the chord,
            // so a circle of that diameter around the midpoint will contain the entire arc.
            let center = self.arc.chord_midpoint().rotate(Vec2::from_angle(rotation));
            BoundingCircle::new(center + translation, self.arc.half_chord_length())
        }
    }
}

impl Bounded2d for CircularSegment {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        self.arc.aabb_2d(translation, rotation)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        self.arc.bounding_circle(translation, rotation)
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

        let (hw, hh) = (self.half_size.x, self.half_size.y);

        // Sine and cosine of rotation angle alpha.
        let (alpha_sin, alpha_cos) = rotation.sin_cos();

        // Sine and cosine of alpha + pi/2. We can avoid the trigonometric functions:
        // sin(beta) = sin(alpha + pi/2) = cos(alpha)
        // cos(beta) = cos(alpha + pi/2) = -sin(alpha)
        let (beta_sin, beta_cos) = (alpha_cos, -alpha_sin);

        // Compute points U and V, the extremes of the ellipse
        let (ux, uy) = (hw * alpha_cos, hw * alpha_sin);
        let (vx, vy) = (hh * beta_cos, hh * beta_sin);

        let half_size = Vec2::new(ux.hypot(vx), uy.hypot(vy));

        Aabb2d::new(translation, half_size)
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.semi_major())
    }
}

impl Bounded2d for Plane2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let normal = Mat2::from_angle(rotation) * *self.normal;
        let facing_x = normal == Vec2::X || normal == Vec2::NEG_X;
        let facing_y = normal == Vec2::Y || normal == Vec2::NEG_Y;

        // Dividing `f32::MAX` by 2.0 is helpful so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let half_width = if facing_x { 0.0 } else { f32::MAX / 2.0 };
        let half_height = if facing_y { 0.0 } else { f32::MAX / 2.0 };
        let half_size = Vec2::new(half_width, half_height);

        Aabb2d::new(translation, half_size)
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, f32::MAX / 2.0)
    }
}

impl Bounded2d for Line2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let direction = Mat2::from_angle(rotation) * *self.direction;

        // Dividing `f32::MAX` by 2.0 is helpful so that we can do operations
        // like growing or shrinking the AABB without breaking things.
        let max = f32::MAX / 2.0;
        let half_width = if direction.x == 0.0 { 0.0 } else { max };
        let half_height = if direction.y == 0.0 { 0.0 } else { max };
        let half_size = Vec2::new(half_width, half_height);

        Aabb2d::new(translation, half_size)
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, f32::MAX / 2.0)
    }
}

impl Bounded2d for Segment2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // Rotate the segment by `rotation`
        let direction = Mat2::from_angle(rotation) * *self.direction;
        let half_size = (self.half_length * direction).abs();

        Aabb2d::new(translation, half_size)
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.half_length)
    }
}

impl<const N: usize> Bounded2d for Polyline2d<N> {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, &self.vertices)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for BoxedPolyline2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, &self.vertices)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for Triangle2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        let rotation_mat = Mat2::from_angle(rotation);
        let [a, b, c] = self.vertices.map(|vtx| rotation_mat * vtx);

        let min = Vec2::new(a.x.min(b.x).min(c.x), a.y.min(b.y).min(c.y));
        let max = Vec2::new(a.x.max(b.x).max(c.x), a.y.max(b.y).max(c.y));

        Aabb2d {
            min: min + translation,
            max: max + translation,
        }
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        let rotation_mat = Mat2::from_angle(rotation);
        let [a, b, c] = self.vertices;

        // The points of the segment opposite to the obtuse or right angle if one exists
        let side_opposite_to_non_acute = if (b - a).dot(c - a) <= 0.0 {
            Some((b, c))
        } else if (c - b).dot(a - b) <= 0.0 {
            Some((c, a))
        } else if (a - c).dot(b - c) <= 0.0 {
            Some((a, b))
        } else {
            // The triangle is acute.
            None
        };

        // Find the minimum bounding circle. If the triangle is obtuse, the circle passes through two vertices.
        // Otherwise, it's the circumcircle and passes through all three.
        if let Some((point1, point2)) = side_opposite_to_non_acute {
            // The triangle is obtuse or right, so the minimum bounding circle's diameter is equal to the longest side.
            // We can compute the minimum bounding circle from the line segment of the longest side.
            let (segment, center) = Segment2d::from_points(point1, point2);
            segment.bounding_circle(rotation_mat * center + translation, rotation)
        } else {
            // The triangle is acute, so the smallest bounding circle is the circumcircle.
            let (Circle { radius }, circumcenter) = self.circumcircle();
            BoundingCircle::new(rotation_mat * circumcenter + translation, radius)
        }
    }
}

impl Bounded2d for Rectangle {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // Compute the AABB of the rotated rectangle by transforming the half-extents
        // by an absolute rotation matrix.
        let (sin, cos) = rotation.sin_cos();
        let abs_rot_mat = Mat2::from_cols_array(&[cos.abs(), sin.abs(), sin.abs(), cos.abs()]);
        let half_size = abs_rot_mat * self.half_size;

        Aabb2d::new(translation, half_size)
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        let radius = self.half_size.length();
        BoundingCircle::new(translation, radius)
    }
}

impl<const N: usize> Bounded2d for Polygon<N> {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, &self.vertices)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        BoundingCircle::from_point_cloud(translation, rotation, &self.vertices)
    }
}

impl Bounded2d for BoxedPolygon {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        Aabb2d::from_point_cloud(translation, rotation, &self.vertices)
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

impl Bounded2d for Capsule2d {
    fn aabb_2d(&self, translation: Vec2, rotation: f32) -> Aabb2d {
        // Get the line segment between the hemicircles of the rotated capsule
        let segment = Segment2d {
            // Multiplying a normalized vector (Vec2::Y) with a rotation returns a normalized vector.
            direction: Direction2d::new_unchecked(Mat2::from_angle(rotation) * Vec2::Y),
            half_length: self.half_length,
        };
        let (a, b) = (segment.point1(), segment.point2());

        // Expand the line segment by the capsule radius to get the capsule half-extents
        let min = a.min(b) - Vec2::splat(self.radius);
        let max = a.max(b) + Vec2::splat(self.radius);

        Aabb2d {
            min: min + translation,
            max: max + translation,
        }
    }

    fn bounding_circle(&self, translation: Vec2, _rotation: f32) -> BoundingCircle {
        BoundingCircle::new(translation, self.radius + self.half_length)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::{
        bounding::Bounded2d,
        primitives::{
            Capsule2d, Circle, Direction2d, Ellipse, Line2d, Plane2d, Polygon, Polyline2d,
            Rectangle, RegularPolygon, Segment2d, Triangle2d,
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
        let ellipse = Ellipse::new(1.0, 0.5);
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
    fn acute_triangle() {
        let acute_triangle =
            Triangle2d::new(Vec2::new(0.0, 1.0), Vec2::NEG_ONE, Vec2::new(1.0, -1.0));
        let translation = Vec2::new(2.0, 1.0);

        let aabb = acute_triangle.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(1.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(3.0, 2.0));

        // For acute triangles, the center is the circumcenter
        let (Circle { radius }, circumcenter) = acute_triangle.circumcircle();
        let bounding_circle = acute_triangle.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, circumcenter + translation);
        assert_eq!(bounding_circle.radius(), radius);
    }

    #[test]
    fn obtuse_triangle() {
        let obtuse_triangle = Triangle2d::new(
            Vec2::new(0.0, 1.0),
            Vec2::new(-10.0, -1.0),
            Vec2::new(10.0, -1.0),
        );
        let translation = Vec2::new(2.0, 1.0);

        let aabb = obtuse_triangle.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, Vec2::new(-8.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(12.0, 2.0));

        // For obtuse and right triangles, the center is the midpoint of the longest side (diameter of bounding circle)
        let bounding_circle = obtuse_triangle.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation - Vec2::Y);
        assert_eq!(bounding_circle.radius(), 10.0);
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

    #[test]
    fn capsule() {
        let capsule = Capsule2d::new(0.5, 2.0);
        let translation = Vec2::new(2.0, 1.0);

        let aabb = capsule.aabb_2d(translation, 0.0);
        assert_eq!(aabb.min, translation - Vec2::new(0.5, 1.5));
        assert_eq!(aabb.max, translation + Vec2::new(0.5, 1.5));

        let bounding_circle = capsule.bounding_circle(translation, 0.0);
        assert_eq!(bounding_circle.center, translation);
        assert_eq!(bounding_circle.radius(), 1.5);
    }
}
