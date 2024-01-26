//! A module for rendering each of the [`bevy_math::primitives`] with [`Gizmos`].

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy_math::primitives::{
    BoxedPolygon, BoxedPolyline2d, BoxedPolyline3d, Capsule, Circle, Cone, ConicalFrustum, Cuboid,
    Cylinder, Direction2d, Direction3d, Ellipse, Line2d, Line3d, Plane2d, Plane3d, Polygon,
    Polyline2d, Polyline3d, Primitive2d, Primitive3d, Rectangle, RegularPolygon, Segment2d,
    Segment3d, Sphere, Torus, Triangle2d,
};
use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::color::Color;

use crate::prelude::{GizmoConfigGroup, Gizmos};

// BoxedPolyline 2D

// NOTE: not sure here yet, maybe we should use a reference to some of the primitives instead since
// cloning all the vertices for drawing might defeat its purpose if we pass in the primitive by
// value

/// A trait for rendering 2D geometric primitives (`P`) with [`Gizmos`].
pub trait GizmoPrimitive2d<P: Primitive2d> {
    /// The output of `primitive_2d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 'a;

    /// Renders a 2D primitive with its associated details.
    fn primitive_2d(&mut self, primitive: P) -> Self::Output<'_>;
}

// direction 2d

/// Builder for configuring the drawing options of [`Direction2d`].
pub struct Direction2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction2d, // direction the arrow points to

    position: Vec2, // position of the start of the arrow
    color: Color,   // color of the arrow
}

impl<T: GizmoConfigGroup> Direction2dBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Direction2d> for Gizmos<'w, 's, T> {
    type Output<'a> = Direction2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Direction2d) -> Self::Output<'_> {
        Direction2dBuilder {
            gizmos: self,
            direction: primitive,
            position: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Direction2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let start = self.position;
        let end = self.position + *self.direction;
        self.gizmos.arrow_2d(start, end, self.color);
    }
}

// circle 2d

/// Builder for configuring the drawing options of [`Circle`].
pub struct Circle2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius: f32, // 2D circle to be rendered

    center: Vec2, // position of the center of the circle
    color: Color, // color of the circle
}

impl<T: GizmoConfigGroup> Circle2dBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Circle> for Gizmos<'w, 's, T> {
    type Output<'a> = Circle2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Circle) -> Self::Output<'_> {
        Circle2dBuilder {
            gizmos: self,
            radius: primitive.radius,
            center: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Circle2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        self.gizmos.circle_2d(self.center, self.radius, self.color);
    }
}

// ellipse 2d

/// Builder for configuring the drawing options of [`Ellipse`].
pub struct Ellipse2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    half_width: f32,  // Half-width of the ellipse
    half_height: f32, // Half-height of the ellipse

    center: Vec2, // Position of the center of the ellipse
    color: Color, // Color of the ellipse
}

impl<T: GizmoConfigGroup> Ellipse2dBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Ellipse> for Gizmos<'w, 's, T> {
    type Output<'a> = Ellipse2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Ellipse) -> Self::Output<'_> {
        Ellipse2dBuilder {
            gizmos: self,
            half_width: primitive.half_size.x,
            half_height: primitive.half_size.y,
            center: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Ellipse2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        self.gizmos
            .ellipse_2d(self.center, self.half_width, self.half_height, self.color);
    }
}

// line 2d

/// Builder for configuring the drawing options of [`Line2d`].
pub struct Line2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction2d, // Direction of the line

    start_position: Vec2, // Starting position of the line
    color: Color,         // Color of the line
}

impl<T: GizmoConfigGroup> Line2dBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Line2d> for Gizmos<'w, 's, T> {
    type Output<'a> = Line2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Line2d) -> Self::Output<'_> {
        Line2dBuilder {
            gizmos: self,
            direction: primitive.direction,
            start_position: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Line2dBuilder<'_, '_, '_, T> {
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
pub struct Plane2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    normal: Direction2d, // Normal of the plane

    normal_position: Vec2, // Starting position of the normal of the plane
    color: Color,          // Color of the plane
}

impl<T: GizmoConfigGroup> Plane2dBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Plane2d> for Gizmos<'w, 's, T> {
    type Output<'a> = Plane2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Plane2d) -> Self::Output<'_> {
        Plane2dBuilder {
            gizmos: self,
            normal: primitive.normal,
            normal_position: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Plane2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        // normal
        let start = self.normal_position;
        let end = self.normal_position + *self.normal;
        self.gizmos.arrow_2d(start, end, self.color);

        // plane line
        let direction = Direction2d::new_unchecked(self.normal.perp());
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
pub struct Segment2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction2d, // Direction of the line segment
    half_length: f32,       // Half-length of the line segment

    draw_arrow: bool,     // decides whether to draw just a line or an arrow
    start_position: Vec2, // Starting position of the line segment
    color: Color,         // Color of the line segment
}

impl<T: GizmoConfigGroup> Segment2dBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Segment2d> for Gizmos<'w, 's, T> {
    type Output<'a> = Segment2dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Segment2d) -> Self::Output<'_> {
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

impl<T: GizmoConfigGroup> Drop for Segment2dBuilder<'_, '_, '_, T> {
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
pub struct Polyline2dBuilder<'a, 'w, 's, const N: usize, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: [Vec2; N], // Vertices of the polyline

    translation: Vec2, // Offset for all the vertices of the polyline
    rotation: f32,     // Rotation of the polyline around the origin in radians
    color: Color,      // Color of the polyline
}

impl<const N: usize, T: GizmoConfigGroup> Polyline2dBuilder<'_, '_, '_, N, T> {
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

impl<'w, 's, const N: usize, T: GizmoConfigGroup> GizmoPrimitive2d<Polyline2d<N>>
    for Gizmos<'w, 's, T>
{
    type Output<'a> = Polyline2dBuilder<'a, 'w, 's, N, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Polyline2d<N>) -> Self::Output<'_> {
        Polyline2dBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<const N: usize, T: GizmoConfigGroup> Drop for Polyline2dBuilder<'_, '_, '_, N, T> {
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
pub struct BoxedPolylineBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: Box<[Vec2]>, // Vertices of the boxed polyline

    translation: Vec2, // Offset for all the vertices of the boxed polyline
    rotation: f32,     // Rotation of the boxed polyline around the origin in radians
    color: Color,      // Color of the boxed polyline
}

impl<T: GizmoConfigGroup> BoxedPolylineBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<BoxedPolyline2d> for Gizmos<'w, 's, T> {
    type Output<'a> = BoxedPolylineBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: BoxedPolyline2d) -> Self::Output<'_> {
        BoxedPolylineBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for BoxedPolylineBuilder<'_, '_, '_, T> {
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
pub struct TriangleBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: [Vec2; 3], // Vertices of the triangle

    translation: Vec2, // Offset for all the vertices of the triangle
    rotation: f32,     // Rotation of the triangle around the origin in radians
    color: Color,      // Color of the triangle
}

impl<T: GizmoConfigGroup> TriangleBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Triangle2d> for Gizmos<'w, 's, T> {
    type Output<'a> = TriangleBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Triangle2d) -> Self::Output<'_> {
        TriangleBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for TriangleBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let [a, b, c] = self.vertices;
        let positions = [a, b, c, a].map(rotate_then_translate_2d(self.rotation, self.translation));
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

// rectangle 2d

/// Builder for configuring the drawing options of [`Rectangle`].
pub struct RectangleBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    half_width: f32,  // Half-width of the rectangle
    half_height: f32, // Half-height of the rectangle

    translation: Vec2, // Offset for all the vertices of the rectangle
    rotation: f32,     // Rotation of the rectangle around the origin in radians
    color: Color,      // Color of the rectangle
}

impl<T: GizmoConfigGroup> RectangleBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<Rectangle> for Gizmos<'w, 's, T> {
    type Output<'a> = RectangleBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Rectangle) -> Self::Output<'_> {
        RectangleBuilder {
            gizmos: self,
            half_width: primitive.half_size.x,
            half_height: primitive.half_size.y,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for RectangleBuilder<'_, '_, '_, T> {
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
pub struct PolygonBuilder<'a, 'w, 's, const N: usize, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: [Vec2; N], // Vertices of the polygon

    translation: Vec2, // Offset for all the vertices of the polygon
    rotation: f32,     // Rotation of the polygon around the origin in radians
    color: Color,      // Color of the polygon
}

impl<const N: usize, T: GizmoConfigGroup> PolygonBuilder<'_, '_, '_, N, T> {
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

impl<'w, 's, const N: usize, T: GizmoConfigGroup> GizmoPrimitive2d<Polygon<N>>
    for Gizmos<'w, 's, T>
{
    type Output<'a> = PolygonBuilder<'a, 'w, 's, N, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: Polygon<N>) -> Self::Output<'_> {
        PolygonBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<const N: usize, T: GizmoConfigGroup> Drop for PolygonBuilder<'_, '_, '_, N, T> {
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
pub struct BoxedPolygonBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: Box<[Vec2]>, // Vertices of the boxed polygon

    translation: Vec2, // Offset for all the vertices of the boxed polygon
    rotation: f32,     // Rotation of the boxed polygon around the origin in radians
    color: Color,      // Color of the boxed polygon
}

impl<T: GizmoConfigGroup> BoxedPolygonBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<BoxedPolygon> for Gizmos<'w, 's, T> {
    type Output<'a> = BoxedPolygonBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: BoxedPolygon) -> Self::Output<'_> {
        BoxedPolygonBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for BoxedPolygonBuilder<'_, '_, '_, T> {
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
pub struct RegularPolygonBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    circumcircle_radius: f32, // Radius of the circumcircle of the regular polygon
    sides: usize,             // Number of sides of the regular polygon

    translation: Vec2, // Offset for all the vertices of the regular polygon
    rotation: f32,     // Rotation of the regular polygon around the origin in radians
    color: Color,      // Color of the regular polygon
}

impl<T: GizmoConfigGroup> RegularPolygonBuilder<'_, '_, '_, T> {
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive2d<RegularPolygon> for Gizmos<'w, 's, T> {
    type Output<'a> = RegularPolygonBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_2d(&mut self, primitive: RegularPolygon) -> Self::Output<'_> {
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

impl<T: GizmoConfigGroup> Drop for RegularPolygonBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let points = (0..=self.sides)
            .map(|p| single_circle_coordinate(self.circumcircle_radius, self.sides, p, 1.0))
            .map(rotate_then_translate_2d(self.rotation, self.translation));
        self.gizmos.linestrip_2d(points, self.color);
    }
}

// === 3D ===

/// A trait for rendering 3D geometric primitives (`P`) with [`Gizmos`].
pub trait GizmoPrimitive3d<P: Primitive3d> {
    /// The output of `primitive_3d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 'a;

    /// Renders a 3D primitive with its associated details.
    fn primitive_3d(&mut self, primitive: P) -> Self::Output<'_>;
}

// direction 3d

/// Builder for configuring the drawing options of [`Direction3d`].
pub struct Direction3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction3d, // Direction the arrow points to

    position: Vec3, // Starting position of the arrow in 3D space
    color: Color,   // Color of the arrow
}

impl<T: GizmoConfigGroup> Direction3dBuilder<'_, '_, '_, T> {
    /// Set the starting position of the arrow in 3D space.
    pub fn position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Set the color of the arrow.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Direction3d> for Gizmos<'w, 's, T> {
    type Output<'a> = Direction3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Direction3d) -> Self::Output<'_> {
        Direction3dBuilder {
            gizmos: self,
            direction: primitive,
            position: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Direction3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        self.gizmos
            .arrow(self.position, self.position + *self.direction, self.color);
    }
}

// sphere

/// Builder for configuring the drawing options of [`Sphere`].
pub struct SphereBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius: f32, // Radius of the sphere

    center: Vec3,    // Center position of the sphere in 3D space
    rotation: Quat,  // Rotation of the sphere around the origin in 3D space
    color: Color,    // Color of the sphere
    segments: usize, // Number of segments used to approximate the sphere geometry
}

impl<T: GizmoConfigGroup> SphereBuilder<'_, '_, '_, T> {
    /// Set the radius of the sphere.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Set the center position of the sphere in 3D space.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the rotation of the sphere around the origin in 3D space.
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the sphere.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the number of segments used to approximate the sphere geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Sphere> for Gizmos<'w, 's, T> {
    type Output<'a> = SphereBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Sphere) -> Self::Output<'_> {
        SphereBuilder {
            gizmos: self,
            radius: primitive.radius,
            center: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
            segments: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for SphereBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let SphereBuilder {
            radius,
            center,
            rotation,
            color,
            segments,
            ..
        } = self;

        // draw two caps, one for the "upper half" and one for the "lower" half of the sphere
        [-1.0, 1.0].into_iter().for_each(|sign| {
            draw_cap(
                self.gizmos,
                *radius,
                *segments,
                *rotation,
                *center,
                sign,
                *color,
            );
        });

        draw_circle(self.gizmos, *radius, *segments, *rotation, *center, *color);
    }
}

// plane 3d

/// Builder for configuring the drawing options of [`Plane3d`].
pub struct Plane3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    normal: Direction3d, // Normal vector of the plane

    normal_position: Vec3, // Position of the normal vector of the plane
    rotation: Quat,        // Rotation of the plane around the origin (`Vec3::ZERO`)
    color: Color,          // Color of the plane
}

impl<T: GizmoConfigGroup> Plane3dBuilder<'_, '_, '_, T> {
    /// Set the normal vector of the plane.
    pub fn normal_position(mut self, normal_position: Vec3) -> Self {
        self.normal_position = normal_position;
        self
    }

    /// Set the rotation of the plane around the origin (`Vec3::ZERO`).
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the plane.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Plane3d> for Gizmos<'w, 's, T> {
    type Output<'a> = Plane3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Plane3d) -> Self::Output<'_> {
        Plane3dBuilder {
            gizmos: self,
            normal: primitive.normal,
            normal_position: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Plane3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Plane3dBuilder {
            gizmos,
            normal,
            normal_position,
            rotation,
            color,
        } = self;

        let normal = *rotation * **normal;
        gizmos.arrow(*normal_position, *normal_position + normal, *color);
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
                    .map(|vs| vs.map(|v| v + *normal_position))
                    .take(3)
                    .for_each(|[start, end]| {
                        gizmos.line(start, end, *color);
                    });
            });
    }
}

// line 3d

/// Builder for configuring the drawing options of [`Line3d`].
pub struct Line3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction3d, // Direction vector of the line

    start_position: Vec3, // Starting position of the line
    rotation: Quat,       // Rotation of the line around the origin (`Vec3::ZERO`)
    color: Color,         // Color of the line
}

impl<T: GizmoConfigGroup> Line3dBuilder<'_, '_, '_, T> {
    /// Set the starting position of the line.
    pub fn start_position(mut self, start_position: Vec3) -> Self {
        self.start_position = start_position;
        self
    }

    /// Set the rotation of the line around the origin (`Vec3::ZERO`).
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the line.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Line3d> for Gizmos<'w, 's, T> {
    type Output<'a> = Line3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Line3d) -> Self::Output<'_> {
        Line3dBuilder {
            gizmos: self,
            direction: primitive.direction,
            start_position: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Line3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Line3dBuilder {
            gizmos,
            direction,
            start_position,
            rotation,
            color,
        } = self;

        let dir = *rotation * **direction;
        gizmos.arrow(*start_position, *start_position + dir, *color);
        [1.0, -1.0].into_iter().for_each(|sign| {
            gizmos.line(
                *start_position,
                *start_position + sign * dir.clamp_length(1000.0, 1000.0),
                *color,
            );
        });
    }
}

// segment 3d

/// Builder for configuring the drawing options of [`Segment3d`].
pub struct Segment3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    direction: Direction3d, // Direction vector of the line segment
    half_length: f32,       // Half the length of the Segment

    start_position: Vec3, // Starting position of the line segment
    rotation: Quat,       // Rotation of the line segment around the origin (`Vec3::ZERO`)
    color: Color,         // Color of the line segment
}

impl<T: GizmoConfigGroup> Segment3dBuilder<'_, '_, '_, T> {
    /// Set the direction vector of the line segment.
    pub fn half_length(mut self, half_length: f32) -> Self {
        self.half_length = half_length;
        self
    }

    /// Set the starting position of the line segment.
    pub fn start_position(mut self, start_position: Vec3) -> Self {
        self.start_position = start_position;
        self
    }

    /// Set the rotation of the line segment around the origin (`Vec3::ZERO`).
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the line segment.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Segment3d> for Gizmos<'w, 's, T> {
    type Output<'a> = Segment3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Segment3d) -> Self::Output<'_> {
        Segment3dBuilder {
            gizmos: self,
            direction: primitive.direction,
            half_length: primitive.half_length,
            start_position: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Segment3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Segment3dBuilder {
            gizmos,
            direction,
            half_length,
            start_position,
            rotation,
            color,
        } = self;

        let dir = *rotation * **direction;
        let start = *start_position;
        let end = *start_position + dir * 2.0 * *half_length;
        gizmos.line(start, end, *color);
    }
}

// polyline 3d

/// Builder for configuring the drawing options of [`Polyline3d`].
pub struct Polyline3dBuilder<'a, 'w, 's, const N: usize, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: [Vec3; N], // Vertices of the polyline

    translation: Vec3, // Translation applied to all vertices of the polyline
    rotation: Quat,    // Rotation of the polyline around the origin (`Vec3::ZERO`)
    color: Color,      // Color of the polyline
}

impl<const N: usize, T: GizmoConfigGroup> Polyline3dBuilder<'_, '_, '_, N, T> {
    /// Set the translation applied to all vertices of the polyline.
    pub fn translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the polyline around the origin (`Vec3::ZERO`) given as a quaternion.
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the polyline.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, const N: usize, T: GizmoConfigGroup> GizmoPrimitive3d<Polyline3d<N>>
    for Gizmos<'w, 's, T>
{
    type Output<'a> = Polyline3dBuilder<'a,  'w, 's, N, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Polyline3d<N>) -> Self::Output<'_> {
        Polyline3dBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<const N: usize, T: GizmoConfigGroup> Drop for Polyline3dBuilder<'_, '_, '_, N, T> {
    fn drop(&mut self) {
        let Polyline3dBuilder {
            gizmos,
            vertices,
            translation,
            rotation,
            color,
        } = self;

        gizmos.linestrip(
            vertices.map(rotate_then_translate_3d(*rotation, *translation)),
            *color,
        );
    }
}

// boxed polyline 3d

/// Builder for configuring the drawing options of [`BoxedPolyline3d`].
pub struct BoxedPolyline3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    vertices: Box<[Vec3]>, // Vertices of the boxed polyline

    translation: Vec3, // Translation applied to all vertices of the boxed polyline
    rotation: Quat, // Rotation of the polyline around the origin (`Vec3::ZERO`) given as a quaternion
    color: Color,   // Color of the polyline and the enclosing box
}

impl<T: GizmoConfigGroup> BoxedPolyline3dBuilder<'_, '_, '_, T> {
    /// Set the translation applied to all vertices of the boxed polyline.
    pub fn translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    /// Set the rotation of the polyline around the origin (`Vec3::ZERO`) given as a quaternion.
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the polyline and the enclosing box.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<BoxedPolyline3d> for Gizmos<'w, 's, T> {
    type Output<'a> = BoxedPolyline3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: BoxedPolyline3d) -> Self::Output<'_> {
        BoxedPolyline3dBuilder {
            gizmos: self,
            vertices: primitive.vertices,
            translation: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for BoxedPolyline3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let BoxedPolyline3dBuilder {
            gizmos,
            vertices,
            translation,
            rotation,
            color,
        } = self;

        gizmos.linestrip(
            vertices
                .iter()
                .copied()
                .map(rotate_then_translate_3d(*rotation, *translation)),
            *color,
        );
    }
}

// cuboid

/// Builder for configuring the drawing options of [`Cuboid3d`].
pub struct Cuboid3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    half_extents: Vec3, // Half extents of the cuboid on each axis

    center: Vec3,   // Center position of the cuboid
    rotation: Quat, // Rotation of the cuboid around its center given as a quaternion
    color: Color,   // Color of the cuboid
}

impl<T: GizmoConfigGroup> Cuboid3dBuilder<'_, '_, '_, T> {
    /// Set the center position of the cuboid.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the rotation of the cuboid around its center given as a quaternion.
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the cuboid.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Cuboid> for Gizmos<'w, 's, T> {
    type Output<'a> = Cuboid3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Cuboid) -> Self::Output<'_> {
        Cuboid3dBuilder {
            gizmos: self,
            half_extents: primitive.half_size,
            center: Default::default(),
            rotation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Cuboid3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Cuboid3dBuilder {
            gizmos,
            half_extents,
            center,
            rotation,
            color,
        } = self;

        let [half_extend_x, half_extend_y, half_extend_z] = half_extents.to_array();

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
        .map(rotate_then_translate_3d(*rotation, *center));

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
                gizmos.line(start, end, *color);
            });
    }
}

// cylinder 3d

/// Builder for configuring the drawing options of [`Cylinder3d`].
pub struct Cylinder3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius: f32,      // Radius of the cylinder
    half_height: f32, // Half height of the cylinder

    center: Vec3,    // Center position of the cylinder
    normal: Vec3,    // Normal vector indicating the orientation of the cylinder
    color: Color,    // Color of the cylinder
    segments: usize, // Number of segments used to approximate the cylinder geometry
}

impl<T: GizmoConfigGroup> Cylinder3dBuilder<'_, '_, '_, T> {
    /// Set the center position of the cylinder.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the normal vector indicating the orientation of the cylinder.
    pub fn normal(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Set the color of the cylinder.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the number of segments used to approximate the cylinder geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Cylinder> for Gizmos<'w, 's, T> {
    type Output<'a> = Cylinder3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Cylinder) -> Self::Output<'_> {
        Cylinder3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_height: primitive.half_height,
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Cylinder3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Cylinder3dBuilder {
            gizmos,
            radius,
            half_height,
            center,
            normal,
            color,
            segments,
        } = self;

        let rotation = Quat::from_rotation_arc(Vec3::Z, *normal);

        [-1.0, 1.0].into_iter().for_each(|sign| {
            draw_circle(
                gizmos,
                *radius,
                *segments,
                rotation,
                *center + sign * *half_height * *normal,
                *color,
            );
        });

        draw_cylinder_vertical_lines(
            gizmos,
            *radius,
            *segments,
            *half_height,
            rotation,
            *center,
            *color,
        );
    }
}

// capsule 3d

/// Builder for configuring the drawing options of [`Capsule3d`].
pub struct Capsule3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius: f32,      // Radius of the capsule
    half_length: f32, // Half length of the capsule

    center: Vec3,    // Center position of the capsule
    normal: Vec3,    // Normal vector indicating the orientation of the capsule
    color: Color,    // Color of the capsule
    segments: usize, // Number of segments used to approximate the capsule geometry
}

impl<T: GizmoConfigGroup> Capsule3dBuilder<'_, '_, '_, T> {
    /// Set the center position of the capsule.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the normal vector indicating the orientation of the capsule.
    pub fn normal(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Set the color of the capsule.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the number of segments used to approximate the capsule geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Capsule> for Gizmos<'w, 's, T> {
    type Output<'a> = Capsule3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Capsule) -> Self::Output<'_> {
        Capsule3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_length: primitive.half_length,
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Capsule3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Capsule3dBuilder {
            gizmos,
            radius,
            half_length,
            center,
            normal,
            color,
            segments,
        } = self;

        let rotation = Quat::from_rotation_arc(Vec3::Z, *normal);

        [1.0, -1.0].into_iter().for_each(|sign| {
            // use "-" here since rotation is ccw and otherwise the caps would face the wrong way
            // around
            let center = *center - sign * *half_length * *normal;
            draw_cap(gizmos, *radius, *segments, rotation, center, sign, *color);
            draw_circle(gizmos, *radius, *segments, rotation, center, *color);
        });

        draw_cylinder_vertical_lines(
            gizmos,
            *radius,
            *segments,
            *half_length,
            rotation,
            *center,
            *color,
        );
    }
}

// cone 3d

/// Builder for configuring the drawing options of [`Cone3d`].
pub struct Cone3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius: f32, // Radius of the cone
    height: f32, // Height of the cone

    center: Vec3,    // Center of the base of the cone
    normal: Vec3,    // Normal vector indicating the orientation of the cone
    color: Color,    // Color of the cone
    segments: usize, // Number of segments used to approximate the cone geometry
}

impl<T: GizmoConfigGroup> Cone3dBuilder<'_, '_, '_, T> {
    /// Set the center of the base of the cone.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the normal vector indicating the orientation of the cone.
    pub fn normal(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Set the color of the cone.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the number of segments used to approximate the cone geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Cone> for Gizmos<'w, 's, T> {
    type Output<'a> = Cone3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Cone) -> Self::Output<'_> {
        Cone3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            height: primitive.height,
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Cone3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Cone3dBuilder {
            gizmos,
            radius,
            height,
            center,
            normal,
            color,
            segments,
        } = self;

        let rotation = Quat::from_rotation_arc(Vec3::Z, *normal);

        draw_circle(gizmos, *radius, *segments, rotation, *center, *color);

        let end = Vec2::ZERO.extend(*height);
        circle_coordinates(*radius, *segments)
            .map(move |p| [p.extend(0.0), end])
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, *center)))
            .for_each(|[start, end]| {
                gizmos.line(start, end, *color);
            });
    }
}

// conical frustum 3d

/// Builder for configuring the drawing options of [`ConicalFrustum3d`].
pub struct ConicalFrustum3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius_top: f32,    // Radius of the top circle
    radius_bottom: f32, // Radius of the bottom circle
    height: f32,        // Height of the conical frustum

    center: Vec3,    // Center of the base circle of the conical frustum
    normal: Vec3,    // Normal vector indicating the orientation of the conical frustum
    color: Color,    // Color of the conical frustum
    segments: usize, // Number of segments used to approximate the curved surfaces
}

impl<T: GizmoConfigGroup> ConicalFrustum3dBuilder<'_, '_, '_, T> {
    /// Set the center of the base circle of the conical frustum.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the normal vector indicating the orientation of the conical frustum.
    pub fn normal(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Set the color of the conical frustum.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the number of segments used to approximate the curved surfaces.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<ConicalFrustum> for Gizmos<'w, 's, T> {
    type Output<'a> = ConicalFrustum3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: ConicalFrustum) -> Self::Output<'_> {
        ConicalFrustum3dBuilder {
            gizmos: self,
            radius_top: primitive.radius_top,
            radius_bottom: primitive.radius_bottom,
            height: primitive.height,
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            segments: 5,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for ConicalFrustum3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let ConicalFrustum3dBuilder {
            gizmos,
            radius_top,
            radius_bottom,
            height,
            center,
            normal,
            color,
            segments,
        } = self;

        let rotation = Quat::from_rotation_arc(Vec3::Z, *normal);
        [(*radius_top, *height), (*radius_bottom, 0.0)]
            .into_iter()
            .for_each(|(radius, height)| {
                draw_circle(
                    gizmos,
                    radius,
                    *segments,
                    rotation,
                    *center + height * *normal,
                    *color,
                );
            });

        circle_coordinates(*radius_top, *segments)
            .map(move |p| p.extend(*height))
            .zip(circle_coordinates(*radius_bottom, *segments).map(|p| p.extend(0.0)))
            .map(|(start, end)| [start, end])
            .map(|ps| ps.map(rotate_then_translate_3d(rotation, *center)))
            .for_each(|[start, end]| {
                gizmos.line(start, end, *color);
            });
    }
}

// torus 3d

/// Builder for configuring the drawing options of [`Torus3d`].
pub struct Torus3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    minor_radius: f32, // Radius of the minor circle (tube)
    major_radius: f32, // Radius of the major circle (ring)

    center: Vec3,          // Center of the torus
    normal: Vec3,          // Normal vector indicating the orientation of the torus
    color: Color,          // Color of the torus
    minor_segments: usize, // Number of segments in the minor (tube) direction
    major_segments: usize, // Number of segments in the major (ring) direction
}

impl<T: GizmoConfigGroup> Torus3dBuilder<'_, '_, '_, T> {
    /// Set the center of the torus.
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the normal vector indicating the orientation of the torus.
    pub fn normal(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Set the color of the torus.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the number of segments in the minor (tube) direction.
    pub fn minor_segments(mut self, minor_segments: usize) -> Self {
        self.minor_segments = minor_segments;
        self
    }

    /// Set the number of segments in the major (ring) direction.
    pub fn major_segments(mut self, major_segments: usize) -> Self {
        self.major_segments = major_segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Torus> for Gizmos<'w, 's, T> {
    type Output<'a> = Torus3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(&mut self, primitive: Torus) -> Self::Output<'_> {
        Torus3dBuilder {
            gizmos: self,
            minor_radius: primitive.minor_radius,
            major_radius: primitive.major_radius,
            center: Default::default(),
            normal: Vec3::Z,
            color: Default::default(),
            minor_segments: 5,
            major_segments: 5,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Torus3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let Torus3dBuilder {
            gizmos,
            minor_radius,
            major_radius,
            center,
            normal,
            color,
            minor_segments,
            major_segments,
        } = self;

        let rotation = Quat::from_rotation_arc(Vec3::Z, *normal);

        [
            (*major_radius - *minor_radius, 0.0),
            (*major_radius + *minor_radius, 0.0),
            (*major_radius, *minor_radius),
            (*major_radius, -*minor_radius),
        ]
        .into_iter()
        .for_each(|(radius, height)| {
            draw_circle(
                gizmos,
                radius,
                *major_segments,
                rotation,
                *center + height * *normal,
                *color,
            );
        });

        let affine = rotate_then_translate_3d(rotation, *center);
        circle_coordinates(*major_radius, *major_segments)
            .flat_map(|p| {
                let translation = affine(p.extend(0.0));
                let dir_to_translation = (translation - *center).normalize();
                let rotation_axis = normal.cross(dir_to_translation).normalize();
                [dir_to_translation, *normal, -dir_to_translation, -*normal]
                    .map(|dir| dir * *minor_radius)
                    .map(|offset| translation + offset)
                    .map(|point| (point, translation, rotation_axis))
            })
            .for_each(|(from, center, rotation_axis)| {
                gizmos
                    .arc_3d(
                        center,
                        rotation_axis,
                        from,
                        FRAC_PI_2,
                        *minor_radius,
                        *color,
                    )
                    .segments(*minor_segments);
            });
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

fn draw_cap<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
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

fn draw_circle<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
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

fn draw_cylinder_vertical_lines<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
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
