//! A module for rendering each of the 3D [`bevy_math::primitives`] with [`Gizmos`].

use super::helpers::*;
use std::f32::consts::TAU;

use bevy_color::Color;
use bevy_math::primitives::{
    BoxedPolyline3d, Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, Line3d, Plane3d,
    Polyline3d, Primitive3d, Segment3d, Sphere, Tetrahedron, Torus, Triangle3d,
};
use bevy_math::{Dir3, Isometry3d, Quat, Vec3};

use crate::circles::SphereBuilder;
use crate::prelude::{GizmoConfigGroup, Gizmos};

const DEFAULT_RESOLUTION: u32 = 5;
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
        primitive: &P,
        isometry: Isometry3d,
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
        primitive: &Dir3,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        let start = Vec3::ZERO;
        let end = primitive.as_vec3();
        self.arrow(isometry * start, isometry * end, color);
    }
}

// sphere

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Sphere> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = SphereBuilder<'a, 'w, 's, Config, Clear> where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Sphere,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.sphere(isometry, primitive.radius, color)
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

    isometry: Isometry3d,
    // Color of the plane
    color: Color,

    // Number of axis to hint the plane
    axis_count: u32,
    // Number of segments used to hint the plane
    segment_count: u32,
    // Length of segments used to hint the plane
    segment_length: f32,
}

impl<Config, Clear> Plane3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of segments used to hint the plane.
    pub fn segment_count(mut self, count: u32) -> Self {
        self.segment_count = count;
        self
    }

    /// Set the length of segments used to hint the plane.
    pub fn segment_length(mut self, length: f32) -> Self {
        self.segment_length = length;
        self
    }

    /// Set the number of axis used to hint the plane.
    pub fn axis_count(mut self, count: u32) -> Self {
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
        primitive: &Plane3d,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Plane3dBuilder {
            gizmos: self,
            normal: primitive.normal,
            isometry,
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
        self.gizmos
            .primitive_3d(&self.normal, self.isometry, self.color);
        let normals_normal = self.normal.any_orthonormal_vector();

        // draws the axes
        // get rotation for each direction
        (0..self.axis_count)
            .map(|i| i as f32 * (1.0 / self.axis_count as f32) * TAU)
            .map(|angle| Quat::from_axis_angle(self.normal.as_vec3(), angle))
            .for_each(|quat| {
                let axis_direction = quat * normals_normal;
                let direction = Dir3::new_unchecked(axis_direction);

                // for each axis draw dotted line
                (0..)
                    .filter(|i| i % 2 != 0)
                    .map(|percent| (percent as f32 + 0.5) * self.segment_length * axis_direction)
                    .map(|position| self.isometry * position)
                    .take(self.segment_count as usize)
                    .for_each(|position| {
                        self.gizmos.primitive_3d(
                            &Segment3d {
                                direction,
                                half_length: self.segment_length * 0.5,
                            },
                            Isometry3d::from_translation(position),
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
        primitive: &Line3d,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let color = color.into();
        let direction = primitive.direction.as_vec3();
        self.arrow(isometry * Vec3::ZERO, isometry * direction, color);

        let [start, end] = [1.0, -1.0]
            .map(|sign| sign * INFINITE_LEN)
            .map(|length| primitive.direction * length)
            .map(|offset| isometry * offset);
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
        primitive: &Segment3d,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let direction = primitive.direction.as_vec3();
        self.line(isometry * direction, isometry * (-direction), color);
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
        primitive: &Polyline3d<N>,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        self.linestrip(primitive.vertices.map(|vec3| isometry * vec3), color);
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
        primitive: &BoxedPolyline3d,
        isometry: Isometry3d,
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
                .map(|vec3| isometry * vec3),
            color,
        );
    }
}

// triangle 3d

impl<'w, 's, Config, Clear> GizmoPrimitive3d<Triangle3d> for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Triangle3d,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let [a, b, c] = primitive.vertices;
        self.linestrip([a, b, c, a].map(|vec3| isometry * vec3), color);
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
        primitive: &Cuboid,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

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
        .map(Vec3::from)
        .map(|vec3| vec3 * primitive.half_size)
        .map(|vec3| isometry * vec3);

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

    isometry: Isometry3d,
    // Color of the cylinder
    color: Color,

    // Number of lines used to approximate the cylinder geometry
    resolution: u32,
}

impl<Config, Clear> Cylinder3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines used to approximate the top an bottom of the cylinder geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
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
        primitive: &Cylinder,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Cylinder3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_height: primitive.half_height,
            isometry,
            color: color.into(),
            resolution: DEFAULT_RESOLUTION,
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

        self.gizmos
            .primitive_3d(
                &ConicalFrustum {
                    radius_top: self.radius,
                    radius_bottom: self.radius,
                    height: self.half_height * 2.0,
                },
                self.isometry,
                self.color,
            )
            .resolution(self.resolution);
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

    isometry: Isometry3d,
    // Color of the capsule
    color: Color,

    // Number of lines used to approximate the capsule geometry
    resolution: u32,
}

impl<Config, Clear> Capsule3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines used to approximate the capsule geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
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
        primitive: &Capsule3d,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Capsule3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_length: primitive.half_length,
            isometry,
            color: color.into(),
            resolution: DEFAULT_RESOLUTION,
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

        let body_height = (self.half_length - self.radius).max(0.0);

        let [upper_apex, lower_apex] = [-1.0, 1.0]
            .map(|sign| Isometry3d::from_translation(Vec3::Y * sign * self.half_length))
            .map(|translation_iso| (translation_iso * self.isometry) * Vec3::ZERO);
        let [upper_center, lower_center] = (body_height != 0.0)
            .then(|| {
                [-1.0, 1.0]
                    .map(|sign| Isometry3d::from_translation(Vec3::Y * sign * body_height))
                    .map(|translation_iso| (translation_iso * self.isometry) * Vec3::ZERO)
            })
            .unwrap_or([Vec3::ZERO; 2]);

        let [upper_points, lower_points] = [-1.0, 1.0]
            .map(|sign| Isometry3d::from_translation(Vec3::Y * sign * body_height))
            .map(|translation_iso| {
                circle_coordinates_closed(self.radius, self.resolution)
                    .map(|vec2| (translation_iso * self.isometry) * Vec3::new(vec2.x, 0.0, vec2.y))
                    .collect::<Vec<_>>()
            });

        upper_points.iter().skip(1).copied().for_each(|start| {
            self.gizmos
                .short_arc_3d_between(upper_center, start, upper_apex, self.color);
        });
        lower_points.iter().skip(1).copied().for_each(|start| {
            self.gizmos
                .short_arc_3d_between(lower_center, start, lower_apex, self.color);
        });

        // don't draw a body of height 0.0
        if body_height != 0.0 {
            let upper_lines = upper_points.windows(2).map(|win| (win[0], win[1]));
            let lower_lines = lower_points.windows(2).map(|win| (win[0], win[1]));
            upper_lines.chain(lower_lines).for_each(|(start, end)| {
                self.gizmos.line(start, end, self.color);
            });

            let connection_lines = upper_points.into_iter().zip(lower_points).skip(1);
            connection_lines.for_each(|(start, end)| {
                self.gizmos.line(start, end, self.color);
            });
        }
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

    isometry: Isometry3d,
    // Color of the cone
    color: Color,

    // Number of lines used to approximate the cone base geometry
    base_resolution: u32,

    // Number of lines used to approximate the cone height geometry
    height_resolution: u32,
}

impl<Config, Clear> Cone3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines used to approximate the cone geometry for its base and height.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.base_resolution = resolution;
        self.height_resolution = resolution;
        self
    }

    /// Set the number of lines used to approximate the height of the cone geometry.
    ///
    /// `resolution` should be a multiple of the value passed to [`Self::height_resolution`]
    /// for the height to connect properly with the base.
    pub fn base_resolution(mut self, resolution: u32) -> Self {
        self.base_resolution = resolution;
        self
    }

    /// Set the number of lines used to approximate the height of the cone geometry.
    ///
    /// `resolution` should be a divisor of the value passed to [`Self::base_resolution`]
    /// for the height to connect properly with the base.
    pub fn height_resolution(mut self, resolution: u32) -> Self {
        self.height_resolution = resolution;
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
        primitive: &Cone,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Cone3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            height: primitive.height,
            isometry,
            color: color.into(),
            base_resolution: DEFAULT_RESOLUTION,
            height_resolution: DEFAULT_RESOLUTION,
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

        let half_height = self.height * 0.5;
        let apex = self.isometry * Vec3::Y * half_height;
        let circle_iso = Isometry3d::from_translation(-half_height * Vec3::Y);
        let circle_coords = circle_coordinates_closed(self.radius, self.height_resolution)
            .map(|vec2| (circle_iso * self.isometry) * vec2.extend(0.0))
            .collect::<Vec<_>>();

        circle_coords
            .iter()
            .skip(1)
            .map(|vec3| (*vec3, apex))
            .for_each(|(start, end)| {
                self.gizmos.line(start, end, self.color);
            });
        circle_coords
            .windows(2)
            .map(|win| (win[0], win[1]))
            .for_each(|(start, end)| {
                self.gizmos.line(start, end, self.color);
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

    isometry: Isometry3d,
    // Color of the conical frustum
    color: Color,

    // Number of lines used to approximate the curved surfaces
    resolution: u32,
}

impl<Config, Clear> ConicalFrustum3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines used to approximate the curved surfaces.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
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
        primitive: &ConicalFrustum,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        ConicalFrustum3dBuilder {
            gizmos: self,
            radius_top: primitive.radius_top,
            radius_bottom: primitive.radius_bottom,
            height: primitive.height,
            isometry,
            color: color.into(),
            resolution: DEFAULT_RESOLUTION,
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

        let half_height = self.height * 0.5;
        let [upper_points, lower_points] = [(-1.0, self.radius_bottom), (1.0, self.radius_top)]
            .map(|(sign, radius)| {
                let translation_iso = Isometry3d::from_translation(Vec3::Y * sign * half_height);
                circle_coordinates_closed(radius, self.resolution)
                    .map(|vec2| (translation_iso * self.isometry) * Vec3::new(vec2.x, 0.0, vec2.y))
                    .collect::<Vec<_>>()
            });

        let upper_lines = upper_points.windows(2).map(|win| (win[0], win[1]));
        let lower_lines = lower_points.windows(2).map(|win| (win[0], win[1]));
        upper_lines.chain(lower_lines).for_each(|(start, end)| {
            self.gizmos.line(start, end, self.color);
        });

        let connection_lines = upper_points.into_iter().zip(lower_points).skip(1);
        connection_lines.for_each(|(start, end)| {
            self.gizmos.line(start, end, self.color);
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

    isometry: Isometry3d,
    // Color of the torus
    color: Color,

    // Number of lines in the minor (tube) direction
    minor_resolution: u32,
    // Number of lines in the major (ring) direction
    major_resolution: u32,
}

impl<Config, Clear> Torus3dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines in the minor (tube) direction.
    pub fn minor_resolution(mut self, minor_resolution: u32) -> Self {
        self.minor_resolution = minor_resolution;
        self
    }

    /// Set the number of lines in the major (ring) direction.
    pub fn major_resolution(mut self, major_resolution: u32) -> Self {
        self.major_resolution = major_resolution;
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
        primitive: &Torus,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Torus3dBuilder {
            gizmos: self,
            minor_radius: primitive.minor_radius,
            major_radius: primitive.major_radius,
            isometry,
            color: color.into(),
            minor_resolution: DEFAULT_RESOLUTION,
            major_resolution: DEFAULT_RESOLUTION,
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

        let center = self.isometry * Vec3::ZERO;
        // draw 4 circles with major_radius
        let [inner, outer, top, bottom] = [
            (self.major_radius - self.minor_radius, 0.0),
            (self.major_radius + self.minor_radius, 0.0),
            (self.major_radius, self.minor_radius),
            (self.major_radius, -self.minor_radius),
        ]
        .map(|(radius, height)| {
            let transformation_iso = Isometry3d::from_translation(height * Vec3::Y);
            circle_coordinates_closed(radius, self.major_resolution)
                .map(|vec2| (transformation_iso * self.isometry) * vec2.extend(0.0))
                .collect::<Vec<_>>()
        });

        [&inner, &outer, &top, &bottom]
            .iter()
            .flat_map(|points| points.windows(2).map(|win| (win[0], win[1])))
            .for_each(|(start, end)| {
                self.gizmos.line(start, end, self.color);
            });

        inner
            .into_iter()
            .zip(top)
            .zip(outer)
            .zip(bottom)
            .flat_map(|(((inner, top), outer), bottom)| {
                [(inner, top), (top, outer), (outer, bottom), (bottom, inner)]
            })
            .for_each(|(from, to)| {
                self.gizmos
                    .short_arc_3d_between(center, from, to, self.color)
                    .resolution(self.minor_resolution);
            });
    }
}

// tetrahedron

impl<'w, 's, T: GizmoConfigGroup> GizmoPrimitive3d<Tetrahedron> for Gizmos<'w, 's, T> {
    type Output<'a> = () where Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Tetrahedron,
        isometry: Isometry3d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let [a, b, c, d] = primitive.vertices.map(|vec3| isometry * vec3);

        let lines = [(a, b), (a, c), (a, d), (b, c), (b, d), (c, d)];

        let color = color.into();
        lines.into_iter().for_each(|(start, end)| {
            self.line(start, end, color);
        });
    }
}
