use bevy_math::primitives::{
    Circle, Direction2d, Ellipse, Line2d, Plane2d, Primitive2d, Primitive3d, Segment2d, Sphere,
};
use bevy_math::{Quat, Vec2, Vec3};
use bevy_render::color::Color;

use crate::prelude::Gizmos;

pub trait PrimitiveDetailFor<P> {}

pub trait GizmoPrimitive2d<P: Primitive2d, D: PrimitiveDetailFor<P>> {
    fn primitive_2d(&mut self, primitive: P, detail: D);
}

pub trait GizmoPrimitive3d<P: Primitive3d, D: PrimitiveDetailFor<P>> {
    fn primitive_3d(&mut self, primitive: P, detail: D);
}

// Direction 2D

struct DirectionDetails {
    position: Vec2,
    color: Color,
}
impl PrimitiveDetailFor<Direction2d> for DirectionDetails {}

impl<'s> GizmoPrimitive2d<Direction2d, DirectionDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Direction2d, detail: DirectionDetails) {
        let DirectionDetails {
            position: start,
            color,
        } = detail;
        let dir = primitive;
        let end = start + *dir;
        self.line_2d(start, end, color);
        draw_arrow_head(self, end, dir, color);
    }
}

// Circle 2D

struct CircleDetails {
    position: Vec2,
    color: Color,
}
impl PrimitiveDetailFor<Circle> for CircleDetails {}

impl<'s> GizmoPrimitive2d<Circle, CircleDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Circle, detail: CircleDetails) {
        let CircleDetails { position, color } = detail;
        self.circle_2d(position, primitive.radius, color);
    }
}

// Ellipse 2D

struct EllipseDetails {
    position: Vec2,
    color: Color,
}
impl PrimitiveDetailFor<Ellipse> for EllipseDetails {}

impl<'s> GizmoPrimitive2d<Ellipse, EllipseDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Ellipse, detail: EllipseDetails) {
        let EllipseDetails { position, color } = detail;
        self.ellipse_2d(position, primitive.half_width, primitive.half_height, color);
    }
}

// Line 2D

struct LineDetails {
    position: Vec2,
    color: Color,
}
impl PrimitiveDetailFor<Line2d> for LineDetails {}

impl<'s> GizmoPrimitive2d<Line2d, LineDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Line2d, detail: LineDetails) {
        let LineDetails { position, color } = detail;
        self.primitive_2d(primitive.direction, DirectionDetails { position, color });
        let line_dir = primitive.direction;
        [1.0, -1.0].into_iter().for_each(|sign| {
            self.line_2d(
                position,
                position + sign * line_dir.clamp_length(1000.0, 1000.0),
                color,
            );
        });
    }
}

// Plane 2D

struct PlaneDetails {
    position: Vec2,
    color: Color,
}
impl PrimitiveDetailFor<Plane2d> for PlaneDetails {}

impl<'s> GizmoPrimitive2d<Plane2d, PlaneDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Plane2d, detail: PlaneDetails) {
        let PlaneDetails { position, color } = detail;
        self.primitive_2d(primitive.normal, DirectionDetails { position, color });
        let plane_line = Line2d {
            direction: Direction2d::from_normalized(primitive.normal.perp()),
        };
        self.primitive_2d(plane_line, LineDetails { position, color });
    }
}

// Segment 2D

struct SegmentDetails {
    position: Vec2,
    color: Color,
}
impl PrimitiveDetailFor<Segment2d> for SegmentDetails {}

impl<'s> GizmoPrimitive2d<Segment2d, SegmentDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Segment2d, detail: SegmentDetails) {
        let SegmentDetails { position, color } = detail;
        self.line_2d(
            position - *primitive.direction * primitive.half_length,
            position + *primitive.direction * primitive.half_length,
            color,
        );
        draw_arrow_head(self, position, primitive.direction, color);
    }
}

// Sphere 3D

struct SphereDetails {
    position: Vec3,
    rotation: Quat,
    color: Color,
}
impl PrimitiveDetailFor<Sphere> for SphereDetails {}

impl<'s> GizmoPrimitive3d<Sphere, SphereDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Sphere, detail: SphereDetails) {
        let SphereDetails {
            position,
            rotation,
            color,
        } = detail;
        self.sphere(position, rotation, primitive.radius, color);
    }
}

// helper function to draw an arrow head, ">" of "->"
fn draw_arrow_head(gizmos: &mut Gizmos, position: Vec2, direction: Direction2d, color: Color) {
    [1.0, -1.0]
        .map(|sign| 180.0 + sign * 45.0)
        .map(f32::to_radians)
        .map(Vec2::from_angle)
        .map(|angle| direction.rotate(angle).clamp_length_max(0.2))
        .into_iter()
        .for_each(|dir| {
            gizmos.line_2d(position, position + dir, color);
        });
}
