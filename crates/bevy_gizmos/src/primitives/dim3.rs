//! A module for rendering each of the 3D [`bevy_math::primitives`] with [`Gizmos`].

use super::helpers::*;
use std::f32::consts::TAU;

use bevy_math::primitives::{
    BoxedPolyline3d, Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, Direction3d, Line3d,
    Plane3d, Polyline3d, Primitive3d, Segment3d, Sphere, Torus,
};
use bevy_math::{Quat, Vec2, Vec3};
use bevy_render::color::Color;

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
        color: Color,
    ) -> Self::Output<'_>;
}

// direction 3d

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Direction3d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Direction3d,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        self.arrow(position, position + (rotation * *primitive), color);
    }
}

// sphere

/// Builder for configuring the drawing options of [`Sphere`].
pub struct SphereBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    radius: f32, // Radius of the sphere

    rotation: Quat, // Rotation of the sphere around the origin in 3D space
    position: Vec3, // Center position of the sphere in 3D space
    color: Color,   // Color of the sphere

    segments: usize, // Number of segments used to approximate the sphere geometry
}

impl<T: GizmoConfigGroup> SphereBuilder<'_, '_, '_, T> {
    /// Set the number of segments used to approximate the sphere geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Sphere> for Gizmos<'w, 's, T> {
    type Output<'a> = SphereBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Sphere,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        SphereBuilder {
            gizmos: self,
            radius: primitive.radius,
            position,
            rotation,
            color,
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for SphereBuilder<'_, '_, '_, T> {
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

        // draw two caps, one for the "upper half" and one for the "lower" half of the sphere
        [-1.0, 1.0].into_iter().for_each(|sign| {
            let top = *center + (*rotation * Vec3::Y) * sign * *radius;
            draw_cap(
                self.gizmos,
                *radius,
                *segments,
                *rotation,
                *center,
                top,
                *color,
            );
        });

        draw_circle(self.gizmos, *radius, *segments, *rotation, *center, *color);
    }
}

// plane 3d

/// Builder for configuring the drawing options of [`Sphere`].
pub struct Plane3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    normal: Direction3d, // direction of the normal orthogonal to the plane

    rotation: Quat, // Rotation of the sphere around the origin in 3D space
    position: Vec3, // Center position of the sphere in 3D space
    color: Color,   // Color of the sphere

    num_axis: usize,     // Number of axis to hint the plane
    num_segments: usize, // Number of segments used to hint the plane
    len_segments: f32,   // Length of segments used to hint the plane
}

impl<T: GizmoConfigGroup> Plane3dBuilder<'_, '_, '_, T> {
    /// Set the number of segments used to hint the plane.
    pub fn segments(mut self, segments: usize) -> Self {
        self.num_segments = segments;
        self
    }

    /// Set the length of segments used to hint the plane.
    pub fn len_segments(mut self, length: f32) -> Self {
        self.len_segments = length;
        self
    }

    /// Set the number of hinting axis used to hint the plane.
    pub fn num_axis(mut self, num_axis: usize) -> Self {
        self.num_axis = num_axis;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Plane3d> for Gizmos<'w, 's, T> {
    type Output<'a> = Plane3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Plane3d,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        Plane3dBuilder {
            gizmos: self,
            normal: primitive.normal,
            rotation,
            position,
            color,
            num_axis: 4,
            num_segments: 3,
            len_segments: 0.25,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Plane3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let normal = self.rotation * *self.normal;
        self.gizmos
            .primitive_3d(self.normal, self.position, self.rotation, self.color);
        let normals_normal = normal.any_orthonormal_vector();

        // get rotation for each direction
        (0..self.num_axis)
            .map(|i| i as f32 * (1.0 / self.num_axis as f32) * TAU)
            .map(|angle| Quat::from_axis_angle(normal, angle))
            .for_each(|quat| {
                let axis_direction = quat * normals_normal;
                let direction = Direction3d::new_unchecked(axis_direction);

                // for each axis draw dotted line
                (0..)
                    .filter(|i| i % 2 == 0)
                    .map(|percent| (percent as f32 + 0.5) * self.len_segments * axis_direction)
                    .map(|position| position + self.position)
                    .take(self.num_segments)
                    .for_each(|position| {
                        self.gizmos.primitive_3d(
                            Segment3d {
                                direction,
                                half_length: self.len_segments * 0.5,
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Line3d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Line3d,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Segment3d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Segment3d,
        position: Vec3,
        rotation: Quat,
        color: Color,
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

impl<'w, 's, const N: usize, T: GizmoConfigGroup> GizmoPrimitive3d<Polyline3d<N>>
    for Gizmos<'w, 's, T>
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Polyline3d<N>,
        position: Vec3,
        rotation: Quat,
        color: Color,
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<BoxedPolyline3d> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: BoxedPolyline3d,
        position: Vec3,
        rotation: Quat,
        color: Color,
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

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Cuboid> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Cuboid,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let [half_extend_x, half_extend_y, half_extend_z] = primitive.half_size.to_array();

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

// cylinder 3d

/// Builder for configuring the drawing options of [`Cylinder3d`].
pub struct Cylinder3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

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

impl<T: GizmoConfigGroup> Cylinder3dBuilder<'_, '_, '_, T> {
    /// Set the number of segments used to approximate the cylinder geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Cylinder> for Gizmos<'w, 's, T> {
    type Output<'a> = Cylinder3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Cylinder,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        Cylinder3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_height: primitive.half_height,
            position,
            rotation,
            color,
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Cylinder3dBuilder<'_, '_, '_, T> {
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

        [-1.0, 1.0].into_iter().for_each(|sign| {
            draw_circle(
                gizmos,
                *radius,
                *segments,
                *rotation,
                *position + sign * *half_height * normal,
                *color,
            );
        });

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
pub struct Capsule3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

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

impl<T: GizmoConfigGroup> Capsule3dBuilder<'_, '_, '_, T> {
    /// Set the number of segments used to approximate the capsule geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Capsule3d> for Gizmos<'w, 's, T> {
    type Output<'a> = Capsule3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Capsule3d,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        Capsule3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_length: primitive.half_length,
            position,
            rotation,
            color,
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Capsule3dBuilder<'_, '_, '_, T> {
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

        [1.0, -1.0].into_iter().for_each(|sign| {
            let center = *position + sign * *half_length * normal;
            let top = center + sign * *radius * normal;
            draw_cap(gizmos, *radius, *segments, *rotation, center, top, *color);
            draw_circle(gizmos, *radius, *segments, *rotation, center, *color);
        });

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

/// Builder for configuring the drawing options of [`Cone3d`].
pub struct Cone3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

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

    segments: usize, // Number of segments used to approximate the cone geometry
}

impl<T: GizmoConfigGroup> Cone3dBuilder<'_, '_, '_, T> {
    /// Set the number of segments used to approximate the cone geometry.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Cone> for Gizmos<'w, 's, T> {
    type Output<'a> = Cone3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: Cone,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        Cone3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            height: primitive.height,
            position,
            rotation,
            color,
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Cone3dBuilder<'_, '_, '_, T> {
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
            segments,
        } = self;

        let half_height = *height * 0.5;

        {
            let positions = (0..=*segments)
                .map(|frac| frac as f32 / *segments as f32)
                .map(|percentage| percentage * TAU)
                .map(|angle| Vec2::from(angle.sin_cos()) * *radius)
                .map(|p| Vec3::new(p.x, -half_height, p.y))
                .map(rotate_then_translate_3d(*rotation, *position));
            gizmos.linestrip(positions, *color);
        };

        let end = Vec3::Y * half_height;
        circle_coordinates(*radius, *segments)
            .map(|p| Vec3::new(p.x, -half_height, p.y))
            .map(move |p| [p, end])
            .map(|ps| ps.map(rotate_then_translate_3d(*rotation, *position)))
            .for_each(|[start, end]| {
                gizmos.line(start, end, *color);
            });
    }
}

// conical frustum 3d

/// Builder for configuring the drawing options of [`ConicalFrustum3d`].
pub struct ConicalFrustum3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    // Radius of the top circle
    radius_top: f32,
    // Radius of the bottom circle
    radius_bottom: f32,
    // Height of the conical frustum
    height: f32,

    // Center of conical frustum, half-way between the top and the bottom
    position: Vec3,
    // Rotation of the conical frustrum
    //
    // default orientation is: conical frustrum base shape normals are aligned with `Vec3::Y` axis
    rotation: Quat,
    // Color of the conical frustum
    color: Color,

    segments: usize, // Number of segments used to approximate the curved surfaces
}

impl<T: GizmoConfigGroup> ConicalFrustum3dBuilder<'_, '_, '_, T> {
    /// Set the number of segments used to approximate the curved surfaces.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<ConicalFrustum> for Gizmos<'w, 's, T> {
    type Output<'a> = ConicalFrustum3dBuilder<'a, 'w, 's, T> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: ConicalFrustum,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        ConicalFrustum3dBuilder {
            gizmos: self,
            radius_top: primitive.radius_top,
            radius_bottom: primitive.radius_bottom,
            height: primitive.height,
            position,
            rotation,
            color,
            segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for ConicalFrustum3dBuilder<'_, '_, '_, T> {
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
        [(*radius_top, half_height), (*radius_bottom, -half_height)]
            .into_iter()
            .for_each(|(radius, height)| {
                draw_circle(
                    gizmos,
                    radius,
                    *segments,
                    *rotation,
                    *position + height * normal,
                    *color,
                );
            });

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

/// Builder for configuring the drawing options of [`Torus3d`].
pub struct Torus3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,

    // Radius of the minor circle (tube)
    minor_radius: f32,
    // Radius of the major circle (ring)
    major_radius: f32,

    // Center of the torus
    position: Vec3,
    // Rotation of the conical frustrum
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

impl<T: GizmoConfigGroup> Torus3dBuilder<'_, '_, '_, T> {
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

    fn primitive_3d(
        &mut self,
        primitive: Torus,
        position: Vec3,
        rotation: Quat,
        color: Color,
    ) -> Self::Output<'_> {
        Torus3dBuilder {
            gizmos: self,
            minor_radius: primitive.minor_radius,
            major_radius: primitive.major_radius,
            position,
            rotation,
            color,
            minor_segments: DEFAULT_NUMBER_SEGMENTS,
            major_segments: DEFAULT_NUMBER_SEGMENTS,
        }
    }
}

impl<T: GizmoConfigGroup> Drop for Torus3dBuilder<'_, '_, '_, T> {
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
                *rotation,
                *position + height * normal,
                *color,
            );
        });

        let affine = rotate_then_translate_3d(*rotation, *position);
        circle_coordinates(*major_radius, *major_segments)
            .map(|p| Vec3::new(p.x, 0.0, p.y))
            .flat_map(|major_circle_point| {
                let minor_center = affine(major_circle_point);
                let dir_to_translation = (minor_center - *position).normalize();
                let points = [dir_to_translation, normal, -dir_to_translation, -normal];
                let points = points.map(|offset| minor_center + offset.normalize() * *minor_radius);

                points
                    .into_iter()
                    .zip(points.into_iter().cycle().skip(1))
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
