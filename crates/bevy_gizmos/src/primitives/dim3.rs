//! A module for rendering each of the 3D [`bevy_math::primitives`] with [`Gizmos`].

use super::helpers::*;
use std::f32::consts::TAU;

use bevy_color::Color;
use bevy_math::primitives::{
    BoxedPolyline3d, Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, Line3d, Plane3d,
    Polyline3d, Primitive3d, Segment3d, Sphere, Tetrahedron, Torus,
};
use bevy_math::{Dir3, Quat, Vec3};

use crate::prelude::{GizmoConfigGroup, Gizmos};

const DEFAULT_NUMBER_SEGMENTS: usize = 5;
// length used to simulate infinite lines
const INFINITE_LEN: f32 = 10_000.0;

/// A trait for rendering 3D geometric primitives (`P`) with [`Gizmos`].
pub trait GizmoPrimitive3d<P: Primitive3d> {
    /// The output of `primitive_3d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 'a;

    /// Renders a 3D primitive with its associated details.
    fn primitive_3d(
        &mut self,
        primitive: P,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_>;
}

// direction 3d

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Dir3> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Dir3,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.arrow(position, position + (rotation * *primitive), color);
    }
}

// sphere

/// Builder for configuring the drawing options of [`Sphere`].
pub struct SphereBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the sphere
    radius: f32,

    // Rotation of the sphere around the origin in 3D space
    rotation: Quat,
    // Center position of the sphere in 3D space
    position: Vec3,
    // Color of the sphere
    color: Color,

    // Number of segments used to approximate the sphere geometry
    segments: usize,
}

impl<Config, Clear> SphereBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to approximate the sphere geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Sphere> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = SphereBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Sphere,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        SphereBuilder {
            gizmos: self,
            radius: primitive.radius,
            position,
            rotation,
            color: color.into(),
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<Config, Clear> Drop for SphereBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let SphereBuilder {
            radius,
            position: center,
            rotation,
            color,
            segments,
            ..
        } = self;

        // draws the upper and lower semi spheres
        [-1.0, 1.0].into_iter().for_each(|sign| {
            let top = *center + (*rotation * Vec3::Y) * sign * *radius;
            draw_semi_sphere(
                self.gizmos,
                *radius,
                *segments,
                *rotation,
                *center,
                top,
                *color,
            );
        });

        // draws one great circle of the sphere
        draw_circle_3d(self.gizmos, *radius, *segments, *rotation, *center, *color);
    }
}

// plane 3d

/// Builder for configuring the drawing options of [`Plane3d`].
pub struct Plane3dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // direction of the normal orthogonal to the plane
    normal: Dir3,

    // Rotation of the plane around the origin in 3D space
    rotation: Quat,
    // Center position of the plane in 3D space
    position: Vec3,
    // Color of the plane
    color: Color,

    // Number of axis to hint the plane
    axis_count: usize,
    // Number of segments used to hint the plane
    segment_count: usize,
    // Length of segments used to hint the plane
    segment_length: f32,
}

impl<Config, Clear> Plane3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to hint the plane.
    pub fn segment_count(mut self, count: usize) -> Self {
        self.segment_count = count;
        self
    }

    /// Set the length of segments used to hint the plane.
    pub fn segment_length(mut self, length: f32) -> Self {
        self.segment_length = length;
        self
    }

    /// Set the number of axis used to hint the plane.
    pub fn axis_count(mut self, count: usize) -> Self {
        self.axis_count = count;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Plane3d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Plane3dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Plane3d,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Plane3dBuilder {
            gizmos: self,
            normal: primitive.normal,
            rotation,
            position,
            color: color.into(),
            axis_count: 4,
            segment_count: 3,
            segment_length: 0.25,
        }
    }
}

impl<Config, Clear> Drop for Plane3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        // draws the normal
        let normal = self.rotation * *self.normal;
        self.gizmos
            .primitive_3d(self.normal, self.position, self.rotation, self.color);
        let normals_normal = normal.any_orthonormal_vector();

        // draws the axes
        // get rotation for each direction
        (0..self.axis_count)
            .map(|i| i as f32 * (1.0 / self.axis_count as f32) * TAU)
            .map(|angle| Quat::from_axis_angle(normal, angle))
            .for_each(|quat| {
                let axis_direction = quat * normals_normal;
                let direction = Dir3::new(axis_direction).unwrap();

                // for each axis draw dotted line
                (0..)
                    .filter(|i| i % 2 != 0)
                    .map(|percent| (percent as f32 + 0.5) * self.segment_length * axis_direction)
                    .map(|position| position + self.position)
                    .take(self.segment_count)
                    .for_each(|position| {
                        self.gizmos.primitive_3d(
                            Segment3d {
                                direction,
                                half_length: self.segment_length * 0.5,
                            },
                            position,
                            Quat::IDENTITY,
                            self.color,
                        );
                    });
            });
    }
}

// line 3d

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Line3d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Line3d,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let color = color.into();
        let direction = rotation * *primitive.direction;
        self.arrow(position, position + direction, color);

        let [start, end] = [1.0, -1.0]
            .map(|sign| sign * INFINITE_LEN)
            .map(|length| direction * length)
            .map(|offset| position + offset);
        self.line(start, end, color);
    }
}

// segment 3d

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Segment3d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Segment3d,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let direction = rotation * *primitive.direction;
        let start = position - direction * primitive.half_length;
        let end = position + direction * primitive.half_length;
        self.line(start, end, color);
    }
}

// polyline 3d

impl<'w, 's, const N: usize, Config, Clear> GizmoPrimitive3d<Polyline3d<N>>
    for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Polyline3d<N>,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip(
            primitive
                .vertices
                .map(rotate_then_translate_3d(rotation, position)),
            color,
        );
    }
}

// boxed polyline 3d

impl<'w, 's, Config, Clear> GizmoPrimitive3d<BoxedPolyline3d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: BoxedPolyline3d,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip(
            primitive
                .vertices
                .iter()
                .copied()
                .map(rotate_then_translate_3d(rotation, position)),
            color,
        );
    }
}

// cuboid

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Cuboid> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Cuboid,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let [half_extend_x, half_extend_y, half_extend_z] = primitive.half_size.to_array();

        // transform the points from the reference unit cube to the cuboid coords
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
        .map(rotate_then_translate_3d(rotation, position));

        // lines for the upper rectangle of the cuboid
        let upper = [a, b, c, d]
            .into_iter()
            .zip([a, b, c, d].into_iter().cycle().skip(1));

        // lines for the lower rectangle of the cuboid
        let lower = [e, f, g, h]
            .into_iter()
            .zip([e, f, g, h].into_iter().cycle().skip(1));

        // lines connecting upper and lower rectangles of the cuboid
        let connections = vertices.into_iter().zip(vertices.into_iter().skip(4));

        let color = color.into();
        upper
            .chain(lower)
            .chain(connections)
            .for_each(|(start, end)| {
                self.line(start, end, color);
            });
    }
}

// cylinder 3d

/// Builder for configuring the drawing options of [`Cylinder`].
pub struct Cylinder3dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the cylinder
    radius: f32,
    // Half height of the cylinder
    half_height: f32,

    // Center position of the cylinder
    position: Vec3,
    // Rotation of the cylinder
    //
    // default orientation is: the cylinder is aligned with `Vec3::Y` axis
    rotation: Quat,
    // Color of the cylinder
    color: Color,

    // Number of segments used to approximate the cylinder geometry
    segments: usize,
}

impl<Config, Clear> Cylinder3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to approximate the cylinder geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Cylinder> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Cylinder3dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Cylinder,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Cylinder3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_height: primitive.half_height,
            position,
            rotation,
            color: color.into(),
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<Config, Clear> Drop for Cylinder3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let Cylinder3dBuilder {
            gizmos,
            radius,
            half_height,
            position,
            rotation,
            color,
            segments,
        } = self;

        let normal = *rotation * Vec3::Y;

        // draw upper and lower circle of the cylinder
        [-1.0, 1.0].into_iter().for_each(|sign| {
            draw_circle_3d(
                gizmos,
                *radius,
                *segments,
                *rotation,
                *position + sign * *half_height * normal,
                *color,
            );
        });

        // draw lines connecting the two cylinder circles
        draw_cylinder_vertical_lines(
            gizmos,
            *radius,
            *segments,
            *half_height,
            *rotation,
            *position,
            *color,
        );
    }
}

// capsule 3d

/// Builder for configuring the drawing options of [`Capsule3d`].
pub struct Capsule3dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the capsule
    radius: f32,
    // Half length of the capsule
    half_length: f32,

    // Center position of the capsule
    position: Vec3,
    // Rotation of the capsule
    //
    // default orientation is: the capsule is aligned with `Vec3::Y` axis
    rotation: Quat,
    // Color of the capsule
    color: Color,

    // Number of segments used to approximate the capsule geometry
    segments: usize,
}

impl<Config, Clear> Capsule3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to approximate the capsule geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Capsule3d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Capsule3dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Capsule3d,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Capsule3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_length: primitive.half_length,
            position,
            rotation,
            color: color.into(),
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<Config, Clear> Drop for Capsule3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let Capsule3dBuilder {
            gizmos,
            radius,
            half_length,
            position,
            rotation,
            color,
            segments,
        } = self;

        let normal = *rotation * Vec3::Y;

        // draw two semi spheres for the capsule
        [1.0, -1.0].into_iter().for_each(|sign| {
            let center = *position + sign * *half_length * normal;
            let top = center + sign * *radius * normal;
            draw_semi_sphere(gizmos, *radius, *segments, *rotation, center, top, *color);
            draw_circle_3d(gizmos, *radius, *segments, *rotation, center, *color);
        });

        // connect the two semi spheres with lines
        draw_cylinder_vertical_lines(
            gizmos,
            *radius,
            *segments,
            *half_length,
            *rotation,
            *position,
            *color,
        );
    }
}

// cone 3d

/// Builder for configuring the drawing options of [`Cone`].
pub struct Cone3dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the cone
    radius: f32,
    // Height of the cone
    height: f32,

    // Center of the cone, half-way between the tip and the base
    position: Vec3,
    // Rotation of the cone
    //
    // default orientation is: cone base normal is aligned with the `Vec3::Y` axis
    rotation: Quat,
    // Color of the cone
    color: Color,

    // Number of segments used to approximate the cone base geometry
    base_segments: usize,

    // Number of segments used to approximate the cone height geometry
    height_segments: usize,
}

impl<Config, Clear> Cone3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to approximate the cone geometry for its base and height.
    pub fn segments(mut self, segments: usize) -> Self {
        self.base_segments = segments;
        self.height_segments = segments;
        self
    }

    /// Set the number of segments to approximate the height of the cone geometry.
    ///
    /// `segments` should be a multiple of the value passed to [`Self::height_segments`]
    /// for the height to connect properly with the base.
    pub fn base_segments(mut self, segments: usize) -> Self {
        self.base_segments = segments;
        self
    }

    /// Set the number of segments to approximate the height of the cone geometry.
    ///
    /// `segments` should be a divisor of the value passed to [`Self::base_segments`]
    /// for the height to connect properly with the base.
    pub fn height_segments(mut self, segments: usize) -> Self {
        self.height_segments = segments;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Cone> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Cone3dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Cone,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Cone3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            height: primitive.height,
            position,
            rotation,
            color: color.into(),
            base_segments: DEFAULT_NUMBER_SEGMENTS,
            height_segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<Config, Clear> Drop for Cone3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let Cone3dBuilder {
            gizmos,
            radius,
            height,
            position,
            rotation,
            color,
            base_segments,
            height_segments,
        } = self;

        let half_height = *height * 0.5;

        // draw the base circle of the cone
        draw_circle_3d(
            gizmos,
            *radius,
            *base_segments,
            *rotation,
            *position - *rotation * Vec3::Y * half_height,
            *color,
        );

        // connect the base circle with the tip of the cone
        let end = Vec3::Y * half_height;
        circle_coordinates(*radius, *height_segments)
            .map(|p| Vec3::new(p.x, -half_height, p.y))
            .map(move |p| [p, end])
            .map(|ps| ps.map(rotate_then_translate_3d(*rotation, *position)))
            .for_each(|[start, end]| {
                gizmos.line(start, end, *color);
            });
    }
}

// conical frustum 3d

/// Builder for configuring the drawing options of [`ConicalFrustum`].
pub struct ConicalFrustum3dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the top circle
    radius_top: f32,
    // Radius of the bottom circle
    radius_bottom: f32,
    // Height of the conical frustum
    height: f32,

    // Center of conical frustum, half-way between the top and the bottom
    position: Vec3,
    // Rotation of the conical frustum
    //
    // default orientation is: conical frustum base shape normals are aligned with `Vec3::Y` axis
    rotation: Quat,
    // Color of the conical frustum
    color: Color,

    // Number of segments used to approximate the curved surfaces
    segments: usize,
}

impl<Config, Clear> ConicalFrustum3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to approximate the curved surfaces.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, Config, Clear> GizmoPrimitive3d<ConicalFrustum> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = ConicalFrustum3dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: ConicalFrustum,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        ConicalFrustum3dBuilder {
            gizmos: self,
            radius_top: primitive.radius_top,
            radius_bottom: primitive.radius_bottom,
            height: primitive.height,
            position,
            rotation,
            color: color.into(),
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<Config, Clear> Drop for ConicalFrustum3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let ConicalFrustum3dBuilder {
            gizmos,
            radius_top,
            radius_bottom,
            height,
            position,
            rotation,
            color,
            segments,
        } = self;

        let half_height = *height * 0.5;
        let normal = *rotation * Vec3::Y;

        // draw the two circles of the conical frustum
        [(*radius_top, half_height), (*radius_bottom, -half_height)]
            .into_iter()
            .for_each(|(radius, height)| {
                draw_circle_3d(
                    gizmos,
                    radius,
                    *segments,
                    *rotation,
                    *position + height * normal,
                    *color,
                );
            });

        // connect the two circles of the conical frustum
        circle_coordinates(*radius_top, *segments)
            .map(move |p| Vec3::new(p.x, half_height, p.y))
            .zip(
                circle_coordinates(*radius_bottom, *segments)
                    .map(|p| Vec3::new(p.x, -half_height, p.y)),
            )
            .map(|(start, end)| [start, end])
            .map(|ps| ps.map(rotate_then_translate_3d(*rotation, *position)))
            .for_each(|[start, end]| {
                gizmos.line(start, end, *color);
            });
    }
}

// torus 3d

/// Builder for configuring the drawing options of [`Torus`].
pub struct Torus3dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the minor circle (tube)
    minor_radius: f32,
    // Radius of the major circle (ring)
    major_radius: f32,

    // Center of the torus
    position: Vec3,
    // Rotation of the conical frustum
    //
    // default orientation is: major circle normal is aligned with `Vec3::Y` axis
    rotation: Quat,
    // Color of the torus
    color: Color,

    // Number of segments in the minor (tube) direction
    minor_segments: usize,
    // Number of segments in the major (ring) direction
    major_segments: usize,
}

impl<Config, Clear> Torus3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
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

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Torus> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = Torus3dBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Torus,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Torus3dBuilder {
            gizmos: self,
            minor_radius: primitive.minor_radius,
            major_radius: primitive.major_radius,
            position,
            rotation,
            color: color.into(),
            minor_segments: DEFAULT_NUMBER_SEGMENTS,
            major_segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<Config, Clear> Drop for Torus3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let Torus3dBuilder {
            gizmos,
            minor_radius,
            major_radius,
            position,
            rotation,
            color,
            minor_segments,
            major_segments,
        } = self;

        let normal = *rotation * Vec3::Y;

        // draw 4 circles with major_radius
        [
            (*major_radius - *minor_radius, 0.0),
            (*major_radius + *minor_radius, 0.0),
            (*major_radius, *minor_radius),
            (*major_radius, -*minor_radius),
        ]
        .into_iter()
        .for_each(|(radius, height)| {
            draw_circle_3d(
                gizmos,
                radius,
                *major_segments,
                *rotation,
                *position + height * normal,
                *color,
            );
        });

        // along the major circle draw orthogonal minor circles
        let affine = rotate_then_translate_3d(*rotation, *position);
        circle_coordinates(*major_radius, *major_segments)
            .map(|p| Vec3::new(p.x, 0.0, p.y))
            .flat_map(|major_circle_point| {
                let minor_center = affine(major_circle_point);

                // direction facing from the center of the torus towards the minor circles center
                let dir_to_translation = (minor_center - *position).normalize();

                // the minor circle is draw with 4 arcs this is done to make the minor circle
                // connect properly with each of the major circles
                let circle_points = [dir_to_translation, normal, -dir_to_translation, -normal]
                    .map(|offset| minor_center + offset.normalize() * *minor_radius);
                circle_points
                    .into_iter()
                    .zip(circle_points.into_iter().cycle().skip(1))
                    .map(move |(from, to)| (minor_center, from, to))
                    .collect::<Vec<_>>()
            })
            .for_each(|(center, from, to)| {
                gizmos
                    .short_arc_3d_between(center, from, to, *color)
                    .segments(*minor_segments);
            });
    }
}

// tetrahedron

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Tetrahedron> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Tetrahedron,
        position: Vec3,
        rotation: Quat,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let [a, b, c, d] = primitive
            .vertices
            .map(rotate_then_translate_3d(rotation, position));

        let lines = [(a, b), (a, c), (a, d), (b, c), (b, d), (c, d)];

        let color = color.into();
        for (a, b) in lines.into_iter() {
            self.line(a, b, color);
        }
    }
}
