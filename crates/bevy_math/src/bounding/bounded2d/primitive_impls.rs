//! Contains [`Bounded2d`] implementations for [geometric primitives](crate::primitives).

use std::f32::consts::PI;

use glam::{Mat2, Vec2};
use smallvec::SmallVec;

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
        // If our arc covers more than a circle, just return the bounding box of the circle.
        if self.half_angle >= PI {
            return Circle {
                radius: self.radius,
            }
            .aabb_2d(translation, rotation);
        }

        // Otherwise, the extreme points will always be either the endpoints or the axis-aligned extrema of the arc's circle.
        // We need to compute which axis-aligned extrema are actually contained within the rotated arc.
        let mut bounds = SmallVec::<[Vec2; 6]>::new();
        let rotation_vec = Vec2::from_angle(rotation);
        bounds.push(self.left_endpoint().rotate(rotation_vec));
        bounds.push(self.right_endpoint().rotate(rotation_vec));

        // The half-angles are measured from a starting point of π/2, being the angle of Vec::Y.
        // Compute the normalized angles of the endpoints with the rotation taken into account, and then
        // check if we are looking for an angle that is between or outside them.
        let left_angle = (PI / 2.0 + self.half_angle + rotation).rem_euclid(2.0 * PI);
        let right_angle = (PI / 2.0 - self.half_angle + rotation).rem_euclid(2.0 * PI);
        let inverted = left_angle < right_angle;
        for extremum in [Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y] {
            let angle = extremum.to_angle().rem_euclid(2.0 * PI);
            // If inverted = true, then right_angle > left_angle, so we are looking for an angle that is not between them.
            // There's a chance that this condition fails due to rounding error, if the endpoint angle is juuuust shy of the axis.
            // But in that case, the endpoint itself is within rounding error of the axis and will define the bounds just fine.
            if !inverted && angle >= right_angle && angle <= left_angle
                || inverted && (angle >= right_angle || angle <= left_angle)
            {
                bounds.push(extremum * self.radius);
            }
        }

        Aabb2d::from_point_cloud(translation, 0.0, &dbg!(bounds))
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
        // If our sector covers more than a circle, just return the bounding box of the circle.
        if self.half_angle() >= PI {
            return Circle {
                radius: self.radius(),
            }
            .aabb_2d(translation, rotation);
        }

        // Otherwise, we use the same logic as for Arc2d, above, just with the circle's cetner as an additional possibility.
        // See above for discussion.
        let mut bounds = SmallVec::<[Vec2; 7]>::new();
        let rotation_vec = Vec2::from_angle(rotation);
        bounds.push(self.arc.left_endpoint().rotate(rotation_vec));
        bounds.push(self.arc.right_endpoint().rotate(rotation_vec));
        bounds.push(self.circle_center());

        let left_angle = (PI / 2.0 + self.half_angle() + rotation).rem_euclid(2.0 * PI);
        let right_angle = (PI / 2.0 - self.half_angle() + rotation).rem_euclid(2.0 * PI);
        let inverted = left_angle < right_angle;
        for extremum in [Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y] {
            let angle = extremum.to_angle().rem_euclid(2.0 * PI);
            if !inverted && angle >= right_angle && angle <= left_angle
                || inverted && (angle <= right_angle || angle >= left_angle)
            {
                bounds.push(extremum * self.radius());
            }
        }

        Aabb2d::from_point_cloud(translation, 0.0, &bounds)
    }

    fn bounding_circle(&self, translation: Vec2, rotation: f32) -> BoundingCircle {
        if self.arc.is_major() {
            // If the arc is major, that is, greater than a semicircle,
            // then bounding circle is just the circle defining the sector.
            BoundingCircle::new(translation, self.arc.radius)
        } else {
            // However, when the arc is minor,
            // we need our bounding circle to include both endpoints of the arc as well as the circle center.
            // This means we need the circumcircle of those three points.
            // The circumcircle will always have a greater curvature than the circle itself, so it will contain
            // the entire circular sector.
            Triangle2d::new(
                self.circle_center(),
                self.arc.left_endpoint(),
                self.arc.right_endpoint(),
            )
            .bounding_circle(translation, rotation)
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
    use std::f32::consts::PI;

    use approx::assert_abs_diff_eq;
    use glam::Vec2;

    use crate::{
        bounding::Bounded2d,
        primitives::{
            Arc2d, Capsule2d, Circle, CircularSegment, Direction2d, Ellipse, Line2d, Plane2d,
            Polygon, Polyline2d, Rectangle, RegularPolygon, Segment2d, Triangle2d,
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
    // Arcs and circular segments have the same bounding shapes so they share test cases.
    fn arc_and_segment() {
        struct TestCase {
            name: &'static str,
            arc: Arc2d,
            translation: Vec2,
            rotation: f32,
            aabb_min: Vec2,
            aabb_max: Vec2,
            bounding_circle_center: Vec2,
            bounding_circle_radius: f32,
        }

        // The apothem of an arc covering 1/6th of a circle.
        let apothem = f32::sqrt(3.0) / 2.0;
        let tests = [
            TestCase {
                name: "1/6th circle untransformed",
                arc: Arc2d::from_radians(1.0, PI / 3.0),
                translation: Vec2::ZERO,
                rotation: 0.0,
                aabb_min: Vec2::new(-0.5, apothem),
                aabb_max: Vec2::new(0.5, 1.0),
                bounding_circle_center: Vec2::new(0.0, apothem),
                bounding_circle_radius: 0.5,
            },
            TestCase {
                name: "1/6th circle with radius 0.5",
                arc: Arc2d::from_radians(0.5, PI / 3.0),
                translation: Vec2::ZERO,
                rotation: 0.0,
                aabb_min: Vec2::new(-0.25, apothem / 2.0),
                aabb_max: Vec2::new(0.25, 0.5),
                bounding_circle_center: Vec2::new(0.0, apothem / 2.0),
                bounding_circle_radius: 0.25,
            },
            TestCase {
                name: "1/6th circle with radius 2.0",
                arc: Arc2d::from_radians(2.0, PI / 3.0),
                translation: Vec2::ZERO,
                rotation: 0.0,
                aabb_min: Vec2::new(-1.0, 2.0 * apothem),
                aabb_max: Vec2::new(1.0, 2.0),
                bounding_circle_center: Vec2::new(0.0, 2.0 * apothem),
                bounding_circle_radius: 1.0,
            },
            TestCase {
                name: "1/6th circle translated",
                arc: Arc2d::from_radians(1.0, PI / 3.0),
                translation: Vec2::new(2.0, 3.0),
                rotation: 0.0,
                aabb_min: Vec2::new(1.5, 3.0 + apothem),
                aabb_max: Vec2::new(2.5, 4.0),
                bounding_circle_center: Vec2::new(2.0, 3.0 + apothem),
                bounding_circle_radius: 0.5,
            },
            TestCase {
                name: "1/6th circle rotated",
                arc: Arc2d::from_radians(1.0, PI / 3.0),
                translation: Vec2::ZERO,
                // Rotate left by 1/12 of a circle, so the right endpoint is on the y-axis.
                rotation: PI / 6.0,
                aabb_min: Vec2::new(-apothem, 0.5),
                aabb_max: Vec2::new(0.0, 1.0),
                // The exact coordinates here are not obvious, but can be computed by constructing
                // an altitude from the midpoint of the chord to the y-axis and using the right triangle
                // similarity theorem.
                bounding_circle_center: Vec2::new(-apothem / 2.0, apothem.powi(2)),
                bounding_circle_radius: 0.5,
            },
            TestCase {
                name: "1/4er circle rotated to be axis-aligned",
                arc: Arc2d::from_radians(1.0, PI / 2.0),
                translation: Vec2::ZERO,
                // Rotate right by 1/8 of a circle, so the right endpoint is on the x-axis and the left endpoint is on the y-axis.
                rotation: -PI / 4.0,
                aabb_min: Vec2::ZERO,
                aabb_max: Vec2::splat(1.0),
                bounding_circle_center: Vec2::splat(0.5),
                bounding_circle_radius: f32::sqrt(2.0) / 2.0,
            },
            TestCase {
                name: "5/6th circle untransformed",
                arc: Arc2d::from_radians(1.0, 5.0 * PI / 3.0),
                translation: Vec2::ZERO,
                rotation: 0.0,
                aabb_min: Vec2::new(-1.0, -apothem),
                aabb_max: Vec2::new(1.0, 1.0),
                bounding_circle_center: Vec2::ZERO,
                bounding_circle_radius: 1.0,
            },
            TestCase {
                name: "5/6th circle translated",
                arc: Arc2d::from_radians(1.0, 5.0 * PI / 3.0),
                translation: Vec2::new(2.0, 3.0),
                rotation: 0.0,
                aabb_min: Vec2::new(1.0, 3.0 - apothem),
                aabb_max: Vec2::new(3.0, 4.0),
                bounding_circle_center: Vec2::new(2.0, 3.0),
                bounding_circle_radius: 1.0,
            },
            TestCase {
                name: "5/6th circle rotated",
                arc: Arc2d::from_radians(1.0, 5.0 * PI / 3.0),
                translation: Vec2::ZERO,
                // Rotate left by 1/12 of a circle, so the left endpoint is on the y-axis.
                rotation: PI / 6.0,
                aabb_min: Vec2::new(-1.0, -1.0),
                aabb_max: Vec2::new(1.0, 1.0),
                bounding_circle_center: Vec2::ZERO,
                bounding_circle_radius: 1.0,
            },
        ];

        for test in tests {
            println!("subtest case: {}", test.name);
            let arc = test.arc;
            let segment: CircularSegment = arc.clone().into();

            let arc_aabb = arc.aabb_2d(test.translation, test.rotation);
            assert_abs_diff_eq!(test.aabb_min, arc_aabb.min);
            assert_abs_diff_eq!(test.aabb_max, arc_aabb.max);
            let segment_aabb = segment.aabb_2d(test.translation, test.rotation);
            assert_abs_diff_eq!(test.aabb_min, segment_aabb.min);
            assert_abs_diff_eq!(test.aabb_max, segment_aabb.max);

            let arc_bounding_circle = arc.bounding_circle(test.translation, test.rotation);
            assert_abs_diff_eq!(test.bounding_circle_center, arc_bounding_circle.center);
            assert_abs_diff_eq!(test.bounding_circle_radius, arc_bounding_circle.radius());
            let segment_bounding_circle = segment.bounding_circle(test.translation, test.rotation);
            assert_abs_diff_eq!(test.bounding_circle_center, segment_bounding_circle.center);
            assert_abs_diff_eq!(
                test.bounding_circle_radius,
                segment_bounding_circle.radius()
            );
        }
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
