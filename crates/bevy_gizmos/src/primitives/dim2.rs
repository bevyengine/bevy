//! A module for rendering each of the 2D [`bevy_math::primitives`] with [`GizmoBuffer`].

use core::f32::consts::{FRAC_PI_2, PI};

use super::helpers::*;

use bevy_color::Color;
use bevy_math::{
    primitives::{
        Annulus, Arc2d, BoxedPolygon, BoxedPolyline2d, Capsule2d, Circle, CircularSector,
        CircularSegment, Ellipse, Line2d, Plane2d, Polygon, Polyline2d, Primitive2d, Rectangle,
        RegularPolygon, Rhombus, Segment2d, Triangle2d,
    },
    Dir2, Isometry2d, Rot2, Vec2,
};

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};

// some magic number since using directions as offsets will result in lines of length 1 pixel
const MIN_LINE_LEN: f32 = 50.0;
const HALF_MIN_LINE_LEN: f32 = 25.0;
// length used to simulate infinite lines
const INFINITE_LEN: f32 = 100_000.0;

/// A trait for rendering 2D geometric primitives (`P`) with [`GizmoBuffer`].
pub trait GizmoPrimitive2d<P: Primitive2d> {
    /// The output of `primitive_2d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 'a;

    /// Renders a 2D primitive with its associated details.
    fn primitive_2d(
        &mut self,
        primitive: &P,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_>;
}

// direction 2d

impl<Config, Clear> GizmoPrimitive2d<Dir2> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Dir2,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }
        let isometry = isometry.into();
        let start = Vec2::ZERO;
        let end = *primitive * MIN_LINE_LEN;
        self.arrow_2d(isometry * start, isometry * end, color);
    }
}

// arc 2d

impl<Config, Clear> GizmoPrimitive2d<Arc2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Arc2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();
        let start_iso = isometry * Isometry2d::from_rotation(Rot2::radians(-primitive.half_angle));

        self.arc_2d(
            start_iso,
            primitive.half_angle * 2.0,
            primitive.radius,
            color,
        );
    }
}

// circle 2d

impl<Config, Clear> GizmoPrimitive2d<Circle> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = crate::circles::Ellipse2dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Circle,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.circle_2d(isometry, primitive.radius, color)
    }
}

// circular sector 2d

impl<Config, Clear> GizmoPrimitive2d<CircularSector> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &CircularSector,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();
        let color = color.into();

        let start_iso =
            isometry * Isometry2d::from_rotation(Rot2::radians(-primitive.arc.half_angle));
        let end_iso = isometry * Isometry2d::from_rotation(Rot2::radians(primitive.arc.half_angle));

        // we need to draw the arc part of the sector, and the two lines connecting the arc and the center
        self.arc_2d(
            start_iso,
            primitive.arc.half_angle * 2.0,
            primitive.arc.radius,
            color,
        );

        let end_position = primitive.arc.radius * Vec2::Y;
        self.line_2d(isometry * Vec2::ZERO, start_iso * end_position, color);
        self.line_2d(isometry * Vec2::ZERO, end_iso * end_position, color);
    }
}

// circular segment 2d

impl<Config, Clear> GizmoPrimitive2d<CircularSegment> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &CircularSegment,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();
        let color = color.into();

        let start_iso =
            isometry * Isometry2d::from_rotation(Rot2::radians(-primitive.arc.half_angle));
        let end_iso = isometry * Isometry2d::from_rotation(Rot2::radians(primitive.arc.half_angle));

        // we need to draw the arc part of the segment, and the line connecting the two ends
        self.arc_2d(
            start_iso,
            primitive.arc.half_angle * 2.0,
            primitive.arc.radius,
            color,
        );

        let position = primitive.arc.radius * Vec2::Y;
        self.line_2d(start_iso * position, end_iso * position, color);
    }
}

// ellipse 2d

impl<Config, Clear> GizmoPrimitive2d<Ellipse> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = crate::circles::Ellipse2dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_2d<'a>(
        &mut self,
        primitive: &Ellipse,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.ellipse_2d(isometry, primitive.half_size, color)
    }
}

// annulus 2d

/// Builder for configuring the drawing options of [`Annulus`].
pub struct Annulus2dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry2d,
    inner_radius: f32,
    outer_radius: f32,
    color: Color,
    inner_resolution: u32,
    outer_resolution: u32,
}

impl<Config, Clear> Annulus2dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive2d<Annulus> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Annulus2dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Annulus,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Annulus2dBuilder {
            gizmos: self,
            isometry: isometry.into(),
            inner_radius: primitive.inner_circle.radius,
            outer_radius: primitive.outer_circle.radius,
            color: color.into(),
            inner_resolution: crate::circles::DEFAULT_CIRCLE_RESOLUTION,
            outer_resolution: crate::circles::DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for Annulus2dBuilder<'_, Config, Clear>
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
            isometry,
            inner_radius,
            outer_radius,
            inner_resolution,
            outer_resolution,
            color,
            ..
        } = self;

        gizmos
            .circle_2d(*isometry, *outer_radius, *color)
            .resolution(*outer_resolution);
        gizmos
            .circle_2d(*isometry, *inner_radius, *color)
            .resolution(*inner_resolution);
    }
}

// rhombus 2d

impl<Config, Clear> GizmoPrimitive2d<Rhombus> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Rhombus,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        };
        let isometry = isometry.into();
        let [a, b, c, d] =
            [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    primitive.half_diagonals.x * sign_x,
                    primitive.half_diagonals.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(|vec2| isometry * vec2);
        self.linestrip_2d(positions, color);
    }
}

// capsule 2d

impl<Config, Clear> GizmoPrimitive2d<Capsule2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Capsule2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        let isometry = isometry.into();
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
        .map(|vec2| isometry * vec2);

        // draw left and right side of capsule "rectangle"
        self.line_2d(bottom_left, top_left, polymorphic_color);
        self.line_2d(bottom_right, top_right, polymorphic_color);

        let start_angle_top = isometry.rotation.as_radians() - FRAC_PI_2;
        let start_angle_bottom = isometry.rotation.as_radians() + FRAC_PI_2;

        // draw arcs
        self.arc_2d(
            Isometry2d::new(top_center, Rot2::radians(start_angle_top)),
            PI,
            primitive.radius,
            polymorphic_color,
        );
        self.arc_2d(
            Isometry2d::new(bottom_center, Rot2::radians(start_angle_bottom)),
            PI,
            primitive.radius,
            polymorphic_color,
        );
    }
}

// line 2d
//
/// Builder for configuring the drawing options of [`Line2d`].
pub struct Line2dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

    direction: Dir2, // Direction of the line

    isometry: Isometry2d,
    color: Color, // color of the line

    draw_arrow: bool, // decides whether to indicate the direction of the line with an arrow
}

impl<Config, Clear> Line2dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive2d<Line2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Line2dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Line2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Line2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            isometry: isometry.into(),
            color: color.into(),
            draw_arrow: false,
        }
    }
}

impl<Config, Clear> Drop for Line2dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let [start, end] = [1.0, -1.0]
            .map(|sign| sign * INFINITE_LEN)
            // offset the line from the origin infinitely into the given direction
            .map(|length| self.direction * length)
            // transform the line with the given isometry
            .map(|offset| self.isometry * offset);

        self.gizmos.line_2d(start, end, self.color);

        // optionally draw an arrow head at the center of the line
        if self.draw_arrow {
            self.gizmos.arrow_2d(
                self.isometry * (-self.direction * MIN_LINE_LEN),
                self.isometry * Vec2::ZERO,
                self.color,
            );
        }
    }
}

// plane 2d

impl<Config, Clear> GizmoPrimitive2d<Plane2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Plane2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        let isometry = isometry.into();
        let polymorphic_color: Color = color.into();

        if !self.enabled {
            return;
        }
        // draw normal of the plane (orthogonal to the plane itself)
        let normal = primitive.normal;
        let normal_segment = Segment2d::from_direction_and_length(normal, HALF_MIN_LINE_LEN * 2.);
        self.primitive_2d(
            &normal_segment,
            // offset the normal so it starts on the plane line
            Isometry2d::new(isometry * (HALF_MIN_LINE_LEN * normal), isometry.rotation),
            polymorphic_color,
        )
        .draw_arrow(true);

        // draw the plane line
        let direction = Dir2::new_unchecked(-normal.perp());
        self.primitive_2d(&Line2d { direction }, isometry, polymorphic_color)
            .draw_arrow(false);

        // draw an arrow such that the normal is always left side of the plane with respect to the
        // planes direction. This is to follow the "counter-clockwise" convention
        self.arrow_2d(
            isometry * Vec2::ZERO,
            isometry * (MIN_LINE_LEN * direction),
            polymorphic_color,
        );
    }
}

// segment 2d

/// Builder for configuring the drawing options of [`Segment2d`].
pub struct Segment2dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

    point1: Vec2, // First point of the segment
    point2: Vec2, // Second point of the segment

    isometry: Isometry2d, // isometric transformation of the line segment
    color: Color,         // color of the line segment

    draw_arrow: bool, // decides whether to draw just a line or an arrow
}

impl<Config, Clear> Segment2dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive2d<Segment2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Segment2dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Segment2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Segment2dBuilder {
            gizmos: self,
            point1: primitive.point1(),
            point2: primitive.point2(),

            isometry: isometry.into(),
            color: color.into(),

            draw_arrow: Default::default(),
        }
    }
}

impl<Config, Clear> Drop for Segment2dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let segment = Segment2d::new(self.point1, self.point2).transformed(self.isometry);

        if self.draw_arrow {
            self.gizmos
                .arrow_2d(segment.point1(), segment.point2(), self.color);
        } else {
            self.gizmos
                .line_2d(segment.point1(), segment.point2(), self.color);
        }
    }
}

// polyline 2d

impl<const N: usize, Config, Clear> GizmoPrimitive2d<Polyline2d<N>> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Polyline2d<N>,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .map(|vec2| isometry * vec2),
            color,
        );
    }
}

// boxed polyline 2d

impl<Config, Clear> GizmoPrimitive2d<BoxedPolyline2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &BoxedPolyline2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .copied()
                .map(|vec2| isometry * vec2),
            color,
        );
    }
}

// triangle 2d

impl<Config, Clear> GizmoPrimitive2d<Triangle2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Triangle2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

        let [a, b, c] = primitive.vertices;
        let positions = [a, b, c, a].map(|vec2| isometry * vec2);
        self.linestrip_2d(positions, color);
    }
}

// rectangle 2d

impl<Config, Clear> GizmoPrimitive2d<Rectangle> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Rectangle,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

        let [a, b, c, d] =
            [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    primitive.half_size.x * sign_x,
                    primitive.half_size.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(|vec2| isometry * vec2);
        self.linestrip_2d(positions, color);
    }
}

// polygon 2d

impl<const N: usize, Config, Clear> GizmoPrimitive2d<Polygon<N>> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Polygon<N>,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

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
                .map(|vec2| isometry * vec2),
            color,
        );
    }
}

// boxed polygon 2d

impl<Config, Clear> GizmoPrimitive2d<BoxedPolygon> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &BoxedPolygon,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

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
                .map(|vec2| isometry * vec2),
            color,
        );
    }
}

// regular polygon 2d

impl<Config, Clear> GizmoPrimitive2d<RegularPolygon> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &RegularPolygon,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

        let points = (0..=primitive.sides)
            .map(|n| single_circle_coordinate(primitive.circumcircle.radius, primitive.sides, n))
            .map(|vec2| isometry * vec2);
        self.linestrip_2d(points, color);
    }
}
