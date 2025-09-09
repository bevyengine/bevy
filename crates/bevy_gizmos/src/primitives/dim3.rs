//! A module for rendering each of the 3D [`bevy_math::primitives`] with [`GizmoBuffer`].

use super::helpers::*;

use bevy_color::Color;
use bevy_math::{
    primitives::{
        Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, Line3d, Plane3d, Polyline3d,
        Primitive3d, Segment3d, Sphere, Tetrahedron, Torus, Triangle3d,
    },
    Dir3, Isometry3d, Quat, UVec2, Vec2, Vec3,
};

use crate::{circles::SphereBuilder, gizmos::GizmoBuffer, prelude::GizmoConfigGroup};

const DEFAULT_RESOLUTION: u32 = 5;
// length used to simulate infinite lines
const INFINITE_LEN: f32 = 10_000.0;

/// A trait for rendering 3D geometric primitives (`P`) with [`GizmoBuffer`].
pub trait GizmoPrimitive3d<P: Primitive3d> {
    /// The output of `primitive_3d`. This is a builder to set non-default values.
    type Output<'a>
    where
        Self: 'a;

    /// Renders a 3D primitive with its associated details.
    fn primitive_3d(
        &mut self,
        primitive: &P,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_>;
}

// direction 3d

impl<Config, Clear> GizmoPrimitive3d<Dir3> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Dir3,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        let isometry = isometry.into();
        let start = Vec3::ZERO;
        let end = primitive.as_vec3();
        self.arrow(isometry * start, isometry * end, color);
    }
}

// sphere

impl<Config, Clear> GizmoPrimitive3d<Sphere> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = SphereBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Sphere,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        self.sphere(isometry, primitive.radius, color)
    }
}

// plane 3d

/// Builder for configuring the drawing options of [`Plane3d`].
pub struct Plane3dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

    // Direction of the normal orthogonal to the plane
    normal: Dir3,

    isometry: Isometry3d,
    // Color of the plane
    color: Color,

    // Defines the amount of cells in the x and y axes
    cell_count: UVec2,
    // Defines the distance between cells along the x and y axes
    spacing: Vec2,
}

impl<Config, Clear> Plane3dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of cells in the x and y axes direction.
    pub fn cell_count(mut self, cell_count: UVec2) -> Self {
        self.cell_count = cell_count;
        self
    }

    /// Set the distance between cells along the x and y axes.
    pub fn spacing(mut self, spacing: Vec2) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<Config, Clear> GizmoPrimitive3d<Plane3d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Plane3dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Plane3d,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Plane3dBuilder {
            gizmos: self,
            normal: primitive.normal,
            isometry: isometry.into(),
            color: color.into(),
            cell_count: UVec2::splat(3),
            spacing: Vec2::splat(1.0),
        }
    }
}

impl<Config, Clear> Drop for Plane3dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        self.gizmos
            .primitive_3d(&self.normal, self.isometry, self.color);
        // the default orientation of the grid is Z-up
        let rot = Quat::from_rotation_arc(Vec3::Z, self.normal.as_vec3());
        self.gizmos.grid(
            Isometry3d::new(self.isometry.translation, self.isometry.rotation * rot),
            self.cell_count,
            self.spacing,
            self.color,
        );
    }
}

// line 3d

impl<Config, Clear> GizmoPrimitive3d<Line3d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Line3d,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();
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

impl<Config, Clear> GizmoPrimitive3d<Segment3d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Segment3d,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let transformed = primitive.transformed(isometry);
        self.line(transformed.point1(), transformed.point2(), color);
    }
}

// polyline 3d

impl<Config, Clear> GizmoPrimitive3d<Polyline3d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Polyline3d,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();
        self.linestrip(
            primitive.vertices.iter().map(|vec3| isometry * *vec3),
            color,
        );
    }
}

// triangle 3d

impl<Config, Clear> GizmoPrimitive3d<Triangle3d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Triangle3d,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();
        let [a, b, c] = primitive.vertices;
        self.linestrip([a, b, c, a].map(|vec3| isometry * vec3), color);
    }
}

// cuboid

impl<Config, Clear> GizmoPrimitive3d<Cuboid> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Cuboid,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

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
pub struct Cylinder3dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

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

impl<Config, Clear> Cylinder3dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines used to approximate the top and bottom of the cylinder geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> GizmoPrimitive3d<Cylinder> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Cylinder3dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Cylinder,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Cylinder3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_height: primitive.half_height,
            isometry: isometry.into(),
            color: color.into(),
            resolution: DEFAULT_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for Cylinder3dBuilder<'_, Config, Clear>
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
pub struct Capsule3dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

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

impl<Config, Clear> Capsule3dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive3d<Capsule3d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Capsule3dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Capsule3d,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Capsule3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            half_length: primitive.half_length,
            isometry: isometry.into(),
            color: color.into(),
            resolution: DEFAULT_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for Capsule3dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let [upper_apex, lower_apex] = [-1.0, 1.0]
            .map(|sign| Vec3::Y * sign * (self.half_length + self.radius))
            .map(|vec3| self.isometry * vec3);
        let [upper_center, lower_center] = [-1.0, 1.0]
            .map(|sign| Vec3::Y * sign * self.half_length)
            .map(|vec3| self.isometry * vec3);
        let [upper_points, lower_points] = [-1.0, 1.0]
            .map(|sign| Vec3::Y * sign * self.half_length)
            .map(|vec3| {
                circle_coordinates_closed(self.radius, self.resolution)
                    .map(|vec2| Vec3::new(vec2.x, 0.0, vec2.y) + vec3)
                    .map(|vec3| self.isometry * vec3)
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

        let circle_rotation = self
            .isometry
            .rotation
            .mul_quat(Quat::from_rotation_x(core::f32::consts::FRAC_PI_2));
        self.gizmos.circle(
            Isometry3d::new(upper_center, circle_rotation),
            self.radius,
            self.color,
        );
        self.gizmos.circle(
            Isometry3d::new(lower_center, circle_rotation),
            self.radius,
            self.color,
        );

        let connection_lines = upper_points.into_iter().zip(lower_points).skip(1);
        connection_lines.for_each(|(start, end)| {
            self.gizmos.line(start, end, self.color);
        });
    }
}

// cone 3d

/// Builder for configuring the drawing options of [`Cone`].
pub struct Cone3dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

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

impl<Config, Clear> Cone3dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive3d<Cone> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Cone3dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Cone,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Cone3dBuilder {
            gizmos: self,
            radius: primitive.radius,
            height: primitive.height,
            isometry: isometry.into(),
            color: color.into(),
            base_resolution: DEFAULT_RESOLUTION,
            height_resolution: DEFAULT_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for Cone3dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let half_height = self.height * 0.5;
        let apex = self.isometry * (Vec3::Y * half_height);
        let circle_center = half_height * Vec3::NEG_Y;
        let circle_coords = circle_coordinates_closed(self.radius, self.height_resolution)
            .map(|vec2| Vec3::new(vec2.x, 0.0, vec2.y) + circle_center)
            .map(|vec3| self.isometry * vec3)
            .collect::<Vec<_>>();

        // connections to apex
        circle_coords
            .iter()
            .skip(1)
            .map(|vec3| (*vec3, apex))
            .for_each(|(start, end)| {
                self.gizmos.line(start, end, self.color);
            });

        // base circle
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
pub struct ConicalFrustum3dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

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

impl<Config, Clear> ConicalFrustum3dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive3d<ConicalFrustum> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ConicalFrustum3dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &ConicalFrustum,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        ConicalFrustum3dBuilder {
            gizmos: self,
            radius_top: primitive.radius_top,
            radius_bottom: primitive.radius_bottom,
            height: primitive.height,
            isometry: isometry.into(),
            color: color.into(),
            resolution: DEFAULT_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for ConicalFrustum3dBuilder<'_, Config, Clear>
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
                let translation = Vec3::Y * sign * half_height;
                circle_coordinates_closed(radius, self.resolution)
                    .map(|vec2| Vec3::new(vec2.x, 0.0, vec2.y) + translation)
                    .map(|vec3| self.isometry * vec3)
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
pub struct Torus3dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

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

impl<Config, Clear> Torus3dBuilder<'_, Config, Clear>
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

impl<Config, Clear> GizmoPrimitive3d<Torus> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = Torus3dBuilder<'a, Config, Clear>
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Torus,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        Torus3dBuilder {
            gizmos: self,
            minor_radius: primitive.minor_radius,
            major_radius: primitive.major_radius,
            isometry: isometry.into(),
            color: color.into(),
            minor_resolution: DEFAULT_RESOLUTION,
            major_resolution: DEFAULT_RESOLUTION,
        }
    }
}

impl<Config, Clear> Drop for Torus3dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        // draw 4 circles with major_radius
        let [inner, outer, top, bottom] = [
            (self.major_radius - self.minor_radius, 0.0),
            (self.major_radius + self.minor_radius, 0.0),
            (self.major_radius, self.minor_radius),
            (self.major_radius, -self.minor_radius),
        ]
        .map(|(radius, height)| {
            let translation = height * Vec3::Y;
            circle_coordinates_closed(radius, self.major_resolution)
                .map(|vec2| Vec3::new(vec2.x, 0.0, vec2.y) + translation)
                .map(|vec3| self.isometry * vec3)
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
                let center = (inner + top + outer + bottom) * 0.25;
                [(inner, top), (top, outer), (outer, bottom), (bottom, inner)]
                    .map(|(start, end)| (start, end, center))
            })
            .for_each(|(from, to, center)| {
                self.gizmos
                    .short_arc_3d_between(center, from, to, self.color)
                    .resolution(self.minor_resolution);
            });
    }
}

// tetrahedron

impl<Config, Clear> GizmoPrimitive3d<Tetrahedron> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'a>
        = ()
    where
        Self: 'a;

    fn primitive_3d(
        &mut self,
        primitive: &Tetrahedron,
        isometry: impl Into<Isometry3d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        if !self.enabled {
            return;
        }

        let isometry = isometry.into();

        let [a, b, c, d] = primitive.vertices.map(|vec3| isometry * vec3);

        let lines = [(a, b), (a, c), (a, d), (b, c), (b, d), (c, d)];

        let color = color.into();
        lines.into_iter().for_each(|(start, end)| {
            self.line(start, end, color);
        });
    }
}
