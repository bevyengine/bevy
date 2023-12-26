use bevy_math::primitives::{
    BoxedPolygon, BoxedPolyline2d, Circle, Direction2d, Ellipse, Line2d, Plane2d, Polygon,
    Polyline2d, Primitive2d, Primitive3d, Rectangle, RegularPolygon, Segment2d, Sphere, Triangle2d,
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

pub struct DirectionDetails {
    pub position: Vec2,
    pub color: Color,
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

pub struct CircleDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Circle> for CircleDetails {}

impl<'s> GizmoPrimitive2d<Circle, CircleDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Circle, detail: CircleDetails) {
        let CircleDetails { position, color } = detail;
        self.circle_2d(position, primitive.radius, color);
    }
}

// Ellipse 2D

pub struct EllipseDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Ellipse> for EllipseDetails {}

impl<'s> GizmoPrimitive2d<Ellipse, EllipseDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Ellipse, detail: EllipseDetails) {
        let EllipseDetails { position, color } = detail;
        self.ellipse_2d(position, primitive.half_width, primitive.half_height, color);
    }
}

// Line 2D

pub struct LineDetails {
    pub position: Vec2,
    pub color: Color,
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

pub struct PlaneDetails {
    pub position: Vec2,
    pub color: Color,
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

pub struct SegmentDetails {
    pub position: Vec2,
    pub color: Color,
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

// Polyline 2D

pub struct PolylineDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polyline2d<N>> for PolylineDetails {}

impl<'s, const N: usize> GizmoPrimitive2d<Polyline2d<N>, PolylineDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Polyline2d<N>, detail: PolylineDetails) {
        let PolylineDetails {
            position,
            rotation,
            color,
        } = detail;
        self.linestrip_2d(
            primitive
                .vertices
                .into_iter()
                .map(|vertex| vertex.rotate(rotation) + position),
            color,
        );
    }
}

// BoxedPolyline 2D
//
// not sure here yet, maybe we should use a reference to some of the primitives instead since
// cloning all the vertices for drawing might defeat its purpose if we pass in the primitive by
// value

pub struct BoxedPolylineDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<BoxedPolyline2d> for BoxedPolylineDetails {}

impl<'s> GizmoPrimitive2d<BoxedPolyline2d, BoxedPolylineDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: BoxedPolyline2d, detail: BoxedPolylineDetails) {
        let BoxedPolylineDetails {
            position,
            rotation,
            color,
        } = detail;
        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .map(|vertex| vertex.rotate(rotation) + position),
            color,
        );
    }
}

// Triangle 2D
//
// NOTE: to self: it's a bit ambigious at the moment how the shapes are defined. We should define /
// document somewhere whether polygons are expected to be closed/open by default. One other
// possibility would be to handle this via some extra struct and make the user specify what they
// provide, e.g.
//
// ```rust
// enum Closedness {
//   Closed,
//   Open
// }
// ```
//
// but then we would also write code in the hot paths of gizmos that optionally handle these
// configurations. It might be preemtive optimization to not go that route on the other hand as
// well
//
// For now I'll assume that all of the primitives are provided in an open configurations. That
// means that primitive.first != primitive.last

pub struct TriangleDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Triangle2d> for TriangleDetails {}

impl<'s> GizmoPrimitive2d<Triangle2d, TriangleDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Triangle2d, detail: TriangleDetails) {
        let TriangleDetails {
            position,
            rotation,
            color,
        } = detail;
        let [a, b, c] = primitive.vertices;
        self.primitive_2d(
            Polyline2d {
                vertices: [a, b, c, a],
            },
            PolylineDetails {
                position,
                rotation,
                color,
            },
        );
    }
}

// Rectangle 2D

pub struct RectangleDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Rectangle> for RectangleDetails {}

impl<'s> GizmoPrimitive2d<Rectangle, RectangleDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Rectangle, detail: RectangleDetails) {
        let RectangleDetails {
            position,
            rotation,
            color,
        } = detail;
        let [a, b, c, d] =
            [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    primitive.half_width * sign_x,
                    primitive.half_height * sign_y,
                )
            });
        self.primitive_2d(
            Polyline2d {
                vertices: [a, b, c, d, a],
            },
            PolylineDetails {
                position,
                rotation,
                color,
            },
        );
    }
}

// Polygon 2D

pub struct PolygonDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polygon<N>> for PolygonDetails {}

impl<'s, const N: usize> GizmoPrimitive2d<Polygon<N>, PolygonDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Polygon<N>, detail: PolygonDetails) {
        let PolygonDetails {
            position,
            rotation,
            color,
        } = detail;
        let closing_point = primitive.vertices.last().cloned();
        self.linestrip_2d(
            primitive
                .vertices
                .into_iter()
                .chain(closing_point)
                .map(|vertex| vertex.rotate(rotation) + position),
            color,
        );
    }
}

// BoxedPolygon 2D

pub struct BoxedPolygonDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<BoxedPolygon> for BoxedPolygonDetails {}

impl<'s> GizmoPrimitive2d<BoxedPolygon, BoxedPolygonDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: BoxedPolygon, detail: BoxedPolygonDetails) {
        let BoxedPolygonDetails {
            position,
            rotation,
            color,
        } = detail;
        let closing_point = primitive.vertices.last().cloned();
        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .cloned()
                .chain(closing_point)
                .map(|vertex| vertex.rotate(rotation) + position),
            color,
        );
    }
}

// RegularPolygon 2D

pub struct RegularPolygonDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}

impl Default for RegularPolygonDetails {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: Vec2::from_angle(0.0),
            color: Color::default(),
        }
    }
}

impl PrimitiveDetailFor<RegularPolygon> for RegularPolygonDetails {}

impl<'s> GizmoPrimitive2d<RegularPolygon, RegularPolygonDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: RegularPolygon, detail: RegularPolygonDetails) {
        let RegularPolygonDetails {
            position,
            rotation,
            color,
        } = detail;

        fn regular_polygon_inner(radius: f32, segments: usize) -> impl Iterator<Item = [Vec2; 2]> {
            (0..segments + 1).map(|i| [i, i + 1]).map(move |vals| {
                vals.map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / segments as f32;
                    let (x, y) = angle.sin_cos();
                    Vec2::new(x, y) * radius
                })
            })
        }

        regular_polygon_inner(primitive.circumcircle.radius, primitive.sides)
            .map(|vertices| vertices.map(|vertex| vertex.rotate(rotation) + position))
            .for_each(|[start, end]| {
                self.line_2d(start, end, color);
            });
    }
}

// Sphere 3D

pub struct SphereDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
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
