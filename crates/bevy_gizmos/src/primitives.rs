use std::f32::consts::{FRAC_PI_2, TAU};

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

/// A trait for rendering 3D geometric primitives (`P`) with associated details (`D`) with [`Gizmos`].
pub trait GizmoPrimitive3d<P: Primitive3d, D: PrimitiveDetailFor<P>> {
    /// Renders a 3D primitive with its associated details.
    fn primitive_3d(&mut self, primitive: P, detail: D);
}

// BoxedPolyline 2D

// NOTE: not sure here yet, maybe we should use a reference to some of the primitives instead since
// cloning all the vertices for drawing might defeat its purpose if we pass in the primitive by
// value

// ======== 3D ==========

// Direction 3D

/// Details for rendering a 3D direction arrow via [`Gizmos`].
#[derive(Default)]
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
    /// Number of segments used to approximate the sphere geometry. Defaults to `5`
    pub segments: usize,
}
impl PrimitiveDetailFor<Sphere> for SphereDetails {}

impl Default for SphereDetails {
    fn default() -> Self {
        Self {
            center: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
            segments: 5,
        }
    }
}

impl<'s> GizmoPrimitive3d<Sphere, SphereDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Sphere, detail: SphereDetails) {
        let SphereDetails {
            center,
            rotation,
            color,
            segments,
        } = detail;
        let Sphere { radius } = primitive;

        // draw two caps, one for the "upper half" and one for the "lower" half of the sphere
        [-1.0, 1.0]
            .into_iter()
            .for_each(|sign| draw_cap(self, radius, segments, rotation, center, sign, color));

        draw_circle(self, radius, segments, rotation, center, color);
    }
}

// Plane 3D

/// Details for rendering a 3D plane via [`Gizmos`].
#[derive(Default)]
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
#[derive(Default)]
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
#[derive(Default)]
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
#[derive(Default)]
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
#[derive(Default)]
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
#[derive(Default)]
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
    /// Normal vector indicating the orientation of the cylinder. Defaults to `Vec3::Z`
    pub normal: Vec3,
    /// Color of the cylinder.
    pub color: Color,
    /// Number of segments used to approximate the cylinder geometry. Defaults to `5`
    pub segments: usize,
}
impl PrimitiveDetailFor<Cylinder> for Cylinder3dDetails {}

impl Default for Cylinder3dDetails {
    fn default() -> Self {
        Self {
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

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

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

        [-1.0, 1.0].into_iter().for_each(|sign| {
            draw_circle(
                self,
                radius,
                segments,
                rotation,
                center + sign * half_height * normal,
                color,
            );
        });

        draw_cylinder_vertical_lines(self, radius, segments, half_height, rotation, center, color);
    }
}

// Capsule 3D

/// Details for rendering a 3D capsule via [`Gizmos`].
pub struct Capsule3dDetails {
    /// Center position of the capsule.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the capsule. Defaults to `Vec3::Z`
    pub normal: Vec3,
    /// Color of the capsule.
    pub color: Color,
    /// Number of segments used to approximate the capsule geometry. Defaults to `5`
    pub segments: usize,
}
impl PrimitiveDetailFor<Capsule> for Capsule3dDetails {}

impl Default for Capsule3dDetails {
    fn default() -> Self {
        Self {
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

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

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

        [1.0, -1.0].into_iter().for_each(|sign| {
            // use "-" here since rotation is ccw and otherwise the caps would face the wrong way
            // around
            let center = center - sign * half_length * normal;
            draw_cap(self, radius, segments, rotation, center, sign, color);
            draw_circle(self, radius, segments, rotation, center, color);
        });

        draw_cylinder_vertical_lines(self, radius, segments, half_length, rotation, center, color);
    }
}

// Cone 3D

/// Details for rendering a 3D cone via [`Gizmos`].
pub struct Cone3dDetails {
    /// Center of the base of the cone.
    pub center: Vec3,
    /// Normal vector indicating the orientation of the cone. Defaults to `Vec3::Z`
    pub normal: Vec3,
    /// Color of the cone.
    pub color: Color,
    /// Number of segments used to approximate the cone geometry. Defaults to `5`
    pub segments: usize,
}
impl PrimitiveDetailFor<Cone> for Cone3dDetails {}

impl Default for Cone3dDetails {
    fn default() -> Self {
        Self {
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

impl<'s> GizmoPrimitive3d<Cone, Cone3dDetails> for Gizmos<'s> {
    fn primitive_3d(&mut self, primitive: Cone, detail: Cone3dDetails) {
        let Cone3dDetails {
            center,
            normal,
            color,
            segments,
        } = detail;
        let Cone { radius, height } = primitive;

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

        draw_circle(self, radius, segments, rotation, center, color);

        circle_coordinates(radius, segments)
            .map(move |p| [p.extend(0.0), Vec2::ZERO.extend(height)])
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
    /// Normal vector indicating the orientation of the conical frustum. Defaults to `Vec3::Z`
    pub normal: Vec3,
    /// Color of the conical frustum.
    pub color: Color,
    /// Number of segments used to approximate the curved surfaces. Defaults to `5`
    pub segments: usize,
}
impl PrimitiveDetailFor<ConicalFrustum> for ConicalFrustum3dDetails {}

impl Default for ConicalFrustum3dDetails {
    fn default() -> Self {
        Self {
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

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

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        [(radius_top, height), (radius_bottom, 0.0)]
            .into_iter()
            .for_each(|(radius, height)| {
                draw_circle(
                    self,
                    radius,
                    segments,
                    rotation,
                    center + height * normal,
                    color,
                );
            });

        circle_coordinates(radius_top, segments)
            .map(move |p| p.extend(height))
            .zip(circle_coordinates(radius_bottom, segments).map(|p| p.extend(0.0)))
            .map(|(start, end)| [start, end])
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
    /// Normal vector indicating the orientation of the torus. Defaults to `Vec3::Z`
    pub normal: Vec3,
    /// Color of the torus.
    pub color: Color,
    /// Number of segments in the minor (tube) direction. Defaults to `5`
    pub minor_segments: usize,
    /// Number of segments in the major (ring) direction. Defaults to `5`
    pub major_segments: usize,
}
impl PrimitiveDetailFor<Torus> for Torus3dDetails {}

impl Default for Torus3dDetails {
    fn default() -> Self {
        Self {
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            minor_segments: 5,
            major_segments: 5,
        }
    }
}

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

        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

        [
            (major_radius - minor_radius, 0.0),
            (major_radius + minor_radius, 0.0),
            (major_radius, minor_radius),
            (major_radius, -minor_radius),
        ]
        .into_iter()
        .for_each(|(radius, height)| {
            draw_circle(
                self,
                radius,
                major_segments,
                rotation,
                center + height * normal,
                color,
            );
        });

        let affine = rotate_then_translate_3d(rotation, center);
        circle_coordinates(major_radius, major_segments)
            .flat_map(|p| {
                let translation = affine(p.extend(0.0));
                let dir_to_translation = (translation - center).normalize();
                let rotation_axis = normal.cross(dir_to_translation).normalize();
                [dir_to_translation, normal, -dir_to_translation, -normal]
                    .map(|dir| dir * minor_radius)
                    .map(|offset| translation + offset)
                    .map(|point| (point, translation, rotation_axis))
            })
            .for_each(|(from, center, rotation_axis)| {
                self.arc_3d(center, rotation_axis, from, FRAC_PI_2, minor_radius, color)
                    .segments(minor_segments);
            });
    }
}

/// A trait for rendering 2D geometric primitives (`P`) with [`Gizmos`].
pub trait GizmoPrimitive2d<'s, P: Primitive2d> {
    /// The output of `primitive_2d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 's,
        's: 'a;

    /// Renders a 2D primitive with its associated details.
    fn primitive_2d<'a>(&'s mut self, primitive: P) -> Self::Output<'a>;
}

// direction 2d

/// Builder for configuring the drawing options of [`Direction2d`].
pub struct Direction2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    direction: Direction2d, // direction the arrow points to

    position: Vec2, // position of the start of the arrow
    color: Color,   // color of the arrow
}

impl<'a, 's> Direction2dBuilder<'a, 's> {
    /// set the position of the start of the arrow
    pub fn position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// set the color of the arrow
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Direction2d> for Gizmos<'s> {
    type Output<'a> = Direction2dBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Direction2d) -> Self::Output<'a> {
        Direction2dBuilder {
            gizmos: self,
            direction: primitive,
            position: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for Direction2dBuilder<'_, '_> {
    fn drop(&mut self) {
        let start = self.position;
        let end = self.position + *self.direction;
        self.gizmos.arrow_2d(start, end, self.color);
    }
}

// circle 2d

/// Builder for configuring the drawing options of [`Circle`].
pub struct Circle2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    radius: f32, // 2D circle to be rendered

    center: Vec2, // position of the center of the circle
    color: Color, // color of the circle
}

impl<'a, 's> Circle2dBuilder<'a, 's> {
    /// Set the position of the center of the circle.
    pub fn center(mut self, center: Vec2) -> Self {
        self.center = center;
        self
    }

    /// Set the color of the circle.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Circle> for Gizmos<'s> {
    type Output<'a> = Circle2dBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Circle) -> Self::Output<'a> {
        Circle2dBuilder {
            gizmos: self,
            radius: primitive.radius,
            center: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for Circle2dBuilder<'_, '_> {
    fn drop(&mut self) {
        self.gizmos.circle_2d(self.center, self.radius, self.color);
    }
}

// ellipse 2d

/// Builder for configuring the drawing options of [`Ellipse`].
pub struct Ellipse2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    half_width: f32,  // Half-width of the ellipse
    half_height: f32, // Half-height of the ellipse

    center: Vec2, // Position of the center of the ellipse
    color: Color, // Color of the ellipse
}

impl<'a, 's> Ellipse2dBuilder<'a, 's> {
    /// Set the position of the center of the ellipse.
    pub fn center(mut self, center: Vec2) -> Self {
        self.center = center;
        self
    }

    /// Set the color of the ellipse.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Ellipse> for Gizmos<'s> {
    type Output<'a> = Ellipse2dBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Ellipse) -> Self::Output<'a> {
        Ellipse2dBuilder {
            gizmos: self,
            half_width: primitive.half_width,
            half_height: primitive.half_height,
            center: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for Ellipse2dBuilder<'_, '_> {
    fn drop(&mut self) {
        self.gizmos
            .ellipse_2d(self.center, self.half_width, self.half_height, self.color);
    }
}

// line 2d

/// Builder for configuring the drawing options of [`Line2d`].
pub struct Line2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    direction: Direction2d, // Direction of the line

    start_position: Vec2, // Starting position of the line
    color: Color,         // Color of the line
}

impl<'a, 's> Line2dBuilder<'a, 's> {
    /// Set the starting position of the line.
    pub fn start_position(mut self, start_position: Vec2) -> Self {
        self.start_position = start_position;
        self
    }

    /// Set the color of the line.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Line2d> for Gizmos<'s> {
    type Output<'a> = Line2dBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Line2d) -> Self::Output<'a> {
        Line2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            start_position: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for Line2dBuilder<'_, '_> {
    fn drop(&mut self) {
        let start = self.start_position;
        let end = self.start_position + *self.direction;
        self.gizmos.arrow_2d(start, end, self.color);

        [1.0, -1.0].into_iter().for_each(|sign| {
            self.gizmos.line_2d(
                self.start_position,
                self.start_position + sign * self.direction.clamp_length(1000.0, 1000.0),
                self.color,
            );
        });
    }
}

// plane 2d

/// Builder for configuring the drawing options of [`Plane2d`].
pub struct Plane2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    normal: Direction2d, // Normal of the plane

    normal_position: Vec2, // Starting position of the normal of the plane
    color: Color,          // Color of the plane
}

impl<'a, 's> Plane2dBuilder<'a, 's> {
    /// Set the starting position of the normal of the plane.
    pub fn normal_position(mut self, normal_position: Vec2) -> Self {
        self.normal_position = normal_position;
        self
    }

    /// Set the color of the plane.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Plane2d> for Gizmos<'s> {
    type Output<'a> = Plane2dBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Plane2d) -> Self::Output<'a> {
        Plane2dBuilder {
            gizmos: self,
            normal: primitive.normal,
            normal_position: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for Plane2dBuilder<'_, '_> {
    fn drop(&mut self) {
        // normal
        let start = self.normal_position;
        let end = self.normal_position + *self.normal;
        self.gizmos.arrow_2d(start, end, self.color);

        // plane line
        let direction = Direction2d::from_normalized(self.normal.perp());
        [1.0, -1.0].into_iter().for_each(|sign| {
            self.gizmos.line_2d(
                self.normal_position,
                self.normal_position + sign * direction.clamp_length(1000.0, 1000.0),
                self.color,
            );
        });
    }
}

// segment 2d

/// Builder for configuring the drawing options of [`Segment2d`].
pub struct Segment2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    direction: Direction2d, // Direction of the line segment
    half_length: f32,       // Half-length of the line segment

    draw_arrow: bool,     // decides whether to draw just a line or an arrow
    start_position: Vec2, // Starting position of the line segment
    color: Color,         // Color of the line segment
}

impl<'a, 's> Segment2dBuilder<'a, 's> {
    /// Set the drawing mode of the line (arrow vs. plain line)
    pub fn draw_arrow(mut self, is_enabled: bool) -> Self {
        self.draw_arrow = is_enabled;
        self
    }

    /// Set the starting position of the line segment.
    pub fn start_position(mut self, start_position: Vec2) -> Self {
        self.start_position = start_position;
        self
    }

    /// Set the color of the line segment.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Segment2d> for Gizmos<'s> {
    type Output<'a> = Segment2dBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Segment2d) -> Self::Output<'a> {
        Segment2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            half_length: primitive.half_length,
            draw_arrow: Default::default(),
            start_position: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for Segment2dBuilder<'_, '_> {
    fn drop(&mut self) {
        let start = self.start_position;
        let end = self.start_position + *self.direction * 2.0 * self.half_length;
        if self.draw_arrow {
            self.gizmos.arrow_2d(start, end, self.color);
        } else {
            self.gizmos.line_2d(start, end, self.color);
        }
    }
}

// polyline 2d

/// Builder for configuring the drawing options of [`Polyline2d`].
pub struct Polyline2dBuilder<'a, 's, const N: usize> {
    gizmos: &'a mut Gizmos<'s>,

    vertices: [Vec2; N], // Vertices of the polyline

    translation: Vec2, // Offset for all the vertices of the polyline
    rotation: f32,     // Rotation of the polyline around the origin in radians
    color: Color,      // Color of the polyline
}

impl<'a, 's, const N: usize> Polyline2dBuilder<'a, 's, N> {
    /// Set the offset for all the vertices of the polyline.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the polyline around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the polyline.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s, const N: usize> GizmoPrimitive2d<'s, Polyline2d<N>> for Gizmos<'s> {
    type Output<'a> = Polyline2dBuilder<'a, 's, N> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Polyline2d<N>) -> Self::Output<'a> {
        Polyline2dBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<const N: usize> Drop for Polyline2dBuilder<'_, '_, N> {
    fn drop(&mut self) {
        self.gizmos.linestrip_2d(
            self.vertices
                .iter()
                .copied()
                .map(rotate_then_translate_2d(self.rotation, self.translation)),
            self.color,
        );
    }
}

// boxed polyline 2d

/// Builder for configuring the drawing options of [`BoxedPolyline2d`].
pub struct BoxedPolylineBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    vertices: Box<[Vec2]>, // Vertices of the boxed polyline

    translation: Vec2, // Offset for all the vertices of the boxed polyline
    rotation: f32,     // Rotation of the boxed polyline around the origin in radians
    color: Color,      // Color of the boxed polyline
}

impl<'a, 's> BoxedPolylineBuilder<'a, 's> {
    /// Set the offset for all the vertices of the boxed polyline.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the boxed polyline around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the boxed polyline.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, BoxedPolyline2d> for Gizmos<'s> {
    type Output<'a> = BoxedPolylineBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: BoxedPolyline2d) -> Self::Output<'a> {
        BoxedPolylineBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<'s> Drop for BoxedPolylineBuilder<'_, 's> {
    fn drop(&mut self) {
        self.gizmos.linestrip_2d(
            self.vertices
                .iter()
                .copied()
                .map(rotate_then_translate_2d(self.rotation, self.translation)),
            self.color,
        );
    }
}

// triangle 2d

/// Builder for configuring the drawing options of [`Triangle2d`].
pub struct TriangleBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    vertices: [Vec2; 3], // Vertices of the triangle

    translation: Vec2, // Offset for all the vertices of the triangle
    rotation: f32,     // Rotation of the triangle around the origin in radians
    color: Color,      // Color of the triangle
}

impl<'a, 's> TriangleBuilder<'a, 's> {
    /// Set the offset for all the vertices of the triangle.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the triangle around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the triangle.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Triangle2d> for Gizmos<'s> {
    type Output<'a> = TriangleBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Triangle2d) -> Self::Output<'a> {
        TriangleBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<'s> Drop for TriangleBuilder<'_, 's> {
    fn drop(&mut self) {
        let [a, b, c] = self.vertices;
        let positions = [a, b, c, a].map(rotate_then_translate_2d(self.rotation, self.translation));
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

// rectangle 2d

/// Builder for configuring the drawing options of [`Rectangle`].
pub struct RectangleBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    half_width: f32,  // Half-width of the rectangle
    half_height: f32, // Half-height of the rectangle

    translation: Vec2, // Offset for all the vertices of the rectangle
    rotation: f32,     // Rotation of the rectangle around the origin in radians
    color: Color,      // Color of the rectangle
}

impl<'a, 's> RectangleBuilder<'a, 's> {
    /// Set the half-width of the rectangle.
    pub fn half_width(mut self, half_width: f32) -> Self {
        self.half_width = half_width;
        self
    }

    /// Set the half-height of the rectangle.
    pub fn half_height(mut self, half_height: f32) -> Self {
        self.half_height = half_height;
        self
    }

    /// Set the offset for all the vertices of the rectangle.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the rectangle around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the rectangle.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, Rectangle> for Gizmos<'s> {
    type Output<'a> = RectangleBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Rectangle) -> Self::Output<'a> {
        RectangleBuilder {
            gizmos: self,
            half_width: primitive.half_width,
            half_height: primitive.half_height,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl Drop for RectangleBuilder<'_, '_> {
    fn drop(&mut self) {
        let [a, b, c, d] = [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)]
            .map(|(sign_x, sign_y)| Vec2::new(self.half_width * sign_x, self.half_height * sign_y));
        let positions =
            [a, b, c, d, a].map(rotate_then_translate_2d(self.rotation, self.translation));
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

// polygon 2d

/// Builder for configuring the drawing options of [`Polygon`].
pub struct PolygonBuilder<'a, 's, const N: usize> {
    gizmos: &'a mut Gizmos<'s>,

    vertices: [Vec2; N], // Vertices of the polygon

    translation: Vec2, // Offset for all the vertices of the polygon
    rotation: f32,     // Rotation of the polygon around the origin in radians
    color: Color,      // Color of the polygon
}

impl<'a, 's, const N: usize> PolygonBuilder<'a, 's, N> {
    /// Set the offset for all the vertices of the polygon.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the polygon around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the polygon.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s, const N: usize> GizmoPrimitive2d<'s, Polygon<N>> for Gizmos<'s> {
    type Output<'a> = PolygonBuilder<'a, 's, N> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: Polygon<N>) -> Self::Output<'a> {
        PolygonBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<const N: usize> Drop for PolygonBuilder<'_, '_, N> {
    fn drop(&mut self) {
        // Check if the polygon needs a closing point
        let closing_point = {
            let last = self.vertices.last();
            (self.vertices.first() != last)
                .then_some(last)
                .flatten()
                .cloned()
        };

        self.gizmos.linestrip_2d(
            self.vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(rotate_then_translate_2d(self.rotation, self.translation)),
            self.color,
        );
    }
}

// boxed polygon 2d

/// Builder for configuring the drawing options of [`BoxedPolygon`].
pub struct BoxedPolygonBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    vertices: Box<[Vec2]>, // Vertices of the boxed polygon

    translation: Vec2, // Offset for all the vertices of the boxed polygon
    rotation: f32,     // Rotation of the boxed polygon around the origin in radians
    color: Color,      // Color of the boxed polygon
}

impl<'a, 's> BoxedPolygonBuilder<'a, 's> {
    /// Set the offset for all the vertices of the boxed polygon.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the boxed polygon around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the boxed polygon.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, BoxedPolygon> for Gizmos<'s> {
    type Output<'a> = BoxedPolygonBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: BoxedPolygon) -> Self::Output<'a> {
        BoxedPolygonBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<'s> Drop for BoxedPolygonBuilder<'_, 's> {
    fn drop(&mut self) {
        let closing_point = {
            let last = self.vertices.last();
            (self.vertices.first() != last)
                .then_some(last)
                .flatten()
                .cloned()
        };
        self.gizmos.linestrip_2d(
            self.vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(rotate_then_translate_2d(self.rotation, self.translation)),
            self.color,
        );
    }
}

// regular polygon 2d

/// Builder for configuring the drawing options of [`RegularPolygon`].
pub struct RegularPolygonBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,

    circumcircle_radius: f32, // Radius of the circumcircle of the regular polygon
    sides: usize,             // Number of sides of the regular polygon

    translation: Vec2, // Offset for all the vertices of the regular polygon
    rotation: f32,     // Rotation of the regular polygon around the origin in radians
    color: Color,      // Color of the regular polygon
}

impl<'a, 's> RegularPolygonBuilder<'a, 's> {
    /// Set the offset for all the vertices of the regular polygon.
    pub fn translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the regular polygon around the origin in radians.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the regular polygon.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'s> GizmoPrimitive2d<'s, RegularPolygon> for Gizmos<'s> {
    type Output<'a> = RegularPolygonBuilder<'a, 's> where Self: 's, 's: 'a;

    fn primitive_2d<'a>(&'s mut self, primitive: RegularPolygon) -> Self::Output<'a> {
        RegularPolygonBuilder {
            gizmos: self,
            circumcircle_radius: primitive.circumcircle.radius,
            sides: primitive.sides,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<'s> Drop for RegularPolygonBuilder<'_, 's> {
    fn drop(&mut self) {
        let points = (0..=self.sides)
            .map(|p| single_circle_coordinate(self.circumcircle_radius, self.sides, p, 1.0))
            .map(rotate_then_translate_2d(self.rotation, self.translation));
        self.gizmos.linestrip_2d(points, self.color);
    }
}

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

// helper - drawing

fn draw_cap(
    gizmos: &mut Gizmos,
    radius: f32,
    segments: usize,
    rotation: Quat,
    center: Vec3,
    sign: f32,
    color: Color,
) {
    let up = rotation * Vec3::Z;
    circle_coordinates(radius, segments)
        .map(|p| p.extend(0.0))
        .map(rotate_then_translate_3d(rotation, center))
        .for_each(|from| {
            // we need to figure out the local rotation axis for each arc which is 90
            // degree perpendicular to the (from - center) vector
            let rotation_axis = {
                let dir = from - center;
                let rot = Quat::from_axis_angle(up, FRAC_PI_2);
                rot * dir
            };

            gizmos
                .arc_3d(center, rotation_axis, from, sign * FRAC_PI_2, radius, color)
                .segments(segments / 2);
        });
}

fn draw_circle(
    gizmos: &mut Gizmos,
    radius: f32,
    segments: usize,
    rotation: Quat,
    translation: Vec3,
    color: Color,
) {
    let positions = (0..=segments)
        .map(|frac| frac as f32 / segments as f32)
        .map(|percentage| percentage * TAU)
        .map(|angle| Vec2::from(angle.sin_cos()) * radius)
        .map(|p| p.extend(0.0))
        .map(rotate_then_translate_3d(rotation, translation));
    gizmos.linestrip(positions, color);
}

fn draw_cylinder_vertical_lines(
    gizmos: &mut Gizmos,
    radius: f32,
    segments: usize,
    half_height: f32,
    rotation: Quat,
    center: Vec3,
    color: Color,
) {
    circle_coordinates(radius, segments)
        .map(move |point_2d| {
            [1.0, -1.0]
                .map(|sign| sign * half_height)
                .map(|height| point_2d.extend(height))
        })
        .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
        .for_each(|[start, end]| {
            gizmos.line(start, end, color);
        });
}
