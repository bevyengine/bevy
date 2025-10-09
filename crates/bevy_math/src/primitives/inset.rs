use crate::{
    ops,
    primitives::{
        Capsule2d, Circle, CircularSegment, Primitive2d, Rectangle, RegularPolygon, Rhombus,
        Triangle2d,
    },
    Vec2,
};

/// A primitive that can be resized uniformly.
///
/// See documentation on [`Inset::inset`].
///
/// See also [`ToRing`](crate::primitives::ToRing).
pub trait Inset: Primitive2d {
    /// Create a new version of this primitive that is resized uniformly.
    /// That is, it resizes the shape inwards such that for the lines between vertices,
    /// it creates new parallel lines that are `distance` inwards from the original lines.
    ///
    /// This is useful for creating smaller shapes or making outlines of `distance` thickness with [`Ring`](crate::primitives::Ring).
    ///
    /// See also [`ToRing::to_ring`](crate::primitives::ToRing::to_ring)
    fn inset(self, distance: f32) -> Self;
}

impl Inset for Circle {
    fn inset(mut self, distance: f32) -> Self {
        self.radius -= distance;
        self
    }
}

impl Inset for Triangle2d {
    fn inset(self, distance: f32) -> Self {
        fn find_inset_point(a: Vec2, b: Vec2, c: Vec2, distance: f32) -> Vec2 {
            let unit_vector_ab = (b - a).normalize();
            let unit_vector_ac = (c - a).normalize();
            let half_angle_bac = unit_vector_ab.angle_to(unit_vector_ac) / 2.0;
            let mean = (unit_vector_ab + unit_vector_ac) / 2.0;
            let direction = mean.normalize();
            let magnitude = distance / ops::sin(half_angle_bac);
            a + direction * magnitude
        }

        let [a, b, c] = self.vertices;

        let new_a = find_inset_point(a, b, c, distance);
        let new_b = find_inset_point(b, c, a, distance);
        let new_c = find_inset_point(c, a, b, distance);

        Self::new(new_a, new_b, new_c)
    }
}

impl Inset for Rhombus {
    fn inset(mut self, distance: f32) -> Self {
        let [half_width, half_height] = self.half_diagonals.into();
        let angle = ops::atan(half_height / half_width);
        let x_offset = distance / ops::sin(angle);
        let y_offset = distance / ops::cos(angle);
        self.half_diagonals -= Vec2::new(x_offset, y_offset);
        self
    }
}

impl Inset for Capsule2d {
    fn inset(mut self, distance: f32) -> Self {
        self.radius -= distance;
        self
    }
}

impl Inset for Rectangle {
    fn inset(mut self, distance: f32) -> Self {
        self.half_size -= Vec2::splat(distance);
        self
    }
}

impl Inset for CircularSegment {
    fn inset(self, distance: f32) -> Self {
        let old_arc = self.arc;
        let radius = old_arc.radius - distance;
        let apothem = old_arc.apothem() + distance;
        // https://en.wikipedia.org/wiki/Circular_segment
        let half_angle = ops::acos(apothem / radius);
        Self::new(radius, half_angle)
    }
}

impl Inset for RegularPolygon {
    fn inset(mut self, distance: f32) -> Self {
        let half_angle = self.internal_angle_radians() / 2.0;
        let offset = distance / ops::sin(half_angle);
        self.circumcircle.radius -= offset;
        self
    }
}
