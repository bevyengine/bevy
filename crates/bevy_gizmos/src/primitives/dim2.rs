//! A module for rendering each of the 2D [`bevy_math::primitives`] with [`GizmoBuffer`].
//!
//! This mirrors the `Meshable` implementation:
//!
//! 1. A trait `ToGizmoBlueprint2d` to take a primitive and generate a blueprint / descriptor / builder from it
//! 2. A trait that uses the data in the blueprint to render something
//!
//! There is also an extension trait [`GizmoPrimitive2d<P>`](GizmoPrimitive2d) that is implemented for `P` on [`GizmoBuffer`]

use core::f32::consts::{FRAC_PI_2, PI};

use super::helpers::*;

use bevy_color::Color;
use bevy_math::{
    primitives::{
        Annulus, Arc2d, Capsule2d, Circle, CircularSector, CircularSegment, Ellipse, Line2d,
        Plane2d, Polygon, Polyline2d, Primitive2d, Rectangle, RegularPolygon, Rhombus, Ring,
        Segment2d, Triangle2d,
    },
    Dir2, Isometry2d, Rot2, Vec2,
};

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};

// some magic number since using directions as offsets will result in lines of length 1 pixel
const MIN_LINE_LEN: f32 = 50.0;
const HALF_MIN_LINE_LEN: f32 = 25.0;
// length used to simulate infinite lines
const INFINITE_LEN: f32 = 100_000.0;

/// A trait for rendering 2D geometric primitives (`P`) with [`GizmoBuffer`].
///
/// When implementing `GizmoPrimitive2d<P>`, if you require a builder `MyBuilder` to set non-default values,
/// implement [`GizmoBlueprint2d`] for the builder `MyBuilder` and set `type Output<'builder, 'primitive> = GizmoBuilder2d<'builder, MyBuilder, Config, Clear>` ([`GizmoBuilder2d`]).
///
/// If you don't require a custom builder you can use [`NoConfigBuilder2d`] (`type Output<'builder, 'primitive> = GizmoBuilder2d<'builder, NoConfigBuilder2d<P>, Config, Clear>`)
pub trait GizmoPrimitive2d<P: Primitive2d> {
    /// The output of `primitive_2d`. This is a builder to set non-default values.
    ///
    /// If you do not require a builder, you can set `type Output<'builder, 'primitive> = ()`.
    type Output<'builder, 'primitive>
    where
        Self: 'builder,
        P: 'primitive;

    /// Renders a 2D primitive with its associated details.
    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive P,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive>;
}

/// A type that provides the data to construct a Gizmo.
///
/// See the documentation for [`GizmoPrimitive2d`].
pub trait GizmoBlueprint2d {
    /// This is run on drop of [`GizmoBuilder2d`] to update the `gizmos`
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync;
}

/// A type that generates a builder `Blueprint2d`
pub trait ToGizmoBlueprint2d {
    /// The builder type.
    ///
    /// Supports borrowing the data from the primitive if required
    type Blueprint2d<'primitive>: GizmoBlueprint2d
    where
        Self: 'primitive;
    /// Construct the builder type
    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_>;
}

/// This is essentially a scope guard that runs the inner builder's [`GizmoBlueprint2d::build_2d`] method on drop.
///
/// Provides access to the "blueprint" via the `Deref`/`DerefMut` traits for configuration purposes.
///
/// See the documentation for [`GizmoPrimitive2d`].
pub struct GizmoBuilder2d<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'builder mut GizmoBuffer<Config, Clear>,
    data: Blueprint,
}

impl<'builder, Blueprint, Config, Clear> GizmoBuilder2d<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Construct a new `GizmoBuilder` from a `GizmoBuffer` and `Data` that implements `Primitive2dGizmoBuilder`
    pub fn new(gizmos: &'builder mut GizmoBuffer<Config, Clear>, data: Blueprint) -> Self {
        Self { gizmos, data }
    }
}

impl<'builder, Blueprint, Config, Clear> core::ops::Deref
    for GizmoBuilder2d<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Target = Blueprint;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'builder, Blueprint, Config, Clear> core::ops::DerefMut
    for GizmoBuilder2d<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'builder, Blueprint, Config, Clear> Drop for GizmoBuilder2d<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        let GizmoBuilder2d { gizmos, data } = self;
        if !gizmos.enabled {
            return;
        }
        data.build_2d(gizmos);
    }
}

/// A simple "builder" without configuration options that simply wraps a primitive.
/// This is useful to avoid boilerplate for [`GizmoBlueprint2d`], [`ToGizmoBlueprint2d`] and [`GizmoPrimitive2d`] trait implementations.
pub struct NoConfigBuilder2d<P> {
    /// The primitive provided by the caller in [`ToGizmoBlueprint2d::to_blueprint_2d`] or  [`GizmoPrimitive2d::primitive_2d`]
    pub primitive: P,
    /// The isometry provided by the caller in [`ToGizmoBlueprint2d::to_blueprint_2d`] or  [`GizmoPrimitive2d::primitive_2d`]
    pub isometry: Isometry2d,
    /// The color provided by the caller in [`ToGizmoBlueprint2d::to_blueprint_2d`] or  [`GizmoPrimitive2d::primitive_2d`]
    pub color: Color,
}

impl<P> NoConfigBuilder2d<P> {
    /// Construct a new `NoConfigBuilder2d` for an inner primitive `P`
    pub fn new(inner: P, isometry: impl Into<Isometry2d>, color: impl Into<Color>) -> Self {
        Self {
            primitive: inner,
            isometry: isometry.into(),
            color: color.into(),
        }
    }
}

// direction 2d

impl<Config, Clear> GizmoPrimitive2d<Dir2> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Dir2>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Dir2,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Dir2> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start = Vec2::ZERO;
        let end = self.primitive * MIN_LINE_LEN;
        gizmos.arrow_2d(self.isometry * start, self.isometry * end, self.color);
    }
}

impl ToGizmoBlueprint2d for Dir2 {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Dir2>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// arc 2d

impl<Config, Clear> GizmoPrimitive2d<Arc2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Arc2d>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Arc2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Arc2d> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start_iso =
            self.isometry * Isometry2d::from_rotation(Rot2::radians(-self.primitive.half_angle));

        gizmos.arc_2d(
            start_iso,
            self.primitive.half_angle * 2.0,
            self.primitive.radius,
            self.color,
        );
    }
}

impl ToGizmoBlueprint2d for Arc2d {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Arc2d>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// circle 2d

impl<Config, Clear> GizmoPrimitive2d<Circle> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, crate::circles::Ellipse2dBuilder, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Circle,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        self.circle_2d(isometry, primitive.radius, color)
    }
}

// implementation of Blueprint traits for Circle are in module circles

// circular sector 2d

impl<Config, Clear> GizmoPrimitive2d<CircularSector> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<CircularSector>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive CircularSector,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<CircularSector> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start_iso = self.isometry
            * Isometry2d::from_rotation(Rot2::radians(-self.primitive.arc.half_angle));
        let end_iso =
            self.isometry * Isometry2d::from_rotation(Rot2::radians(self.primitive.arc.half_angle));

        // we need to draw the arc part of the sector, and the two lines connecting the arc and the center
        gizmos.arc_2d(
            start_iso,
            self.primitive.arc.half_angle * 2.0,
            self.primitive.arc.radius,
            self.color,
        );

        let end_position = self.primitive.arc.radius * Vec2::Y;
        gizmos.line_2d(
            self.isometry * Vec2::ZERO,
            start_iso * end_position,
            self.color,
        );
        gizmos.line_2d(
            self.isometry * Vec2::ZERO,
            end_iso * end_position,
            self.color,
        );
    }
}

impl ToGizmoBlueprint2d for CircularSector {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<CircularSector>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// circular segment 2d

impl<Config, Clear> GizmoPrimitive2d<CircularSegment> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<CircularSegment>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive CircularSegment,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<CircularSegment> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start_iso = self.isometry
            * Isometry2d::from_rotation(Rot2::radians(-self.primitive.arc.half_angle));
        let end_iso =
            self.isometry * Isometry2d::from_rotation(Rot2::radians(self.primitive.arc.half_angle));

        // we need to draw the arc part of the segment, and the line connecting the two ends
        gizmos.arc_2d(
            start_iso,
            self.primitive.arc.half_angle * 2.0,
            self.primitive.arc.radius,
            self.color,
        );

        let position = self.primitive.arc.radius * Vec2::Y;
        gizmos.line_2d(start_iso * position, end_iso * position, self.color);
    }
}

impl ToGizmoBlueprint2d for CircularSegment {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<CircularSegment>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// ellipse 2d

impl<Config, Clear> GizmoPrimitive2d<Ellipse> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, crate::circles::Ellipse2dBuilder, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Ellipse,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        self.ellipse_2d(isometry, primitive.half_size, color)
    }
}

// implementation of Blueprint traits for Ellipse are in module circles

// annulus 2d

/// Builder for configuring the drawing options of [`Annulus`].
pub struct Annulus2dBuilder {
    isometry: Isometry2d,
    inner_radius: f32,
    outer_radius: f32,
    color: Color,
    inner_resolution: u32,
    outer_resolution: u32,
}

impl Annulus2dBuilder {
    /// Set the number of line-segments for each circle of the annulus.
    pub fn resolution(&mut self, resolution: u32) -> &mut Self {
        self.outer_resolution = resolution;
        self.inner_resolution = resolution;
        self
    }

    /// Set the number of line-segments for the outer circle of the annulus.
    pub fn outer_resolution(&mut self, resolution: u32) -> &mut Self {
        self.outer_resolution = resolution;
        self
    }

    /// Set the number of line-segments for the inner circle of the annulus.
    pub fn inner_resolution(&mut self, resolution: u32) -> &mut Self {
        self.inner_resolution = resolution;
        self
    }
}

impl<Config, Clear> GizmoPrimitive2d<Annulus> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, Annulus2dBuilder, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Annulus,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for Annulus2dBuilder {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let Annulus2dBuilder {
            isometry,
            inner_radius,
            outer_radius,
            inner_resolution,
            outer_resolution,
            color,
            ..
        } = self;

        gizmos
            .circle_2d(*isometry, *outer_radius, *color)
            .resolution(*outer_resolution);
        gizmos
            .circle_2d(*isometry, *inner_radius, *color)
            .resolution(*inner_resolution);
    }
}

impl ToGizmoBlueprint2d for Annulus {
    type Blueprint2d<'primitive> = Annulus2dBuilder;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        Annulus2dBuilder {
            isometry: isometry.into(),
            inner_radius: self.inner_circle.radius,
            outer_radius: self.outer_circle.radius,
            color: color.into(),
            inner_resolution: crate::circles::DEFAULT_CIRCLE_RESOLUTION,
            outer_resolution: crate::circles::DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

// rhombus 2d

impl<Config, Clear> GizmoPrimitive2d<Rhombus> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Rhombus>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Rhombus,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Rhombus> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [a, b, c, d] =
            [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    self.primitive.half_diagonals.x * sign_x,
                    self.primitive.half_diagonals.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(|vec2| self.isometry * vec2);
        gizmos.linestrip_2d(positions, self.color);
    }
}

impl ToGizmoBlueprint2d for Rhombus {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Rhombus>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// capsule 2d

impl<Config, Clear> GizmoPrimitive2d<Capsule2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Capsule2d>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Capsule2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Capsule2d> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        // transform points from the reference unit square to capsule "rectangle"
        let [top_left, top_right, bottom_left, bottom_right, top_center, bottom_center] = [
            [-1.0, 1.0],
            [1.0, 1.0],
            [-1.0, -1.0],
            [1.0, -1.0],
            // just reuse the pipeline for these points as well
            [0.0, 1.0],
            [0.0, -1.0],
        ]
        .map(|[sign_x, sign_y]| Vec2::X * sign_x + Vec2::Y * sign_y)
        .map(|reference_point| {
            let scaling = Vec2::X * self.primitive.radius + Vec2::Y * self.primitive.half_length;
            reference_point * scaling
        })
        .map(|vec2| self.isometry * vec2);

        // draw left and right side of capsule "rectangle"
        gizmos.line_2d(bottom_left, top_left, self.color);
        gizmos.line_2d(bottom_right, top_right, self.color);

        let start_angle_top = self.isometry.rotation.as_radians() - FRAC_PI_2;
        let start_angle_bottom = self.isometry.rotation.as_radians() + FRAC_PI_2;

        // draw arcs
        gizmos.arc_2d(
            Isometry2d::new(top_center, Rot2::radians(start_angle_top)),
            PI,
            self.primitive.radius,
            self.color,
        );
        gizmos.arc_2d(
            Isometry2d::new(bottom_center, Rot2::radians(start_angle_bottom)),
            PI,
            self.primitive.radius,
            self.color,
        );
    }
}

impl ToGizmoBlueprint2d for Capsule2d {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Capsule2d>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// line 2d
//
/// Builder for configuring the drawing options of [`Line2d`].
pub struct Line2dBuilder {
    direction: Dir2, // Direction of the line

    isometry: Isometry2d,
    color: Color, // color of the line

    draw_arrow: bool, // decides whether to indicate the direction of the line with an arrow
}

impl Line2dBuilder {
    /// Set the drawing mode of the line (arrow vs. plain line)
    pub fn draw_arrow(&mut self, is_enabled: bool) -> &mut Self {
        self.draw_arrow = is_enabled;
        self
    }
}

impl<Config, Clear> GizmoPrimitive2d<Line2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, Line2dBuilder, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Line2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for Line2dBuilder {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [start, end] = [1.0, -1.0]
            .map(|sign| sign * INFINITE_LEN)
            // offset the line from the origin infinitely into the given direction
            .map(|length| self.direction * length)
            // transform the line with the given isometry
            .map(|offset| self.isometry * offset);

        gizmos.line_2d(start, end, self.color);

        // optionally draw an arrow head at the center of the line
        if self.draw_arrow {
            gizmos.arrow_2d(
                self.isometry * (-self.direction * MIN_LINE_LEN),
                self.isometry * Vec2::ZERO,
                self.color,
            );
        }
    }
}

impl ToGizmoBlueprint2d for Line2d {
    type Blueprint2d<'primitive> = Line2dBuilder;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        Line2dBuilder {
            direction: self.direction,
            isometry: isometry.into(),
            color: color.into(),
            draw_arrow: false,
        }
    }
}

// plane 2d

impl<Config, Clear> GizmoPrimitive2d<Plane2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Plane2d>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Plane2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Plane2d> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        // draw normal of the plane (orthogonal to the plane itself)
        let normal = self.primitive.normal;
        let normal_segment = Segment2d::from_direction_and_length(normal, HALF_MIN_LINE_LEN * 2.);
        gizmos
            .primitive_2d(
                &normal_segment,
                // offset the normal so it starts on the plane line
                Isometry2d::new(
                    self.isometry * (HALF_MIN_LINE_LEN * normal),
                    self.isometry.rotation,
                ),
                self.color,
            )
            .draw_arrow(true);

        // draw the plane line
        let direction = Dir2::new_unchecked(-normal.perp());
        gizmos
            .primitive_2d(&Line2d { direction }, self.isometry, self.color)
            .draw_arrow(false);

        // draw an arrow such that the normal is always left side of the plane with respect to the
        // planes direction. This is to follow the "counter-clockwise" convention
        gizmos.arrow_2d(
            self.isometry * Vec2::ZERO,
            self.isometry * (MIN_LINE_LEN * direction),
            self.color,
        );
    }
}

impl ToGizmoBlueprint2d for Plane2d {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Plane2d>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// segment 2d

/// Builder for configuring the drawing options of [`Segment2d`].
pub struct Segment2dBuilder {
    point1: Vec2, // First point of the segment
    point2: Vec2, // Second point of the segment

    isometry: Isometry2d, // isometric transformation of the line segment
    color: Color,         // color of the line segment

    draw_arrow: bool, // decides whether to draw just a line or an arrow
}

impl Segment2dBuilder {
    /// Set the drawing mode of the line (arrow vs. plain line)
    pub fn draw_arrow(&mut self, is_enabled: bool) -> &mut Self {
        self.draw_arrow = is_enabled;
        self
    }
}

impl GizmoBlueprint2d for Segment2dBuilder {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let segment = Segment2d::new(self.point1, self.point2).transformed(self.isometry);

        if self.draw_arrow {
            gizmos.arrow_2d(segment.point1(), segment.point2(), self.color);
        } else {
            gizmos.line_2d(segment.point1(), segment.point2(), self.color);
        }
    }
}

impl<Config, Clear> GizmoPrimitive2d<Segment2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, Segment2dBuilder, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Segment2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl ToGizmoBlueprint2d for Segment2d {
    type Blueprint2d<'primitive> = Segment2dBuilder;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        Segment2dBuilder {
            point1: self.point1(),
            point2: self.point2(),

            isometry: isometry.into(),
            color: color.into(),

            draw_arrow: Default::default(),
        }
    }
}

// polyline 2d
impl<Config, Clear> GizmoPrimitive2d<Polyline2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<&'primitive Polyline2d>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Polyline2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl<'primitive> GizmoBlueprint2d for NoConfigBuilder2d<&'primitive Polyline2d> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        gizmos.linestrip_2d(
            self.primitive
                .vertices
                .iter()
                .copied()
                .map(|vec2| self.isometry * vec2),
            self.color,
        );
    }
}

impl ToGizmoBlueprint2d for Polyline2d {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<&'primitive Polyline2d>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(self, isometry, color)
    }
}

// triangle 2d

impl<Config, Clear> GizmoPrimitive2d<Triangle2d> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Triangle2d>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Triangle2d,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Triangle2d> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [a, b, c] = self.primitive.vertices;
        let positions = [a, b, c, a].map(|vec2| self.isometry * vec2);
        gizmos.linestrip_2d(positions, self.color);
    }
}

impl ToGizmoBlueprint2d for Triangle2d {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Triangle2d>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// rectangle 2d

impl<Config, Clear> GizmoPrimitive2d<Rectangle> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<Rectangle>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Rectangle,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<Rectangle> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [a, b, c, d] =
            [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    self.primitive.half_size.x * sign_x,
                    self.primitive.half_size.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(|vec2| self.isometry * vec2);
        gizmos.linestrip_2d(positions, self.color);
    }
}

impl ToGizmoBlueprint2d for Rectangle {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<Rectangle>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// polygon 2d

impl<Config, Clear> GizmoPrimitive2d<Polygon> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<&'primitive Polygon>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Polygon,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl<'primitive> GizmoBlueprint2d for NoConfigBuilder2d<&'primitive Polygon> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        // Check if the polygon needs a closing point
        let closing_point = {
            let first = self.primitive.vertices.first();
            (self.primitive.vertices.last() != first)
                .then_some(first)
                .flatten()
                .cloned()
        };

        gizmos.linestrip_2d(
            self.primitive
                .vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(|vec2| self.isometry * vec2),
            self.color,
        );
    }
}

impl ToGizmoBlueprint2d for Polygon {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<&'primitive Polygon>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(self, isometry, color)
    }
}

// regular polygon 2d

impl<Config, Clear> GizmoPrimitive2d<RegularPolygon> for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<'builder, NoConfigBuilder2d<RegularPolygon>, Config, Clear>
    where
        Self: 'builder;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive RegularPolygon,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl GizmoBlueprint2d for NoConfigBuilder2d<RegularPolygon> {
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let points = (0..=self.primitive.sides)
            .map(|n| {
                single_circle_coordinate(
                    self.primitive.circumcircle.radius,
                    self.primitive.sides,
                    n,
                )
            })
            .map(|vec2| self.isometry * vec2);
        gizmos.linestrip_2d(points, self.color);
    }
}

impl ToGizmoBlueprint2d for RegularPolygon {
    type Blueprint2d<'primitive> = NoConfigBuilder2d<RegularPolygon>;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        NoConfigBuilder2d::new(*self, isometry, color)
    }
}

// ring

/// Build a gizmo from a [`Ring<Primitive2d>`](Ring)
///
/// `RingBuilder` is an example of a "composite" builder
pub struct RingBuilder<B>
where
    B: GizmoBlueprint2d,
{
    /// The builder for the outer shape
    pub outer_builder: B,
    /// The builder for the inner shape
    pub inner_builder: B,
}

impl<P, Config, Clear> GizmoPrimitive2d<Ring<P>> for GizmoBuffer<Config, Clear>
where
    P: Primitive2d + ToGizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
    GizmoBuffer<Config, Clear>: GizmoPrimitive2d<P>,
{
    type Output<'builder, 'primitive>
        = GizmoBuilder2d<
        'builder,
        RingBuilder<<P as ToGizmoBlueprint2d>::Blueprint2d<'primitive>>,
        Config,
        Clear,
    >
    where
        P: 'primitive;

    fn primitive_2d<'primitive>(
        &mut self,
        primitive: &'primitive Ring<P>,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Output<'_, 'primitive> {
        GizmoBuilder2d::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

impl<B> GizmoBlueprint2d for RingBuilder<B>
where
    B: GizmoBlueprint2d,
{
    fn build_2d<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        self.outer_builder.build_2d(gizmos);
        self.inner_builder.build_2d(gizmos);
    }
}

impl<P> ToGizmoBlueprint2d for Ring<P>
where
    P: ToGizmoBlueprint2d + Primitive2d,
{
    type Blueprint2d<'primitive>
        = RingBuilder<<P as ToGizmoBlueprint2d>::Blueprint2d<'primitive>>
    where
        P: 'primitive;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        let isometry = isometry.into();
        let color = color.into();
        RingBuilder {
            outer_builder: self.outer_shape.to_blueprint_2d(isometry, color),
            inner_builder: self.inner_shape.to_blueprint_2d(isometry, color),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::DefaultGizmoConfigGroup;

    use super::*;
    // can build all types as rings
    #[test]
    fn can_build_types() {
        let mut gizmos: GizmoBuffer<DefaultGizmoConfigGroup, ()> = GizmoBuffer::default();
        let isometry = Isometry2d::default();
        let color = Color::default();

        let annulus = Annulus::default();
        let arc_2d = Arc2d::default();
        let capsule_2d = Capsule2d::default();
        let circle = Circle::default();
        let circular_sector = CircularSector::default();
        let circular_segment = CircularSegment::default();
        let ellipse = Ellipse::default();
        let line_2d = Line2d { direction: Dir2::X };
        let plane_2d = Plane2d::default();
        let polygon = Polygon::new([Vec2::X, Vec2::Y, Vec2::NEG_X]);
        let polyline_2d = Polyline2d::default();
        let rectangle = Rectangle::default();
        let regular_polygon = RegularPolygon::default();
        let rhombus = Rhombus::default();
        let segment_2d = Segment2d::default();
        let triangle_2d = Triangle2d::default();

        gizmos.primitive_2d(&annulus, isometry, color);
        gizmos.primitive_2d(&arc_2d, isometry, color);
        gizmos.primitive_2d(&capsule_2d, isometry, color);
        gizmos.primitive_2d(&circle, isometry, color);
        gizmos.primitive_2d(&circular_sector, isometry, color);
        gizmos.primitive_2d(&circular_segment, isometry, color);
        gizmos.primitive_2d(&ellipse, isometry, color);
        gizmos.primitive_2d(&line_2d, isometry, color);
        gizmos.primitive_2d(&plane_2d, isometry, color);
        gizmos.primitive_2d(&polygon, isometry, color);
        gizmos.primitive_2d(&polyline_2d, isometry, color);
        gizmos.primitive_2d(&rectangle, isometry, color);
        gizmos.primitive_2d(&regular_polygon, isometry, color);
        gizmos.primitive_2d(&rhombus, isometry, color);
        gizmos.primitive_2d(&segment_2d, isometry, color);
        gizmos.primitive_2d(&triangle_2d, isometry, color);

        let mut builder_annulus = annulus.to_blueprint_2d(isometry, color);
        let mut builder_arc_2d = arc_2d.to_blueprint_2d(isometry, color);
        let mut builder_capsule_2d = capsule_2d.to_blueprint_2d(isometry, color);
        let mut builder_circle = circle.to_blueprint_2d(isometry, color);
        let mut builder_circular_sector = circular_sector.to_blueprint_2d(isometry, color);
        let mut builder_circular_segment = circular_segment.to_blueprint_2d(isometry, color);
        let mut builder_ellipse = ellipse.to_blueprint_2d(isometry, color);
        let mut builder_line_2d = line_2d.to_blueprint_2d(isometry, color);
        let mut builder_plane_2d = plane_2d.to_blueprint_2d(isometry, color);
        let mut builder_polygon = polygon.to_blueprint_2d(isometry, color);
        let mut builder_polyline_2d = polyline_2d.to_blueprint_2d(isometry, color);
        let mut builder_rectangle = rectangle.to_blueprint_2d(isometry, color);
        let mut builder_regular_polygon = regular_polygon.to_blueprint_2d(isometry, color);
        let mut builder_rhombus = rhombus.to_blueprint_2d(isometry, color);
        let mut builder_segment_2d = segment_2d.to_blueprint_2d(isometry, color);
        let mut builder_triangle_2d = triangle_2d.to_blueprint_2d(isometry, color);

        builder_annulus.build_2d(&mut gizmos);
        builder_arc_2d.build_2d(&mut gizmos);
        builder_capsule_2d.build_2d(&mut gizmos);
        builder_circle.build_2d(&mut gizmos);
        builder_circular_sector.build_2d(&mut gizmos);
        builder_circular_segment.build_2d(&mut gizmos);
        builder_ellipse.build_2d(&mut gizmos);
        builder_line_2d.build_2d(&mut gizmos);
        builder_plane_2d.build_2d(&mut gizmos);
        builder_polygon.build_2d(&mut gizmos);
        builder_polyline_2d.build_2d(&mut gizmos);
        builder_rectangle.build_2d(&mut gizmos);
        builder_regular_polygon.build_2d(&mut gizmos);
        builder_rhombus.build_2d(&mut gizmos);
        builder_segment_2d.build_2d(&mut gizmos);
        builder_triangle_2d.build_2d(&mut gizmos);

        let ring_annulus = Ring::new(annulus, annulus);
        let ring_arc_2d = Ring::new(arc_2d, arc_2d);
        let ring_capsule_2d = Ring::new(capsule_2d, capsule_2d);
        let ring_circle = Ring::new(circle, circle);
        let ring_circular_sector = Ring::new(circular_sector, circular_sector);
        let ring_circular_segment = Ring::new(circular_segment, circular_segment);
        let ring_ellipse = Ring::new(ellipse, ellipse);
        let ring_line_2d = Ring::new(line_2d, line_2d);
        let ring_plane_2d = Ring::new(plane_2d, plane_2d);
        let ring_polygon = Ring::new(polygon.clone(), polygon);
        let ring_polyline_2d = Ring::new(polyline_2d.clone(), polyline_2d);
        let ring_rectangle = Ring::new(rectangle, rectangle);
        let ring_regular_polygon = Ring::new(regular_polygon, regular_polygon);
        let ring_rhombus = Ring::new(rhombus, rhombus);
        let ring_segment_2d = Ring::new(segment_2d, segment_2d);
        let ring_triangle_2d = Ring::new(triangle_2d, triangle_2d);

        gizmos.primitive_2d(&ring_annulus, isometry, color);
        gizmos.primitive_2d(&ring_arc_2d, isometry, color);
        gizmos.primitive_2d(&ring_capsule_2d, isometry, color);
        gizmos.primitive_2d(&ring_circle, isometry, color);
        gizmos.primitive_2d(&ring_circular_sector, isometry, color);
        gizmos.primitive_2d(&ring_circular_segment, isometry, color);
        gizmos.primitive_2d(&ring_ellipse, isometry, color);
        gizmos.primitive_2d(&ring_line_2d, isometry, color);
        gizmos.primitive_2d(&ring_plane_2d, isometry, color);
        gizmos.primitive_2d(&ring_polygon, isometry, color);
        gizmos.primitive_2d(&ring_polyline_2d, isometry, color);
        gizmos.primitive_2d(&ring_rectangle, isometry, color);
        gizmos.primitive_2d(&ring_regular_polygon, isometry, color);
        gizmos.primitive_2d(&ring_rhombus, isometry, color);
        gizmos.primitive_2d(&ring_segment_2d, isometry, color);
        gizmos.primitive_2d(&ring_triangle_2d, isometry, color);

        let mut builder_ring_annulus = ring_annulus.to_blueprint_2d(isometry, color);
        let mut builder_ring_arc_2d = ring_arc_2d.to_blueprint_2d(isometry, color);
        let mut builder_ring_capsule_2d = ring_capsule_2d.to_blueprint_2d(isometry, color);
        let mut builder_ring_circle = ring_circle.to_blueprint_2d(isometry, color);
        let mut builder_ring_circular_sector =
            ring_circular_sector.to_blueprint_2d(isometry, color);
        let mut builder_ring_circular_segment =
            ring_circular_segment.to_blueprint_2d(isometry, color);
        let mut builder_ring_ellipse = ring_ellipse.to_blueprint_2d(isometry, color);
        let mut builder_ring_line_2d = ring_line_2d.to_blueprint_2d(isometry, color);
        let mut builder_ring_plane_2d = ring_plane_2d.to_blueprint_2d(isometry, color);
        let mut builder_ring_polygon = ring_polygon.to_blueprint_2d(isometry, color);
        let mut builder_ring_polyline_2d = ring_polyline_2d.to_blueprint_2d(isometry, color);
        let mut builder_ring_rectangle = ring_rectangle.to_blueprint_2d(isometry, color);
        let mut builder_ring_regular_polygon =
            ring_regular_polygon.to_blueprint_2d(isometry, color);
        let mut builder_ring_rhombus = ring_rhombus.to_blueprint_2d(isometry, color);
        let mut builder_ring_segment_2d = ring_segment_2d.to_blueprint_2d(isometry, color);
        let mut builder_ring_triangle_2d = ring_triangle_2d.to_blueprint_2d(isometry, color);

        builder_ring_annulus.build_2d(&mut gizmos);
        builder_ring_arc_2d.build_2d(&mut gizmos);
        builder_ring_capsule_2d.build_2d(&mut gizmos);
        builder_ring_circle.build_2d(&mut gizmos);
        builder_ring_circular_sector.build_2d(&mut gizmos);
        builder_ring_circular_segment.build_2d(&mut gizmos);
        builder_ring_ellipse.build_2d(&mut gizmos);
        builder_ring_line_2d.build_2d(&mut gizmos);
        builder_ring_plane_2d.build_2d(&mut gizmos);
        builder_ring_polygon.build_2d(&mut gizmos);
        builder_ring_polyline_2d.build_2d(&mut gizmos);
        builder_ring_rectangle.build_2d(&mut gizmos);
        builder_ring_regular_polygon.build_2d(&mut gizmos);
        builder_ring_rhombus.build_2d(&mut gizmos);
        builder_ring_segment_2d.build_2d(&mut gizmos);
        builder_ring_triangle_2d.build_2d(&mut gizmos);
    }
}
