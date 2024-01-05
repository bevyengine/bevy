use std::f32::consts::TAU;

use bevy_math::primitives::{
    BoxedPolygon, BoxedPolyline2d, BoxedPolyline3d, Capsule, Circle, Cone, ConicalFrustum, Cuboid,
    Cylinder, Direction2d, Direction3d, Ellipse, Line2d, Line3d, Plane2d, Plane3d, Polygon,
    Polyline2d, Polyline3d, Primitive2d, Primitive3d, Rectangle, RegularPolygon, Segment2d,
    Segment3d, Sphere, Torus, Triangle2d,
};
use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::color::Color;

use crate::prelude::Gizmos;

/// The [`PrimitiveDetailFor`] trait serves as a marker trait, indicating that a implementor `D` is
/// intended to provide additional details specific to the geometric primitive `P`. This allows the
/// [`Gizmos`] to associate specific information with each primitive when rendering.
pub trait PrimitiveDetailFor<P> {}

/// A trait for rendering 2D geometric primitives (`P`) with associated details (`D`) with [`Gizmos`].
pub trait GizmoPrimitive2d<P: Primitive2d, D: PrimitiveDetailFor<P>> {
    /// Renders a 2D primitive with its associated details.
    fn primitive_2d(&mut self, primitive: P, detail: D);
}

/// A trait for rendering 3D geometric primitives (`P`) with associated details (`D`) with [`Gizmos`].
pub trait GizmoPrimitive3d<P: Primitive3d, D: PrimitiveDetailFor<P>> {
    /// Renders a 3D primitive with its associated details.
    fn primitive_3d(&mut self, primitive: P, detail: D);
}

// Direction 2D

/// Extra data used to draw [`Direction2d`] via [`Gizmos`]
pub struct Direction2dDetails {
    /// position of the start of the arrow
    pub position: Vec2,
    /// color of the arrow
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

/// Details for rendering a 2D circle via [`Gizmos`].
pub struct Circle2dDetails {
    /// Position of the center of the circle.
    pub center: Vec2,
    /// Color of the circle.
    pub color: Color,
}
impl PrimitiveDetailFor<Circle> for Circle2dDetails {}

impl<'s> GizmoPrimitive2d<Circle, Circle2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Circle, detail: Circle2dDetails) {
        let Circle2dDetails { center, color } = detail;
        self.circle_2d(center, primitive.radius, color);
    }
}

// Ellipse 2D

/// Details for rendering a 2D ellipse via [`Gizmos`].
pub struct Ellipse2dDetails {
    /// Position of the center of the ellipse.
    pub center: Vec2,
    /// Color of the ellipse.
    pub color: Color,
}
impl PrimitiveDetailFor<Ellipse> for Ellipse2dDetails {}

impl<'s> GizmoPrimitive2d<Ellipse, Ellipse2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Ellipse, detail: Ellipse2dDetails) {
        let Ellipse2dDetails { center, color } = detail;
        self.ellipse_2d(center, primitive.half_width, primitive.half_height, color);
    }
}

// Line 2D

/// Details for rendering a 2D line via [`Gizmos`].
pub struct Line2dDetails {
    /// Starting position of the line.
    pub start_position: Vec2,
    /// Color of the line.
    pub color: Color,
}
impl PrimitiveDetailFor<Line2d> for Line2dDetails {}

impl<'s> GizmoPrimitive2d<Line2d, Line2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Line2d, detail: Line2dDetails) {
        let Line2dDetails {
            start_position,
            color,
        } = detail;
        self.primitive_2d(
            primitive.direction,
            Direction2dDetails {
                position: start_position,
                color,
            },
        );
        let line_dir = primitive.direction;
        [1.0, -1.0].into_iter().for_each(|sign| {
            self.line_2d(
                start_position,
                start_position + sign * line_dir.clamp_length(1000.0, 1000.0),
                color,
            );
        });
    }
}

// Plane 2D

/// Details for rendering a 2D plane via [`Gizmos`].
pub struct Plane2dDetails {
    /// Starting position of the normal of the plane.
    pub normal_position: Vec2,
    /// Color of the plane.
    pub color: Color,
}
impl PrimitiveDetailFor<Plane2d> for Plane2dDetails {}

impl<'s> GizmoPrimitive2d<Plane2d, Plane2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Plane2d, detail: Plane2dDetails) {
        let Plane2dDetails {
            normal_position,
            color,
        } = detail;
        // normal
        let normal_details = Direction2dDetails {
            position: normal_position,
            color,
        };
        self.primitive_2d(primitive.normal, normal_details);

        // plane line
        let direction = Direction2d::from_normalized(primitive.normal.perp());
        let plane_line = Line2d { direction };
        let plane_line_details = Line2dDetails {
            start_position: normal_position,
            color,
        };
        self.primitive_2d(plane_line, plane_line_details);
    }
}

// Segment 2D

/// Details for rendering a 2D line segment via [`Gizmos`].
pub struct Segment2dDetails {
    /// Starting position of the line segment.
    pub start_position: Vec2,
    /// Color of the line segment.
    pub color: Color,
}
impl PrimitiveDetailFor<Segment2d> for Segment2dDetails {}

impl<'s> GizmoPrimitive2d<Segment2d, Segment2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Segment2d, detail: Segment2dDetails) {
        let Segment2dDetails {
            start_position,
            color,
        } = detail;
        let start = start_position;
        let end = start_position + *primitive.direction * 2.0 * primitive.half_length;
        self.line_2d(start, end, color);
    }
}

/// Details for rendering a 2D line segment via [`Gizmos`].
pub struct Segment2dArrowDetails {
    /// Starting position of the line segment.
    pub start_position: Vec2,
    /// Color of the line segment.
    pub color: Color,
}
impl PrimitiveDetailFor<Segment2d> for Segment2dArrowDetails {}

impl<'s> GizmoPrimitive2d<Segment2d, Segment2dArrowDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Segment2d, detail: Segment2dArrowDetails) {
        let Segment2dArrowDetails {
            start_position,
            color,
        } = detail;
        let start = start_position;
        let end = start_position + *primitive.direction * 2.0 * primitive.half_length;
        self.arrow_2d(start, end, color);
    }
}

// Polyline 2D

/// Details for rendering a 2D polyline via [`Gizmos`].
pub struct Polyline2dDetails {
    /// Offset for all the vertices of the polyline. If the polyline starts at `Vec2::ZERO`, this is
    /// also the starting point of the polyline.
    pub translation: Vec2,
    /// Rotation of the polyline around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientaion.
    pub rotation: f32,
    /// Color of the polyline.
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polyline2d<N>> for Polyline2dDetails {}

impl<'s, const N: usize> GizmoPrimitive2d<Polyline2d<N>, Polyline2dDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Polyline2d<N>, detail: Polyline2dDetails) {
        let Polyline2dDetails {
            translation,
            rotation,
            color,
        } = detail;
        self.linestrip_2d(
            primitive
                .vertices
                .into_iter()
                .map(rotate_then_translate_2d(rotation, translation)),
            color,
        );
    }
}

// BoxedPolyline 2D
//
// not sure here yet, maybe we should use a reference to some of the primitives instead since
// cloning all the vertices for drawing might defeat its purpose if we pass in the primitive by
// value

/// Details for rendering a 2D boxed polyline via [`Gizmos`].
pub struct BoxedPolylineDetails {
    /// Offset for all the vertices of the boxed polyline. If the polyline starts at `Vec2::ZERO`, this is
    /// also the starting point of the polyline.
    pub translation: Vec2,
    /// Rotation of the boxed polyline around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientation.
    pub rotation: f32,
    /// Color of the boxed polyline.
    pub color: Color,
}
impl PrimitiveDetailFor<BoxedPolyline2d> for BoxedPolylineDetails {}

impl<'s> GizmoPrimitive2d<BoxedPolyline2d, BoxedPolylineDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: BoxedPolyline2d, detail: BoxedPolylineDetails) {
        let BoxedPolylineDetails {
            translation,
            rotation,
            color,
        } = detail;
        self.linestrip_2d(
            primitive
                .vertices
                .iter()
                .map(|v| rotate_then_translate_2d(rotation, translation)(*v)),
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

/// Details for rendering a 2D triangle via [`Gizmos`].
pub struct TriangleDetails {
    /// Offset for all the vertices of the triangle. If the triangle starts at `Vec2::ZERO`, this is
    /// also the starting point of the triangle.
    pub translation: Vec2,
    /// Rotation of the triangle around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientation.
    pub rotation: f32,
    /// Color of the triangle.
    pub color: Color,
}
impl PrimitiveDetailFor<Triangle2d> for TriangleDetails {}

impl<'s> GizmoPrimitive2d<Triangle2d, TriangleDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Triangle2d, detail: TriangleDetails) {
        let TriangleDetails {
            translation,
            rotation,
            color,
        } = detail;
        let [a, b, c] = primitive.vertices;
        self.primitive_2d(
            Polyline2d {
                vertices: [a, b, c, a],
            },
            Polyline2dDetails {
                translation,
                rotation,
                color,
            },
        );
    }
}

// Rectangle 2D

/// Details for rendering a 2D rectangle via [`Gizmos`].
pub struct RectangleDetails {
    /// Offset for all the vertices of the rectangle. If the rectangle starts at `Vec2::ZERO`, this is
    /// also the starting point of the rectangle.
    pub translation: Vec2,
    /// Rotation of the rectangle around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientation.
    pub rotation: f32,
    /// Color of the rectangle.
    pub color: Color,
}
impl PrimitiveDetailFor<Rectangle> for RectangleDetails {}

impl<'s> GizmoPrimitive2d<Rectangle, RectangleDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Rectangle, detail: RectangleDetails) {
        let RectangleDetails {
            translation,
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
                translation,
                rotation,
                color,
            },
        );
    }
}

// Polygon 2D

/// Details for rendering a 2D polygon via [`Gizmos`].
pub struct PolygonDetails {
    /// Offset for all the vertices of the polygon. If the polygon starts at `Vec2::ZERO`, this is
    /// also the starting point of the polygon.
    pub translation: Vec2,
    /// Rotation of the polygon around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientation.
    pub rotation: f32,
    /// Color of the polygon.
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polygon<N>> for PolygonDetails {}

impl<'s, const N: usize> GizmoPrimitive2d<Polygon<N>, PolygonDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: Polygon<N>, detail: PolygonDetails) {
        let PolygonDetails {
            translation,
            rotation,
            color,
        } = detail;
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
                .into_iter()
                .chain(closing_point)
                .map(rotate_then_translate_2d(rotation, translation)),
            color,
        );
    }
}

// BoxedPolygon 2D

/// Details for rendering a 2D boxed polygon via [`Gizmos`].
pub struct BoxedPolygonDetails {
    /// Offset for all the vertices of the boxed polygon. If the boxed polygon starts at
    /// `Vec2::ZERO`, this is also the starting point of the boxed polygon.
    pub translation: Vec2,
    /// Rotation of the boxed polygon around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientation.
    pub rotation: f32,
    /// Color of the boxed polygon.
    pub color: Color,
}
impl PrimitiveDetailFor<BoxedPolygon> for BoxedPolygonDetails {}

impl<'s> GizmoPrimitive2d<BoxedPolygon, BoxedPolygonDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: BoxedPolygon, detail: BoxedPolygonDetails) {
        let BoxedPolygonDetails {
            translation,
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
                .map(|vertex| rotate_then_translate_2d(rotation, translation)(vertex)),
            color,
        );
    }
}

// RegularPolygon 2D

/// Details for rendering a 2D regular polygon via [`Gizmos`].
pub struct RegularPolygonDetails {
    /// Offset for all the vertices of the regular polygon. If the regular polygon starts at `Vec2::ZERO`, this is
    /// also the starting point of the regular polygon.
    pub translation: Vec2,
    /// Rotation of the regular polygon around the origin (`Vec2::ZERO`) given in radians with ccw
    /// orientation.
    pub rotation: f32,
    /// Color of the regular polygon.
    pub color: Color,
}

impl Default for RegularPolygonDetails {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            color: Color::default(),
        }
    }
}

impl PrimitiveDetailFor<RegularPolygon> for RegularPolygonDetails {}

impl<'s> GizmoPrimitive2d<RegularPolygon, RegularPolygonDetails> for Gizmos<'s> {
    fn primitive_2d(&mut self, primitive: RegularPolygon, detail: RegularPolygonDetails) {
        let RegularPolygonDetails {
            translation,
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
            .map(|vertices| vertices.map(rotate_then_translate_2d(rotation, translation)))
            .for_each(|[start, end]| {
                self.line_2d(start, end, color);
            });
    }
}

// Direction 3D

/// Details for rendering a 3D direction arrow via [`Gizmos`].
pub struct Direction3dDetails {
    /// Starting position of the arrow in 3D space.
    pub position: Vec3,
    /// Color of the arrow.
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

/// Details for rendering a 3D sphere via [`Gizmos`].
pub struct SphereDetails {
    /// Center position of the sphere in 3D space.
    pub center: Vec3,
    /// Rotation of the sphere around the origin (`Vec3::ZERO`).
    pub rotation: Quat,
    /// Color of the sphere.
    pub color: Color,
    /// Number of segments used to approximate the sphere geometry.
    pub segments: usize,
}
impl PrimitiveDetailFor<Sphere> for SphereDetails {}

impl<'s> GizmoPrimitive3d<Sphere, SphereDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Sphere, detail: SphereDetails) {
        let SphereDetails {
            center,
            rotation,
            color,
            segments,
        } = detail;
        let Sphere { radius } = primitive;
        let base_circle = circle_lines(radius, segments).map(|ps| ps.map(|p| p.extend(0.0)));
        let vertical_circles = [-1.0, 1.0].into_iter().flat_map(|sign| {
            circle_coordinates(radius, segments).flat_map(move |start| {
                shortest_arc_3d(
                    start.extend(0.0),
                    sign * radius * Vec3::Z,
                    Vec3::ZERO,
                    segments,
                )
            })
        });

        base_circle
            .chain(vertical_circles)
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
            .for_each(|[start, end]| self.line(start, end, color));
    }
}

// Plane 3D

/// Details for rendering a 3D plane via [`Gizmos`].
pub struct Plane3dDetails {
    /// Position of the point on the plane from which the normal emanates.
    pub normal_position: Vec3,
    /// Rotation of the plane around the origin (`Vec3::ZERO`).
    pub rotation: Quat,
    /// Color of the plane.
    pub color: Color,
}
impl PrimitiveDetailFor<Plane3d> for Plane3dDetails {}

impl<'s> GizmoPrimitive3d<Plane3d, Plane3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Plane3d, detail: Plane3dDetails) {
        let Plane3dDetails {
            normal_position,
            rotation,
            color,
        } = detail;
        let normal = rotation * *primitive.normal;
        self.arrow(normal_position, normal_position + normal, color);
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
                    .map(|vs| vs.map(|v| v + normal_position))
                    .take(3)
                    .for_each(|[start, end]| {
                        self.line(start, end, color);
                    });
            });
    }
}

// Line 3D

/// Details for rendering a 3D line via [`Gizmos`].
pub struct Line3dDetails {
    /// Starting position of the line.
    pub start_position: Vec3,
    /// Rotation of the line around the origin (`Vec3::ZERO`).
    pub rotation: Quat,
    /// Color of the line.
    pub color: Color,
}
impl PrimitiveDetailFor<Line3d> for Line3dDetails {}

impl<'s> GizmoPrimitive3d<Line3d, Line3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Line3d, detail: Line3dDetails) {
        let Line3dDetails {
            start_position,
            rotation,
            color,
        } = detail;
        let dir = rotation * *primitive.direction;
        self.arrow(start_position, start_position + dir, color);
        [1.0, -1.0].into_iter().for_each(|sign| {
            self.line(
                start_position,
                start_position + sign * dir.clamp_length(1000.0, 1000.0),
                color,
            );
        });
    }
}

// Segment 3D

/// Details for rendering a 3D line segment via [`Gizmos`].
pub struct Segment3dDetails {
    /// Starting position of the line segment.
    pub start_position: Vec3,
    /// Rotation of the line segment around the origin (`Vec3::ZERO`).
    pub rotation: Quat,
    /// Color of the line segment.
    pub color: Color,
}
impl PrimitiveDetailFor<Segment3d> for Segment3dDetails {}

impl<'s> GizmoPrimitive3d<Segment3d, Segment3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Segment3d, detail: Segment3dDetails) {
        let Segment3dDetails {
            start_position,
            rotation,
            color,
        } = detail;
        let dir = rotation * *primitive.direction;
        let start = start_position;
        let end = start_position + dir * 2.0 * primitive.half_length;
        self.line(start, end, color);
    }
}

// Polyline 3D

/// Details for rendering a 3D polyline via [`Gizmos`].
pub struct Polyline3dDetails {
    /// Translation applied to all vertices of the polyline.
    pub translation: Vec3,
    /// Rotation of the polyline around the origin (`Vec3::ZERO`) given as a quaternion.
    pub rotation: Quat,
    /// Color of the polyline.
    pub color: Color,
}
impl<const N: usize> PrimitiveDetailFor<Polyline3d<N>> for Polyline3dDetails {}

impl<'s, const N: usize> GizmoPrimitive3d<Polyline3d<N>, Polyline3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Polyline3d<N>, detail: Polyline3dDetails) {
        let Polyline3dDetails {
            translation,
            rotation,
            color,
        } = detail;
        self.linestrip(
            primitive
                .vertices
                .map(rotate_then_translate_3d(rotation, translation)),
            color,
        );
    }
}

// BoxedPolyline 3D

/// Details for rendering a 3D boxed polyline via [`Gizmos`].
pub struct BoxedPolyline3dDetails {
    /// Translation applied to all vertices of the enclosed polyline.
    pub translation: Vec3,
    /// Rotation of the polyline around the origin (`Vec3::ZERO`) given as a quaternion.
    pub rotation: Quat,
    /// Color of the polyline and the enclosing box.
    pub color: Color,
}
impl PrimitiveDetailFor<BoxedPolyline3d> for BoxedPolyline3dDetails {}

impl<'s> GizmoPrimitive3d<BoxedPolyline3d, BoxedPolyline3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: BoxedPolyline3d, detail: BoxedPolyline3dDetails) {
        let BoxedPolyline3dDetails {
            translation,
            rotation,
            color,
        } = detail;
        self.linestrip(
            primitive
                .vertices
                .iter()
                .copied()
                .map(rotate_then_translate_3d(rotation, translation)),
            color,
        );
    }
}

// Cuboid 3D

/// Details for rendering a 3D cuboid via [`Gizmos`].
pub struct Cuboid3dDetails {
    /// Center position of the cuboid.
    pub center: Vec3,
    /// Rotation of the cuboid around its center given as a quaternion.
    pub rotation: Quat,
    /// Color of the cuboid.
    pub color: Color,
}
impl PrimitiveDetailFor<Cuboid> for Cuboid3dDetails {}

impl<'s> GizmoPrimitive3d<Cuboid, Cuboid3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cuboid, detail: Cuboid3dDetails) {
        let Cuboid3dDetails {
            center,
            rotation,
            color,
        } = detail;
        // NOTE: half extends sould probably be a UVec3 similarly the Rectangle should probably use
        // UVec2 to prevent negative sizes
        let [half_extend_x, half_extend_y, half_extend_z] = primitive.half_extents.to_array();

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
        .map(|[sx, sy, sz]| Vec3::new(sx * half_extend_x, sy * half_extend_y, sz * half_extend_z))
        .map(rotate_then_translate_3d(rotation, center));

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

/// Details for rendering a 3D cylinder via [`Gizmos`].
pub struct Cylinder3dDetails {
    /// Center position of the cylinder.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the cylinder.
    pub normal: Vec3,
    /// Color of the cylinder.
    pub color: Color,
    /// Number of segments used to approximate the cylinder geometry.
    pub segments: usize,
}
impl PrimitiveDetailFor<Cylinder> for Cylinder3dDetails {}

impl<'s> GizmoPrimitive3d<Cylinder, Cylinder3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cylinder, detail: Cylinder3dDetails) {
        let Cylinder3dDetails {
            center,
            normal,
            color,
            segments,
        } = detail;
        let Cylinder {
            radius,
            half_height,
        } = primitive;

        fn cylinder_vertical_lines(
            radius: f32,
            segments: usize,
            half_height: f32,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            circle_coordinates(radius, segments).map(move |point_2d| {
                [1.0, -1.0]
                    .map(|sign| sign * half_height)
                    .map(|height| point_2d.extend(height))
            })
        }

        fn cylinder_circle_lines(
            radius: f32,
            segments: usize,
            half_height: f32,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            circle_lines(radius, segments)
                .chain(circle_coordinates(radius, segments).map(|p| [p, Vec2::ZERO]))
                .map(move |ps| ps.map(|p| p.extend(half_height)))
        }

        let top_lines = cylinder_circle_lines(radius, segments, half_height);
        let bottom_lines = cylinder_circle_lines(radius, segments, -half_height);
        let vertical_lines = cylinder_vertical_lines(radius, segments, half_height);

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        top_lines
            .chain(bottom_lines)
            .chain(vertical_lines)
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}

// Capsule 3D

/// Details for rendering a 3D capsule via [`Gizmos`].
pub struct Capsule3dDetails {
    /// Center position of the capsule.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the capsule.
    pub normal: Vec3,
    /// Color of the capsule.
    pub color: Color,
    /// Number of segments used to approximate the capsule geometry.
    pub segments: usize,
}
impl PrimitiveDetailFor<Capsule> for Capsule3dDetails {}

impl<'s> GizmoPrimitive3d<Capsule, Capsule3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Capsule, detail: Capsule3dDetails) {
        let Capsule3dDetails {
            center,
            normal,
            color,
            segments,
        } = detail;
        let Capsule {
            radius,
            half_length,
        } = primitive;

        fn cylinder_vertical_lines(
            radius: f32,
            segments: usize,
            half_height: f32,
        ) -> impl Iterator<Item = [Vec3; 2]> {
            circle_coordinates(radius, segments).map(move |point_2d| {
                [1.0, -1.0]
                    .map(|sign| sign * half_height)
                    .map(|height| point_2d.extend(height))
            })
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

        let caps = [1.0, -1.0].into_iter().flat_map(|sign| {
            circle_coordinates(radius, segments).flat_map(move |start| {
                shortest_arc_3d(
                    start.extend(sign * half_length),
                    sign * (half_length + radius) * Vec3::Z,
                    sign * half_length * Vec3::Z,
                    segments,
                )
            })
        });

        let vertical_lines = cylinder_vertical_lines(radius, segments, half_length);

        let circle_lines = [1.0, -1.0].into_iter().flat_map(|sign| {
            circle_lines(radius, segments).map(move |ps| ps.map(|p| p.extend(sign * half_length)))
        });

        caps.chain(circle_lines)
            .chain(vertical_lines)
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}

// Cone 3D

/// Details for rendering a 3D cone via [`Gizmos`].
pub struct Cone3dDetails {
    /// Center of the base of the cone.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the cone.
    pub normal: Vec3,
    /// Color of the cone.
    pub color: Color,
    /// Number of segments used to approximate the cone geometry.
    pub segments: usize,
}
impl PrimitiveDetailFor<Cone> for Cone3dDetails {}

impl<'s> GizmoPrimitive3d<Cone, Cone3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cone, detail: Cone3dDetails) {
        let Cone3dDetails {
            center,
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
            let circle_points = circle_lines(radius, segments).map(|ps| ps.map(|p| p.extend(0.0)));
            let cone_pointy_lines = circle_coordinates(radius, segments)
                .map(move |p| [p.extend(0.0), Vec2::ZERO.extend(height)]);
            circle_points.chain(cone_pointy_lines)
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        cone_lines(radius, segments, height)
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}

// ConicalFrustum 3D

/// Details for rendering a 3D conical frustum via [`Gizmos`].
pub struct ConicalFrustum3dDetails {
    /// Center of the base circle of the conical frustum.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the conical frustum.
    pub normal: Vec3,
    /// Color of the conical frustum.
    pub color: Color,
    /// Number of segments used to approximate the curved surfaces.
    pub segments: usize,
}
impl PrimitiveDetailFor<ConicalFrustum> for ConicalFrustum3dDetails {}

impl<'s> GizmoPrimitive3d<ConicalFrustum, ConicalFrustum3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: ConicalFrustum, detail: ConicalFrustum3dDetails) {
        let ConicalFrustum3dDetails {
            center,
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
            let top_circle_points =
                circle_lines(radius_top, segments).map(move |ps| ps.map(|p| p.extend(height)));
            let bottom_circle_points =
                circle_lines(radius_bottom, segments).map(move |ps| ps.map(|p| p.extend(0.0)));

            let connecting_lines = circle_coordinates(radius_top, segments)
                .map(move |p| p.extend(height))
                .zip(circle_coordinates(radius_bottom, segments).map(|p| p.extend(0.0)))
                .map(|(start, end)| [start, end]);

            top_circle_points
                .chain(bottom_circle_points)
                .chain(connecting_lines)
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        cone_frustum_lines(radius_bottom, radius_top, segments, height)
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}

// Torus 3D

/// Details for rendering a 3D torus via [`Gizmos`].
pub struct Torus3dDetails {
    /// Center of the torus.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the torus.
    pub normal: Vec3,
    /// Color of the torus.
    pub color: Color,
    /// Number of segments in the minor (tube) direction.
    pub minor_segments: usize,
    /// Number of segments in the major (ring) direction.
    pub major_segments: usize,
}
impl PrimitiveDetailFor<Torus> for Torus3dDetails {}

impl<'s> GizmoPrimitive3d<Torus, Torus3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Torus, detail: Torus3dDetails) {
        let Torus3dDetails {
            center,
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
                circle_lines(radius, segments).map(move |ps| ps.map(|p| p.extend(height)))
            })
        }

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        let big_rings = torus_big_rings(minor_radius, major_radius, major_segments);

        let small_rings = circle_coordinates(major_radius, major_segments).flat_map(|p| {
            let translation = p.extend(0.0);
            let normal_3d = p.perp().extend(0.0);
            let rotation = Quat::from_rotation_arc(Vec3::Z, normal_3d);
            circle_lines(minor_radius, minor_segments).map(move |ps| {
                ps.map(|p| p.extend(0.0))
                    .map(rotate_then_translate_3d(rotation, translation))
            })
        });

        big_rings
            .chain(small_rings)
            .map(move |ps| ps.map(rotate_then_translate_3d(rotation, center)))
            .for_each(|[start, end]| {
                self.line(start, end, color);
            });
    }
}
// note that I'm not sure wether the last few impls are optimal yet. I just kinda
// hacked this together to have something which is working

// helpers - affine transform

fn rotate_then_translate_2d(rotation: f32, translation: Vec2) -> impl Fn(Vec2) -> Vec2 {
    move |v| Mat2::from_angle(rotation).mul_vec2(v) + translation
}

fn rotate_then_translate_3d(rotation: Quat, translation: Vec3) -> impl Fn(Vec3) -> Vec3 {
    move |v| rotation * v + translation
}

// helpers - circle related things

fn single_circle_coordinate(radius: f32, segments: usize, nth_point: usize, fraction: f32) -> Vec2 {
    let angle = nth_point as f32 * TAU * fraction / segments as f32;
    let (x, y) = angle.sin_cos();
    Vec2::new(x, y) * radius
}

fn circle_coordinates(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..)
        .map(move |p| single_circle_coordinate(radius, segments, p, 1.0))
        .take(segments)
}

fn circle_lines(radius: f32, segments: usize) -> impl Iterator<Item = [Vec2; 2]> {
    (0..)
        .map(|p| [p, p + 1])
        .map(move |ps| {
            ps.map(move |nth_point| single_circle_coordinate(radius, segments, nth_point, 1.0))
        })
        .take(segments)
}

// helpers - arc

// this draws the shortest arc between two points in 3d given a center of the arc. Since it's the
// shortest arc, that means that:
//   - any arc spanning more than PI will be inverted to it's shorter counter part the other way
//   around
//   - the arc spanning exactly PI is kind of ambigious since it could go both ways around the
//   circle depending on how the angle is determined internally
//
// In those situations, calculate another point which is located on the arc and use the `arc_3d`
// function instead
fn shortest_arc_3d(
    start: Vec3,
    end: Vec3,
    center: Vec3,
    segments: usize,
) -> impl Iterator<Item = [Vec3; 2]> {
    // https://math.stackexchange.com/a/329816

    let u = start - center;
    let v = end - center;
    let alpha = u.angle_between(v);

    (0..)
        .map(|p| [p, p + 1])
        .map(move |ps| {
            ps.map(|k| (k as f32 * alpha) / segments as f32)
                .map(|theta| center + ((alpha - theta).sin() * u + theta.sin() * v) / alpha.sin())
        })
        .take(segments)
}

fn arc_3d(
    start: Vec3,
    middle: Vec3,
    end: Vec3,
    center: Vec3,
    segments: usize,
) -> impl Iterator<Item = [Vec3; 2]> {
    shortest_arc_3d(start, middle, center, segments)
        .chain(shortest_arc_3d(middle, end, center, segments))
}
