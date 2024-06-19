//! A module for rendering each of the 2D [`bevy_math::primitives`] with [`Gizmos`].

use std::f32::consts::PI;

use super::helpers::*;

use bevy_color::Color;
use bevy_math::primitives::{
    Annulus, Arc2d, BoxedPolygon, BoxedPolyline2d, Capsule2d, Circle, CircularSector,
    CircularSegment, Ellipse, Line2d, Plane2d, Polygon, Polyline2d, Primitive2d, Rectangle,
    RegularPolygon, Rhombus, Segment2d, Triangle2d,
};
use bevy_math::{Dir2, Mat2, Vec2};

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
        primitive: &P,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_>;
}

// direction 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Dir2> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self : 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Dir2,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let direction = Mat2::from_angle(angle) * **primitive;

        let start = position;
        let end = position + MIN_LINE_LEN * direction;
        self.arrow_2d(start, end, color);
    }
}

// arc 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Arc2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Arc2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.arc_2d(
            position,
            angle,
            primitive.half_angle * 2.0,
            primitive.radius,
            color,
        );
    }
}

// circle 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Circle> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = crate::circles::Ellipse2dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Circle,
        position: Vec2,
        _angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.circle_2d(position, primitive.radius, color)
    }
}

// circular sector 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<CircularSector> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &CircularSector,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let color = color.into();

        // we need to draw the arc part of the sector, and the two lines connecting the arc and the center
        self.arc_2d(
            position,
            angle,
            primitive.arc.half_angle * 2.0,
            primitive.arc.radius,
            color,
        );

        let start = position
            + primitive.arc.radius * Mat2::from_angle(angle - primitive.arc.half_angle) * Vec2::Y;
        let end = position
            + primitive.arc.radius * Mat2::from_angle(angle + primitive.arc.half_angle) * Vec2::Y;
        self.line_2d(position, start, color);
        self.line_2d(position, end, color);
    }
}

// circular segment 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<CircularSegment> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &CircularSegment,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let color = color.into();

        // we need to draw the arc part of the segment, and the line connecting the two ends
        self.arc_2d(
            position,
            angle,
            primitive.arc.half_angle * 2.0,
            primitive.arc.radius,
            color,
        );

        let start = position
            + primitive.arc.radius * Mat2::from_angle(angle - primitive.arc.half_angle) * Vec2::Y;
        let end = position
            + primitive.arc.radius * Mat2::from_angle(angle + primitive.arc.half_angle) * Vec2::Y;
        self.line_2d(end, start, color);
    }
}

// ellipse 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Ellipse> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = crate::circles::Ellipse2dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_2d<'a>(
        &mut self,
        primitive: &Ellipse,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.ellipse_2d(position, angle, primitive.half_size, color)
    }
}

// annulus 2d

/// Builder for configuring the drawing options of [`Annulus`].
pub struct Annulus2dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,
    position: Vec2,
    inner_radius: f32,
    outer_radius: f32,
    color: Color,
    inner_resolution: u32,
    outer_resolution: u32,
}

impl<Config, Clear> Annulus2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments for each circle of the annulus.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.outer_resolution = resolution;
        self.inner_resolution = resolution;
        self
    }

    /// Set the number of line-segments for the outer circle of the annulus.
    pub fn outer_resolution(mut self, resolution: u32) -> Self {
        self.outer_resolution = resolution;
        self
    }

    /// Set the number of line-segments for the inner circle of the annulus.
    pub fn inner_resolution(mut self, resolution: u32) -> Self {
        self.inner_resolution = resolution;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Annulus> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Annulus2dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Annulus,
        position: Vec2,
        _angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Annulus2dBuilder {
            gizmos: self,
            position,
            inner_radius: primitive.inner_circle.radius,
            outer_radius: primitive.outer_circle.radius,
            color: color.into(),
            inner_resolution: crate::circles::DEFAULT_CIRCLE_RESOLUTION,
            outer_resolution: crate::circles::DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for Annulus2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let Annulus2dBuilder {
            gizmos,
            position,
            inner_radius,
            outer_radius,
            inner_resolution,
            outer_resolution,
            color,
            ..
        } = self;

        gizmos
            .circle_2d(*position, *outer_radius, *color)
            .resolution(*outer_resolution);
        gizmos
            .circle_2d(*position, *inner_radius, *color)
            .resolution(*inner_resolution);
    }
}

// rhombus 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Rhombus> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Rhombus,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        };
        let [a, b, c, d] =
            [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    primitive.half_diagonals.x * sign_x,
                    primitive.half_diagonals.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(rotate_then_translate_2d(angle, position));
        self.linestrip_2d(positions, color);
    }
}

// capsule 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Capsule2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Capsule2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        let polymorphic_color: Color = color.into();

        if !self.enabled {
            return;
        }

        // transform points from the reference unit square to capsule "rectangle"
        let [top_left, top_right, bottom_left, bottom_right, top_center, bottom_center] = [
            [-1.0, 1.0],
            [1.0, 1.0],
            [-1.0, -1.0],
            [1.0, -1.0],
            // just reuse the pipeline for these points as well
            [0.0, 1.0],
            [0.0, -1.0],
        ]
        .map(|[sign_x, sign_y]| Vec2::X * sign_x + Vec2::Y * sign_y)
        .map(|reference_point| {
            let scaling = Vec2::X * primitive.radius + Vec2::Y * primitive.half_length;
            reference_point * scaling
        })
        .map(rotate_then_translate_2d(angle, position));

        // draw left and right side of capsule "rectangle"
        self.line_2d(bottom_left, top_left, polymorphic_color);
        self.line_2d(bottom_right, top_right, polymorphic_color);

        let start_angle_top = angle;
        let start_angle_bottom = PI + angle;

        // draw arcs
        self.arc_2d(
            top_center,
            start_angle_top,
            PI,
            primitive.radius,
            polymorphic_color,
        );
        self.arc_2d(
            bottom_center,
            start_angle_bottom,
            PI,
            primitive.radius,
            polymorphic_color,
        );
    }
}

// line 2d
//
/// Builder for configuring the drawing options of [`Line2d`].
pub struct Line2dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    direction: Dir2, // Direction of the line

    position: Vec2, // position of the center of the line
    rotation: Mat2, // rotation of the line
    color: Color,   // color of the line

    draw_arrow: bool, // decides whether to indicate the direction of the line with an arrow
}

impl<Config, Clear> Line2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the drawing mode of the line (arrow vs. plain line)
    pub fn draw_arrow(mut self, is_enabled: bool) -> Self {
        self.draw_arrow = is_enabled;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Line2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Line2dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Line2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Line2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            position,
            rotation: Mat2::from_angle(angle),
            color: color.into(),
            draw_arrow: false,
        }
    }
}

impl<Config, Clear> Drop for Line2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let direction = self.rotation * *self.direction;

        let [start, end] = [1.0, -1.0]
            .map(|sign| sign * INFINITE_LEN)
            // offset the line from the origin infinitely into the given direction
            .map(|length| direction * length)
            // translate the line to the given position
            .map(|offset| self.position + offset);

        self.gizmos.line_2d(start, end, self.color);

        // optionally draw an arrow head at the center of the line
        if self.draw_arrow {
            self.gizmos.arrow_2d(
                self.position - direction * MIN_LINE_LEN,
                self.position,
                self.color,
            );
        }
    }
}

// plane 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Plane2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Plane2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        let polymorphic_color: Color = color.into();

        if !self.enabled {
            return;
        }
        let rotation = Mat2::from_angle(angle);

        // draw normal of the plane (orthogonal to the plane itself)
        let normal = primitive.normal;
        let normal_segment = Segment2d {
            direction: normal,
            half_length: HALF_MIN_LINE_LEN,
        };
        self.primitive_2d(
            &normal_segment,
            // offset the normal so it starts on the plane line
            position + HALF_MIN_LINE_LEN * rotation * *normal,
            angle,
            polymorphic_color,
        )
        .draw_arrow(true);

        // draw the plane line
        let direction = Dir2::new_unchecked(-normal.perp());
        self.primitive_2d(&Line2d { direction }, position, angle, polymorphic_color)
            .draw_arrow(false);

        // draw an arrow such that the normal is always left side of the plane with respect to the
        // planes direction. This is to follow the "counter-clockwise" convention
        self.arrow_2d(
            position,
            position + MIN_LINE_LEN * (rotation * *direction),
            polymorphic_color,
        );
    }
}

// segment 2d

/// Builder for configuring the drawing options of [`Segment2d`].
pub struct Segment2dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    direction: Dir2,  // Direction of the line segment
    half_length: f32, // Half-length of the line segment

    position: Vec2, // position of the center of the line segment
    rotation: Mat2, // rotation of the line segment
    color: Color,   // color of the line segment

    draw_arrow: bool, // decides whether to draw just a line or an arrow
}

impl<Config, Clear> Segment2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the drawing mode of the line (arrow vs. plain line)
    pub fn draw_arrow(mut self, is_enabled: bool) -> Self {
        self.draw_arrow = is_enabled;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Segment2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Segment2dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Segment2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Segment2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            half_length: primitive.half_length,

            position,
            rotation: Mat2::from_angle(angle),
            color: color.into(),

            draw_arrow: Default::default(),
        }
    }
}

impl<Config, Clear> Drop for Segment2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
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

impl<'w, 's, const N: usize, Config, Clear> GizmoPrimitive2d<Polyline2d<N>>
    for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Polyline2d<N>,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .map(rotate_then_translate_2d(angle, position)),
            color,
        );
    }
}

// boxed polyline 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<BoxedPolyline2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &BoxedPolyline2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .map(rotate_then_translate_2d(angle, position)),
            color,
        );
    }
}

// triangle 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Triangle2d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Triangle2d,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }
        let [a, b, c] = primitive.vertices;
        let positions = [a, b, c, a].map(rotate_then_translate_2d(angle, position));
        self.linestrip_2d(positions, color);
    }
}

// rectangle 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<Rectangle> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Rectangle,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
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
        let positions = [a, b, c, d, a].map(rotate_then_translate_2d(angle, position));
        self.linestrip_2d(positions, color);
    }
}

// polygon 2d

impl<'w, 's, const N: usize, Config, Clear> GizmoPrimitive2d<Polygon<N>>
    for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Polygon<N>,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        // Check if the polygon needs a closing point
        let closing_point = {
            let first = primitive.vertices.first();
            (primitive.vertices.last() != first)
                .then_some(first)
                .flatten()
                .cloned()
        };

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(rotate_then_translate_2d(angle, position)),
            color,
        );
    }
}

// boxed polygon 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<BoxedPolygon> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &BoxedPolygon,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let closing_point = {
            let first = primitive.vertices.first();
            (primitive.vertices.last() != first)
                .then_some(first)
                .flatten()
                .cloned()
        };
        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(rotate_then_translate_2d(angle, position)),
            color,
        );
    }
}

// regular polygon 2d

impl<'w, 's, Config, Clear> GizmoPrimitive2d<RegularPolygon> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &RegularPolygon,
        position: Vec2,
        angle: f32,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let points = (0..=primitive.sides)
            .map(|p| single_circle_coordinate(primitive.circumcircle.radius, primitive.sides, p))
            .map(rotate_then_translate_2d(angle, position));
        self.linestrip_2d(points, color);
    }
}
