use std::f32::consts::TAU;

use bevy_math::primitives::{
    BoxedPolygon, BoxedPolyline2d, BoxedPolyline3d, Capsule, Circle, Cone, ConicalFrustum, Cuboid,
    Cylinder, Direction2d, Direction3d, Ellipse, Line2d, Line3d, Plane2d, Plane3d, Polygon,
    Polyline2d, Polyline3d, Primitive2d, Primitive3d, Rectangle, RegularPolygon, Segment2d,
    Segment3d, Sphere, Torus, Triangle2d,
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

pub struct Direction2dDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Direction2d> for Direction2dDetails {}

impl<'s> GizmoPrimitive2d<Direction2d, Direction2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Direction2d, detail: Direction2dDetails) {
        let Direction2dDetails {
            position: start,
            color,
        } = detail;
        let dir = primitive;
        let end = start + *dir;
        self.arrow_2d(start, end, color);
    }
}

// Circle 2D

pub struct Circle2dDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Circle> for Circle2dDetails {}

impl<'s> GizmoPrimitive2d<Circle, Circle2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Circle, detail: Circle2dDetails) {
        let Circle2dDetails { position, color } = detail;
        self.circle_2d(position, primitive.radius, color);
    }
}

// Ellipse 2D

pub struct Ellipse2dDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Ellipse> for Ellipse2dDetails {}

impl<'s> GizmoPrimitive2d<Ellipse, Ellipse2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Ellipse, detail: Ellipse2dDetails) {
        let Ellipse2dDetails { position, color } = detail;
        self.ellipse_2d(position, primitive.half_width, primitive.half_height, color);
    }
}

// Line 2D

pub struct Line2dDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Line2d> for Line2dDetails {}

impl<'s> GizmoPrimitive2d<Line2d, Line2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Line2d, detail: Line2dDetails) {
        let Line2dDetails { position, color } = detail;
        self.primitive_2d(primitive.direction, Direction2dDetails { position, color });
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

pub struct Plane2dDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Plane2d> for Plane2dDetails {}

impl<'s> GizmoPrimitive2d<Plane2d, Plane2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Plane2d, detail: Plane2dDetails) {
        let Plane2dDetails { position, color } = detail;
        self.primitive_2d(primitive.normal, Direction2dDetails { position, color });
        let plane_line = Line2d {
            direction: Direction2d::from_normalized(primitive.normal.perp()),
        };
        self.primitive_2d(plane_line, Line2dDetails { position, color });
    }
}

// Segment 2D

pub struct Segment2dDetails {
    pub position: Vec2,
    pub color: Color,
}
impl PrimitiveDetailFor<Segment2d> for Segment2dDetails {}

impl<'s> GizmoPrimitive2d<Segment2d, Segment2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Segment2d, detail: Segment2dDetails) {
        let Segment2dDetails { position, color } = detail;
        let start = position - *primitive.direction * primitive.half_length;
        let end = position + *primitive.direction * primitive.half_length;
        self.arrow_2d(start, end, color);
    }
}

// Polyline 2D

pub struct Polyline2dDetails {
    pub position: Vec2,
    pub rotation: Vec2,
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polyline2d<N>> for Polyline2dDetails {}

impl<'s, const N: usize> GizmoPrimitive2d<Polyline2d<N>, Polyline2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Polyline2d<N>, detail: Polyline2dDetails) {
        let Polyline2dDetails {
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
            Polyline2dDetails {
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
            Polyline2dDetails {
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
                    let angle = i as f32 * TAU / segments as f32;
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

// Direction 3D

pub struct Direction3dDetails {
    pub position: Vec3,
    pub color: Color,
}
impl PrimitiveDetailFor<Direction3d> for Direction3dDetails {}

impl<'s> GizmoPrimitive3d<Direction3d, Direction3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Direction3d, detail: Direction3dDetails) {
        let Direction3dDetails {
            position: start,
            color,
        } = detail;
        let dir = primitive;
        let end = start + *dir;
        self.arrow(start, end, color);
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

// Plane 3D

pub struct Plane3dDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
}
impl PrimitiveDetailFor<Plane3d> for Plane3dDetails {}

impl<'s> GizmoPrimitive3d<Plane3d, Plane3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Plane3d, detail: Plane3dDetails) {
        let Plane3dDetails {
            position,
            rotation,
            color,
        } = detail;
        let normal = rotation * *primitive.normal;
        self.arrow(position, position + normal, color);
        let ortho = normal.any_orthonormal_vector();
        (0..4)
            .map(|i| i as f32 * 0.25 * 360.0)
            .map(f32::to_radians)
            .map(|angle| Quat::from_axis_angle(normal, angle))
            .for_each(|quat| {
                let dir = quat * ortho;
                (0..)
                    .filter(|i| i % 2 == 0)
                    .map(|i| [i, i + 1])
                    .map(|percents| percents.map(|p| p as f32 * 0.25 * dir))
                    .map(|vs| vs.map(|v| v + position))
                    .take(3)
                    .for_each(|[start, end]| {
                        self.line(start, end, color);
                    });
            });
    }
}

// Line 3D

pub struct Line3dDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
}
impl PrimitiveDetailFor<Line3d> for Line3dDetails {}

impl<'s> GizmoPrimitive3d<Line3d, Line3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Line3d, detail: Line3dDetails) {
        let Line3dDetails {
            position,
            rotation,
            color,
        } = detail;
        let dir = rotation * *primitive.direction;
        self.arrow(position, position + dir, color);
        [1.0, -1.0].into_iter().for_each(|sign| {
            self.line(
                position,
                position + sign * dir.clamp_length(1000.0, 1000.0),
                color,
            );
        });
    }
}

// Segment 3D

pub struct Segment3dDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
}
impl PrimitiveDetailFor<Segment3d> for Segment3dDetails {}

impl<'s> GizmoPrimitive3d<Segment3d, Segment3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Segment3d, detail: Segment3dDetails) {
        let Segment3dDetails {
            position,
            rotation,
            color,
        } = detail;
        let dir = rotation * *primitive.direction;
        let start = position - dir * primitive.half_length;
        let end = position + dir * primitive.half_length;
        self.line(start, end, color);
    }
}

// Polyline 3D

pub struct Polyline3dDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polyline3d<N>> for Polyline3dDetails {}

impl<'s, const N: usize> GizmoPrimitive3d<Polyline3d<N>, Polyline3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Polyline3d<N>, detail: Polyline3dDetails) {
        let Polyline3dDetails {
            position,
            rotation,
            color,
        } = detail;
        self.linestrip(primitive.vertices.map(|v| rotation * v + position), color);
    }
}

// BoxedPolyline 3D

pub struct BoxedPolyline3dDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
}
impl PrimitiveDetailFor<BoxedPolyline3d> for BoxedPolyline3dDetails {}

impl<'s> GizmoPrimitive3d<BoxedPolyline3d, BoxedPolyline3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: BoxedPolyline3d, detail: BoxedPolyline3dDetails) {
        let BoxedPolyline3dDetails {
            position,
            rotation,
            color,
        } = detail;
        self.linestrip(
            primitive.vertices.iter().map(|v| rotation * *v + position),
            color,
        );
    }
}

// Cuboid 3D

pub struct Cuboid3dDetails {
    pub position: Vec3,
    pub rotation: Quat,
    pub color: Color,
}
impl PrimitiveDetailFor<Cuboid> for Cuboid3dDetails {}

impl<'s> GizmoPrimitive3d<Cuboid, Cuboid3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cuboid, detail: Cuboid3dDetails) {
        let Cuboid3dDetails {
            position,
            rotation,
            color,
        } = detail;
        // NOTE: half extends sould probably be a UVec3 similarly the Rectangle should probably use
        // UVec2 to prevent negative sizes
        let [x, y, z] = primitive.half_extents.to_array();

        let vertices @ [a, b, c, d, e, f, g, h] = [
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
        ]
        .map(|[sx, sy, sz]| Vec3::new(sx * x, sy * y, sz * z) + position);

        let upper = [a, b, c, d]
            .into_iter()
            .zip([a, b, c, d].into_iter().cycle().skip(1));

        let lower = [e, f, g, h]
            .into_iter()
            .zip([e, f, g, h].into_iter().cycle().skip(1));

        let connections = vertices.into_iter().zip(vertices.into_iter().skip(4));

        upper
            .chain(lower)
            .chain(connections)
            .for_each(|(start, end)| {
                self.line(start, end, color);
            });
    }
}

// Cylinder 3D

pub struct Cylinder3dDetails {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Color,
    pub segments: usize,
}
impl PrimitiveDetailFor<Cylinder> for Cylinder3dDetails {}

impl<'s> GizmoPrimitive3d<Cylinder, Cylinder3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cylinder, detail: Cylinder3dDetails) {
        let Cylinder3dDetails {
            position,
            normal,
            color,
            segments,
        } = detail;
        let Cylinder {
            radius,
            half_height,
        } = primitive;

        fn cylinder_lines(
            radius: f32,
            segments: usize,
            half_height: f32,
            normal: Vec3,
            position: Vec3,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
            (0..segments + 1).map(move |i| {
                let angle = i as f32 * TAU / segments as f32;
                let (x, y) = angle.sin_cos();
                let p_2d = Vec2::new(x, y) * radius;
                [1.0, -1.0]
                    .map(|sign| sign * half_height)
                    .map(|height| p_2d.extend(height))
                    .map(|v| rotation * v + position)
            })
        }

        self.circle(position + half_height * normal, normal, radius, color);
        self.circle(position - half_height * normal, -normal, radius, color);

        cylinder_lines(radius, segments, half_height, normal, position).for_each(|[start, end]| {
            self.line(start, end, color);
        });
    }
}

// Capsule 3D

pub struct Capsule3dDetails {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Color,
    pub segments: usize,
}
impl PrimitiveDetailFor<Capsule> for Capsule3dDetails {}

impl<'s> GizmoPrimitive3d<Capsule, Capsule3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Capsule, detail: Capsule3dDetails) {
        let Capsule3dDetails {
            position,
            normal,
            color,
            segments,
        } = detail;
        let Capsule {
            radius,
            half_length,
        } = primitive;

        self.primitive_3d(
            Cylinder {
                radius,
                half_height: half_length,
            },
            Cylinder3dDetails {
                position,
                normal,
                color,
                segments,
            },
        );
        [1.0, -1.0].into_iter().for_each(|sign| {
            self.primitive_3d(
                Sphere { radius },
                SphereDetails {
                    position: position + sign * (normal * half_length),
                    rotation: Quat::from_rotation_arc(Vec3::Z, normal),
                    color,
                },
            );
        });
    }
}

// Cone 3D

pub struct Cone3dDetails {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Color,
    pub segments: usize,
}
impl PrimitiveDetailFor<Cone> for Cone3dDetails {}

impl<'s> GizmoPrimitive3d<Cone, Cone3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cone, detail: Cone3dDetails) {
        let Cone3dDetails {
            position,
            normal,
            color,
            segments,
        } = detail;
        let Cone { radius, height } = primitive;

        fn cone_lines(
            radius: f32,
            segments: usize,
            height: f32,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            let circle_points = (0..segments + 1).map(|i| [i, i + 1]).map(move |is| {
                is.map(|i| {
                    let angle = i as f32 * TAU / segments as f32;
                    let (x, y) = angle.sin_cos();
                    let p2d = Vec2::new(x, y) * radius;
                    p2d.extend(0.0)
                })
            });
            let cone_pointy_lines = (0..segments + 1).map(move |i| {
                let angle = i as f32 * TAU / segments as f32;
                let (x, y) = angle.sin_cos();
                let p2d = Vec2::new(x, y) * radius;
                let p3d = p2d.extend(0.0);
                [p3d, Vec2::ZERO.extend(height)]
            });
            circle_points.chain(cone_pointy_lines)
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        cone_lines(radius, segments, height)
            .map(|ps| ps.map(|p| rotation * p + position))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}

// ConicalFrustum 3D

pub struct ConicalFrustum3dDetails {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Color,
    pub segments: usize,
}
impl PrimitiveDetailFor<ConicalFrustum> for ConicalFrustum3dDetails {}

impl<'s> GizmoPrimitive3d<ConicalFrustum, ConicalFrustum3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: ConicalFrustum, detail: ConicalFrustum3dDetails) {
        let ConicalFrustum3dDetails {
            position,
            normal,
            color,
            segments,
        } = detail;
        let ConicalFrustum {
            radius_top,
            radius_bottom,
            height,
        } = primitive;

        fn cone_frustum_lines(
            radius_bottom: f32,
            radius_top: f32,
            segments: usize,
            height: f32,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            let top_circle_points = (0..segments + 1).map(|i| [i, i + 1]).map(move |is| {
                is.map(|i| {
                    let angle = i as f32 * TAU / segments as f32;
                    let (x, y) = angle.sin_cos();
                    let p2d = Vec2::new(x, y) * radius_top;
                    p2d.extend(height)
                })
            });
            let bottom_circle_points = (0..segments + 1).map(|i| [i, i + 1]).map(move |is| {
                is.map(|i| {
                    let angle = i as f32 * TAU / segments as f32;
                    let (x, y) = angle.sin_cos();
                    let p2d = Vec2::new(x, y) * radius_bottom;
                    p2d.extend(0.0)
                })
            });

            let connecting_lines = (0..segments + 1)
                .map(move |i| {
                    let angle = i as f32 * TAU / segments as f32;
                    let (x, y) = angle.sin_cos();
                    let p2d = Vec2::new(x, y) * radius_top;
                    p2d.extend(height)
                })
                .zip((0..segments + 1).map(move |i| {
                    let angle = i as f32 * TAU / segments as f32;
                    let (x, y) = angle.sin_cos();
                    let p2d = Vec2::new(x, y) * radius_bottom;
                    p2d.extend(0.0)
                }))
                .map(|(start, end)| [start, end]);

            top_circle_points
                .chain(bottom_circle_points)
                .chain(connecting_lines)
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        cone_frustum_lines(radius_bottom, radius_top, segments, height)
            .map(|ps| ps.map(|p| rotation * p + position))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}

// Torus 3D

pub struct Torus3dDetails {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Color,
    pub minor_segments: usize,
    pub major_segments: usize,
}
impl PrimitiveDetailFor<Torus> for Torus3dDetails {}

impl<'s> GizmoPrimitive3d<Torus, Torus3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Torus, detail: Torus3dDetails) {
        let Torus3dDetails {
            position,
            normal,
            color,
            minor_segments,
            major_segments,
        } = detail;
        let Torus {
            minor_radius,
            major_radius,
        } = primitive;

        fn torus_big_rings(
            minor_radius: f32,
            major_radius: f32,
            segments: usize,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            [
                (major_radius - minor_radius, 0.0),
                (major_radius + minor_radius, 0.0),
                (major_radius, minor_radius),
                (major_radius, -minor_radius),
            ]
            .into_iter()
            .flat_map(move |(radius, height)| {
                (0..segments + 1).map(|i| [i, i + 1]).map(move |is| {
                    is.map(|i| {
                        let angle = i as f32 * TAU / segments as f32;
                        let (x, y) = angle.sin_cos();
                        let p2d = Vec2::new(x, y) * radius;
                        p2d.extend(height)
                    })
                })
            })
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        let big_rings = torus_big_rings(minor_radius, major_radius, major_segments);

        let small_rings = (0..major_segments + 1).flat_map(move |i| {
            let angle = i as f32 * TAU / major_segments as f32;
            let (x, y) = angle.sin_cos();
            let pos_2d = Vec2::new(x, y) * major_radius;
            let pos_3d = pos_2d.extend(0.0);
            let normal_3d = pos_2d.perp().extend(0.0);
            let rotation = Quat::from_rotation_arc(Vec3::Z, normal_3d);
            (0..minor_segments + 1).map(|i| [i, i + 1]).map(move |is| {
                is.map(|i| {
                    let angle = i as f32 * TAU / minor_segments as f32;
                    let (x, y) = angle.sin_cos();
                    let p2d = Vec2::new(x, y) * minor_radius;
                    p2d.extend(0.0)
                })
                .map(|p| rotation * p + pos_3d)
            })
        });
        big_rings
            .chain(small_rings)
            .map(move |ps| ps.map(|p| rotation * p + position))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}
// note that I'm not sure wether the last few impls are optimal yet. I just kinda
// hacked this together to have something which is working
