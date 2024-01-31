//! A module for rendering each of the 2D [`bevy_math::primitives`] with [`Gizmos`].

use super::helpers::*;

use bevy_math::primitives::{
    BoxedPolygon, BoxedPolyline2d, Circle, Direction2d, Ellipse, Line2d, Plane2d, Polygon,
    Polyline2d, Primitive2d, Rectangle, RegularPolygon, Segment2d, Triangle2d,
};
use bevy_math::{Mat2, Vec2};
use bevy_render::color::Color;

use crate::prelude::{GizmoConfigGroup, Gizmos};

// some magic number since using directions as offsets will result in lines of length 1 pixel
const MIN_LINE_LEN: f32 = 50.0;
const HALF_MIN_LINE_LEN: f32 = 25.0;
// length used to simulate infinite lines
const INFINITE_LEN: f32 = 100_000.0;

/// A trait for rendering 2D geometric primitives (`P`) with [`Gizmos`].
pub trait GizmoPrimitive2d<P: Primitive2d> {
    /// The output of `primitive_2d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 'a;

    /// Renders a 2D primitive with its associated details.
    fn primitive_2d(
        &mut self,
        primitive: P,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_>;
}

// direction 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Direction2d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self : 'a;

    fn primitive_2d(
        &mut self,
        primitive: Direction2d,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let direction = rotation * *primitive;

        let start = position;
        let end = position + MIN_LINE_LEN * direction;
        self.arrow_2d(start, end, color);
    }
}

// circle 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Circle> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Circle,
        position: Vec2,
        _rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.circle_2d(position, primitive.radius, color);
    }
}

// ellipse 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Ellipse> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Ellipse,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.ellipse_2d(
            position,
            rotation,
            primitive.half_size.x,
            primitive.half_size.y,
            color,
        );
    }
}

// line 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Line2d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Line2d,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let direction = rotation * *primitive.direction;

        self.arrow_2d(position, position + direction * MIN_LINE_LEN, color);

        let [start, end] = [1.0, -1.0]
            .map(|sign| sign * INFINITE_LEN)
            .map(|length| direction * length)
            .map(|offset| position + offset);
        self.line_2d(start, end, color);
    }
}

// plane 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Plane2d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Plane2d,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        // normal
        let normal = rotation * *primitive.normal;
        let normal_segment = Segment2d {
            direction: Direction2d::new_unchecked(normal),
            half_length: HALF_MIN_LINE_LEN,
        };
        let normal_direction = rotation * normal;
        self.primitive_2d(
            normal_segment,
            position + HALF_MIN_LINE_LEN * normal_direction,
            rotation,
            color,
        )
        .draw_arrow(true);

        // plane line
        let direction = Direction2d::new_unchecked(normal.perp());
        self.primitive_2d(Line2d { direction }, position, rotation, color);
    }
}

// segment 2d

/// Builder for configuring the drawing options of [`Segment2d`].
pub struct Segment2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction2d, // Direction of the line segment
    half_length: f32,       // Half-length of the line segment

    position: Vec2,
    rotation: Mat2,
    color: Color,

    draw_arrow: bool, // decides whether to draw just a line or an arrow
}

impl<T: GizmoConfigGroup> Segment2dBuilder<'_, '_, '_, T> {
    /// Set the drawing mode of the line (arrow vs. plain line)
    pub fn draw_arrow(mut self, is_enabled: bool) -> Self {
        self.draw_arrow = is_enabled;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Segment2d> for Gizmos<'w, 's, T> {
    type Output<'a> = Segment2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Segment2d,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        Segment2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            half_length: primitive.half_length,

            position,
            rotation,
            color,

            draw_arrow: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Segment2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let direction = self.rotation * *self.direction;
        let start = self.position - direction * self.half_length;
        let end = self.position + direction * self.half_length;

        if self.draw_arrow {
            self.gizmos.arrow_2d(start, end, self.color);
        } else {
            self.gizmos.line_2d(start, end, self.color);
        }
    }
}

// polyline 2d

impl<'w, 's, const N: usize, T: GizmoConfigGroup> GizmoPrimitive2d<Polyline2d<N>>
    for Gizmos<'w, 's, T>
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Polyline2d<N>,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .map(rotate_then_translate_2d(rotation, position)),
            color,
        );
    }
}

// boxed polyline 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<BoxedPolyline2d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: BoxedPolyline2d,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .map(rotate_then_translate_2d(rotation, position)),
            color,
        );
    }
}

// triangle 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Triangle2d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Triangle2d,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }
        let [a, b, c] = primitive.vertices;
        let positions = [a, b, c, a].map(rotate_then_translate_2d(rotation, position));
        self.linestrip_2d(positions, color);
    }
}

// rectangle 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Rectangle> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Rectangle,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let [a, b, c, d] =
            [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    primitive.half_size.x * sign_x,
                    primitive.half_size.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(rotate_then_translate_2d(rotation, position));
        self.linestrip_2d(positions, color);
    }
}

// polygon 2d

impl<'w, 's, const N: usize, T: GizmoConfigGroup> GizmoPrimitive2d<Polygon<N>>
    for Gizmos<'w, 's, T>
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: Polygon<N>,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        // Check if the polygon needs a closing point
        let closing_point = {
            let last = primitive.vertices.last();
            (primitive.vertices.first() != last)
                .then_some(last)
                .flatten()
                .cloned()
        };

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(rotate_then_translate_2d(rotation, position)),
            color,
        );
    }
}

// boxed polygon 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<BoxedPolygon> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: BoxedPolygon,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let closing_point = {
            let last = primitive.vertices.last();
            (primitive.vertices.first() != last)
                .then_some(last)
                .flatten()
                .cloned()
        };
        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(rotate_then_translate_2d(rotation, position)),
            color,
        );
    }
}

// regular polygon 2d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<RegularPolygon> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: RegularPolygon,
        position: Vec2,
        rotation: Mat2,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let points = (0..=primitive.sides)
            .map(|p| {
                single_circle_coordinate(primitive.circumcircle.radius, primitive.sides, p, 1.0)
            })
            .map(rotate_then_translate_2d(rotation, position));
        self.linestrip_2d(points, color);
    }
}
