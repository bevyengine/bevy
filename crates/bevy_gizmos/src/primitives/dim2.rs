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
        Annulus, Arc2d, Capsule2d, CircularSector, CircularSegment, Line2d, Plane2d, Polygon,
        Polyline2d, Primitive2d, Rectangle, RegularPolygon, Rhombus, Ring, Segment2d, Triangle2d,
    },
    Dir2, Isometry2d, Rot2, Vec2,
};

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup, GizmoAsset};

// some magic number since using directions as offsets will result in lines of length 1 pixel
const MIN_LINE_LEN: f32 = 50.0;
const HALF_MIN_LINE_LEN: f32 = 25.0;
// length used to simulate infinite lines
const INFINITE_LEN: f32 = 100_000.0;

/// A trait for rendering 2D geometric primitives (`P`) with [`GizmoBuffer`].
///
/// This trait is automatically implemented for [`GizmoBuffer`] for all `P` where `P` implements [`Primitive2d`] and [`ToGizmoBlueprint2d`].
///
/// You typically interact with this trait via the [`Gizmos`] system parameter when using immediate-mode gizmos
/// or the [`GizmoAsset`] asset when using retained-mode gizmos.
pub trait GizmoPrimitive2d<P, Config, Clear>
where
    P: Primitive2d + ToGizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// The blueprint type constructed from the primitive.
    type Output<'builder, 'primitive>: GizmoBlueprint
    where
        Self: 'builder,
        P: 'primitive;

    /// Renders a 2D primitive with its associated details.
    ///
    /// Note that gizmos are queued when the [`Gizmo2dBuilder`] that wraps the [`GizmoBlueprint`] is dropped, e.g. when it goes out of scope.
    /// If you want to queue them immediately you can call [`.immediate()`](GizmoBuilder::immediate) which will consume the builder.
    fn primitive_2d<'builder, 'primitive>(
        &'builder mut self,
        primitive: &'primitive P,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> GizmoAssembler<'builder, Self::Output<'builder, 'primitive>, Config, Clear>;
}

impl<P, Config, Clear> GizmoPrimitive2d<P, Config, Clear> for GizmoBuffer<Config, Clear>
where
    P: Primitive2d + ToGizmoBlueprint2d,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Output<'builder, 'primitive>
        = <P as ToGizmoBlueprint2d>::Blueprint2d<'primitive>
    where
        Self: 'builder,
        P: 'primitive;

    fn primitive_2d<'builder, 'primitive>(
        &mut self,
        primitive: &'primitive P,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> GizmoAssembler<'_, Self::Output<'_, 'primitive>, Config, Clear> {
        GizmoAssembler::new(self, primitive.to_blueprint_2d(isometry, color))
    }
}

/// A type that provides the data to construct a gizmo.
///
/// This is the trait that does the heavy lifting when drawing gizmos, via its [`build`](GizmoBlueprint::build) method.
///
/// This type should store and must provide the Isometry ([`Isometry2d`]) and [`Color`].
///
/// See the documentation for [`GizmoPrimitive2d`] for 2d primitives. See also [`ToGizmoBlueprint2d`] which is recommended that you implement.
///
/// When you require a builder for configuration, then:
/// 1. Create a primitive
/// 2. Implement [`ToGizmoBlueprint2d`] for your primitive, to produce a builder
/// 3. Implement [`GizmoBlueprint`] for your builder
///
/// ```rs
/// #[derive(Clone, Copy)]
/// struct Smiley {
///     radius: f32,
/// }
///
/// impl Primitive2d for Smiley {}
///
/// impl ToGizmoBlueprint2d for Smiley {
///     type Blueprint2d<'primitive>
///         = SmileyBuilder
///     where
///         Self: 'primitive;
///
///     /// Construct the blueprint type
///     fn to_blueprint_2d(
///         &self,
///         isometry: impl Into<Isometry2d>,
///         color: impl Into<Color>,
///     ) -> Self::Blueprint2d<'_> {
///         SmileyBuilder::new(self.radius, isometry.into(), color.into())
///     }
/// }
///
/// struct SmileyBuilder {
///     radius: f32,
///     isometry: Isometry2d,
///     color: Color,
///     resolution: u32,
/// }
///
/// impl SmileyBuilder {
///     fn new(radius: f32, isometry: Isometry2d, color: Color) -> Self {
///         Self {
///             radius,
///             isometry,
///             color,
///             resolution: 50,
///         }
///     }
///
///     fn resolution(&mut self, resolution: u32) -> &mut Self {
///         if resolution >= 3 {
///             self.resolution = resolution;
///         }
///         self
///     }
/// }
///
/// impl GizmoBlueprint for SmileyBuilder {
///     fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
///     where
///         Config: GizmoConfigGroup,
///         Clear: 'static + Send + Sync,
///     {
///         // head
///         gizmos
///             .circle_2d(self.isometry, self.radius, self.color)
///             .resolution(self.resolution);
///         // eyes
///         gizmos
///             .circle_2d(
///                 self.isometry * Vec2::new(-self.radius / 3.0, self.radius / 3.0),
///                 self.radius / 4.0,
///                 self.color,
///             )
///             .resolution(self.resolution / 2);
///         gizmos
///             .circle_2d(
///                 self.isometry * Vec2::new(self.radius / 3.0, self.radius / 3.0),
///                 self.radius / 4.0,
///                 self.color,
///             )
///             .resolution(self.resolution / 2);
///         // mouth
///         gizmos
///             .arc_2d(
///                 self.isometry
///                     * Isometry2d::new(
///                         Vec2::new(0.0, 0.0),
///                         Rot2::radians(FRAC_PI_2 + FRAC_PI_8),
///                     ),
///                 FRAC_PI_2 + FRAC_PI_4,
///                 self.radius * 3.0 / 4.0,
///                 self.color,
///             )
///             .resolution(self.resolution / 2);
///     }
/// }
///
/// let mut gizmos: GizmoBuffer<DefaultGizmoConfigGroup, ()> = GizmoBuffer::default();
///
/// let smiley = Smiley { radius: 50.0 };
///
/// gizmos
///     .primitive_2d(&smiley, Vec2::ZERO, GREEN)
///     .resolution(20);
///
// let _asset: GizmoAsset = smiley.to_blueprint_2d(Vec2::ZERO, ORANGE_RED).into();
///
/// ```
///
/// If your Blueprint is simple and doesn't require a configuration builder,
/// you can implement [`SimpleGizmoBlueprint2d`]
/// instead of [`ToGizmoBlueprint2d`] and `GizmoBlueprint`.
pub trait GizmoBlueprint {
    /// Queue the gizmo drawing instructions into the [`GizmoBuffer`].
    ///
    /// When using [`GizmoPrimitive2d`], this is run on drop of [`GizmoBuilder2d`].
    fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync;
}

impl<B: GizmoBlueprint> From<B> for GizmoAsset {
    fn from(mut value: B) -> Self {
        let mut asset = GizmoAsset::new();
        value.build(&mut asset);
        asset
    }
}

// -------

/// The dyn-compatible version of [`GizmoBlueprint`]
pub trait DynGizmoBlueprint<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Queue the gizmo drawing instructions into the [`GizmoBuffer`].
    fn build_dyn(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>);
}

impl<T: GizmoBlueprint, Config, Clear> DynGizmoBlueprint<Config, Clear> for T
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn build_dyn(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>) {
        self.build(gizmos);
    }
}

/// A type that generates a blueprint `Blueprint2d`.
///
/// This should be implemented in conjunction with [`GizmoBlueprint`].
/// See `GizmoBlueprint` for further information.
pub trait ToGizmoBlueprint2d {
    /// The blueprint type.
    ///
    /// Supports borrowing the data from the primitive if required.
    type Blueprint2d<'primitive>: GizmoBlueprint
    where
        Self: 'primitive;

    /// Construct the Blueprint type.
    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_>;
}

/// A helper trait for implementing the Blueprint traits.
///
/// By implementing the `SimpleGizmoBlueprint2d` trait,
/// the type automatically implements [`ToGizmoBlueprint2d`]
/// to produce a `SimpleGizmoBuilder2d<House>`,
/// which itself implements[ `GizmoBlueprint`].
///
/// If your Blueprint needs a configuration builder,
/// you should implement `ToGizmoBlueprint2d` and `GizmoBlueprint`
/// instead of `SimpleGizmoBlueprint2d`.
///
/// ```
/// #[derive(Clone, Copy)]
/// struct House {
///     width: f32,
///     plate_height: f32,
///     roof_height: f32,
/// }
///
/// impl Primitive2d for House {}
///
/// impl SimpleGizmoBlueprint2d for House {
///     fn build_2d<Config, Clear>(
///         &self,
///         gizmos: &mut GizmoBuffer<Config, Clear>,
///         isometry: Isometry2d,
///         color: Color,
///     ) where
///         Config: GizmoConfigGroup,
///         Clear: 'static + Send + Sync,
///     {
///         // the walls
///         gizmos.rect_2d(isometry, Vec2::new(self.width, self.plate_height), color);
///         // the roof
///         gizmos.primitive_2d(
///             &Triangle2d::new(
///                 Vec2::new(-self.width / 2.0, self.plate_height / 2.0),
///                 Vec2::new(0.0, self.plate_height / 2.0 + self.roof_height),
///                 Vec2::new(self.width / 2.0, self.plate_height / 2.0),
///             ),
///             isometry,
///             color,
///         );
///     }
/// }
/// ```
pub trait SimpleGizmoBlueprint2d {
    ///
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync;
}

impl<P> ToGizmoBlueprint2d for P
where
    P: SimpleGizmoBlueprint2d,
{
    type Blueprint2d<'primitive>
        = SimpleGizmoBuilder2d<&'primitive P>
    where
        Self: 'primitive;

    fn to_blueprint_2d(
        &self,
        isometry: impl Into<Isometry2d>,
        color: impl Into<Color>,
    ) -> Self::Blueprint2d<'_> {
        SimpleGizmoBuilder2d::new(self, isometry, color)
    }
}

impl<'primitive, P> GizmoBlueprint for SimpleGizmoBuilder2d<&'primitive P>
where
    P: SimpleGizmoBlueprint2d,
{
    fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        self.primitive.build_2d(gizmos, self.isometry, self.color);
    }
}

/// This is essentially a scope guard that runs the inner blueprint's [`GizmoBlueprint::build`] method on drop.
///
/// Provides access to the "blueprint" via the `Deref`/`DerefMut` traits for configuration purposes.
///
/// See the documentation for [`GizmoPrimitive2d`].
pub struct GizmoAssembler<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'builder mut GizmoBuffer<Config, Clear>,
    blueprint: Blueprint,
}

impl<'builder, Blueprint, Config, Clear> GizmoAssembler<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Construct a new `GizmoBuilder` from a `GizmoBuffer` and `Blueprint` that implements `Primitive2dGizmoBuilder`
    pub fn new(gizmos: &'builder mut GizmoBuffer<Config, Clear>, blueprint: Blueprint) -> Self {
        Self { gizmos, blueprint }
    }

    /// Consume the builder, which will immediately drop it, running the [`GizmoBlueprint::build`] method.
    pub fn immediate(self) {}
}

impl<'builder, Blueprint, Config, Clear> core::ops::Deref
    for GizmoAssembler<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Target = Blueprint;

    fn deref(&self) -> &Self::Target {
        &self.blueprint
    }
}

impl<'builder, Blueprint, Config, Clear> core::ops::DerefMut
    for GizmoAssembler<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.blueprint
    }
}

impl<'builder, Blueprint, Config, Clear> Drop for GizmoAssembler<'builder, Blueprint, Config, Clear>
where
    Blueprint: GizmoBlueprint,
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        let GizmoAssembler { gizmos, blueprint } = self;
        if !gizmos.enabled {
            return;
        }
        blueprint.build(gizmos);
    }
}

/// A simple "builder" without configuration options that simply wraps a primitive.
/// This is useful to avoid boilerplate for [`GizmoBlueprint`], [`ToGizmoBlueprint2d`] and [`GizmoPrimitive2d`] trait implementations.
pub struct SimpleGizmoBuilder2d<P> {
    /// The primitive provided by the caller in [`ToGizmoBlueprint2d::to_blueprint_2d`] or [`GizmoPrimitive2d::primitive_2d`]
    pub primitive: P,
    /// The isometry provided by the caller in [`ToGizmoBlueprint2d::to_blueprint_2d`] or [`GizmoPrimitive2d::primitive_2d`]
    pub isometry: Isometry2d,
    /// The color provided by the caller in [`ToGizmoBlueprint2d::to_blueprint_2d`] or [`GizmoPrimitive2d::primitive_2d`]
    pub color: Color,
}

impl<P> SimpleGizmoBuilder2d<P> {
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

impl SimpleGizmoBlueprint2d for Dir2 {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start = Vec2::ZERO;
        let end = *self * MIN_LINE_LEN;
        gizmos.arrow_2d(isometry * start, isometry * end, color);
    }
}

// arc 2d

impl SimpleGizmoBlueprint2d for Arc2d {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start_iso = isometry * Isometry2d::from_rotation(Rot2::radians(-self.half_angle));

        gizmos.arc_2d(start_iso, self.half_angle * 2.0, self.radius, color);
    }
}

// circle 2d

// implementation of Blueprint traits for Circle are in module circles

// circular sector 2d

impl SimpleGizmoBlueprint2d for CircularSector {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start_iso = isometry * Isometry2d::from_rotation(Rot2::radians(-self.arc.half_angle));
        let end_iso = isometry * Isometry2d::from_rotation(Rot2::radians(self.arc.half_angle));

        // we need to draw the arc part of the sector, and the two lines connecting the arc and the center
        gizmos.arc_2d(start_iso, self.arc.half_angle * 2.0, self.arc.radius, color);

        let end_position = self.arc.radius * Vec2::Y;
        gizmos.line_2d(isometry * Vec2::ZERO, start_iso * end_position, color);
        gizmos.line_2d(isometry * Vec2::ZERO, end_iso * end_position, color);
    }
}

// circular segment 2d

impl SimpleGizmoBlueprint2d for CircularSegment {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let start_iso = isometry * Isometry2d::from_rotation(Rot2::radians(-self.arc.half_angle));
        let end_iso = isometry * Isometry2d::from_rotation(Rot2::radians(self.arc.half_angle));

        // we need to draw the arc part of the segment, and the line connecting the two ends
        gizmos.arc_2d(start_iso, self.arc.half_angle * 2.0, self.arc.radius, color);

        let position = self.arc.radius * Vec2::Y;
        gizmos.line_2d(start_iso * position, end_iso * position, color);
    }
}

// ellipse 2d

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

impl GizmoBlueprint for Annulus2dBuilder {
    fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
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

impl SimpleGizmoBlueprint2d for Rhombus {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [a, b, c, d] =
            [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(
                    self.half_diagonals.x * sign_x,
                    self.half_diagonals.y * sign_y,
                )
            });
        let positions = [a, b, c, d, a].map(|vec2| isometry * vec2);
        gizmos.linestrip_2d(positions, color);
    }
}

// capsule 2d

impl SimpleGizmoBlueprint2d for Capsule2d {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
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
            let scaling = Vec2::X * self.radius + Vec2::Y * self.half_length;
            reference_point * scaling
        })
        .map(|vec2| isometry * vec2);

        // draw left and right side of capsule "rectangle"
        gizmos.line_2d(bottom_left, top_left, color);
        gizmos.line_2d(bottom_right, top_right, color);

        let start_angle_top = isometry.rotation.as_radians() - FRAC_PI_2;
        let start_angle_bottom = isometry.rotation.as_radians() + FRAC_PI_2;

        // draw arcs
        gizmos.arc_2d(
            Isometry2d::new(top_center, Rot2::radians(start_angle_top)),
            PI,
            self.radius,
            color,
        );
        gizmos.arc_2d(
            Isometry2d::new(bottom_center, Rot2::radians(start_angle_bottom)),
            PI,
            self.radius,
            color,
        );
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

impl GizmoBlueprint for Line2dBuilder {
    fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
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

impl SimpleGizmoBlueprint2d for Plane2d {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        // draw normal of the plane (orthogonal to the plane itself)
        let normal = self.normal;
        let normal_segment = Segment2d::from_direction_and_length(normal, HALF_MIN_LINE_LEN * 2.);
        gizmos
            .primitive_2d(
                &normal_segment,
                // offset the normal so it starts on the plane line
                Isometry2d::new(isometry * (HALF_MIN_LINE_LEN * normal), isometry.rotation),
                color,
            )
            .draw_arrow(true);

        // draw the plane line
        let direction = Dir2::new_unchecked(-normal.perp());
        gizmos
            .primitive_2d(&Line2d { direction }, isometry, color)
            .draw_arrow(false);

        // draw an arrow such that the normal is always left side of the plane with respect to the
        // planes direction. This is to follow the "counter-clockwise" convention
        gizmos.arrow_2d(
            isometry * Vec2::ZERO,
            isometry * (MIN_LINE_LEN * direction),
            color,
        );
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

impl GizmoBlueprint for Segment2dBuilder {
    fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
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

impl<'primitive> SimpleGizmoBlueprint2d for Polyline2d {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        gizmos.linestrip_2d(
            self.vertices.iter().copied().map(|vec2| isometry * vec2),
            color,
        );
    }
}

// triangle 2d

impl SimpleGizmoBlueprint2d for Triangle2d {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [a, b, c] = self.vertices;
        let positions = [a, b, c, a].map(|vec2| isometry * vec2);
        gizmos.linestrip_2d(positions, color);
    }
}

// rectangle 2d

impl SimpleGizmoBlueprint2d for Rectangle {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let [a, b, c, d] =
            [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)].map(|(sign_x, sign_y)| {
                Vec2::new(self.half_size.x * sign_x, self.half_size.y * sign_y)
            });
        let positions = [a, b, c, d, a].map(|vec2| isometry * vec2);
        gizmos.linestrip_2d(positions, color);
    }
}

// polygon 2d

impl SimpleGizmoBlueprint2d for Polygon {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        // Check if the polygon needs a closing point
        let closing_point = {
            let first = self.vertices.first();
            (self.vertices.last() != first)
                .then_some(first)
                .flatten()
                .cloned()
        };

        gizmos.linestrip_2d(
            self.vertices
                .iter()
                .copied()
                .chain(closing_point)
                .map(|vec2| isometry * vec2),
            color,
        );
    }
}

// regular polygon 2d

impl SimpleGizmoBlueprint2d for RegularPolygon {
    fn build_2d<Config, Clear>(
        &self,
        gizmos: &mut GizmoBuffer<Config, Clear>,
        isometry: Isometry2d,
        color: Color,
    ) where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        let points = (0..=self.sides)
            .map(|n| single_circle_coordinate(self.circumcircle.radius, self.sides, n))
            .map(|vec2| isometry * vec2);
        gizmos.linestrip_2d(points, color);
    }
}

// ring

/// Build a gizmo from a [`Ring<Primitive2d>`](Ring)
///
/// `RingBuilder` is an example of a "composite" builder
pub struct RingBuilder<B>
where
    B: GizmoBlueprint,
{
    /// The builder for the outer shape
    pub outer_builder: B,
    /// The builder for the inner shape
    pub inner_builder: B,
}

impl<B> GizmoBlueprint for RingBuilder<B>
where
    B: GizmoBlueprint,
{
    fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
    where
        Config: GizmoConfigGroup,
        Clear: 'static + Send + Sync,
    {
        self.outer_builder.build(gizmos);
        self.inner_builder.build(gizmos);
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
    use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};

    use bevy_color::palettes::css::{GREEN, ORANGE_RED};
    use bevy_math::primitives::{Circle, Ellipse};

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

        builder_annulus.build(&mut gizmos);
        builder_arc_2d.build(&mut gizmos);
        builder_capsule_2d.build(&mut gizmos);
        builder_circle.build(&mut gizmos);
        builder_circular_sector.build(&mut gizmos);
        builder_circular_segment.build(&mut gizmos);
        builder_ellipse.build(&mut gizmos);
        builder_line_2d.build(&mut gizmos);
        builder_plane_2d.build(&mut gizmos);
        builder_polygon.build(&mut gizmos);
        builder_polyline_2d.build(&mut gizmos);
        builder_rectangle.build(&mut gizmos);
        builder_regular_polygon.build(&mut gizmos);
        builder_rhombus.build(&mut gizmos);
        builder_segment_2d.build(&mut gizmos);
        builder_triangle_2d.build(&mut gizmos);

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

        builder_ring_annulus.build(&mut gizmos);
        builder_ring_arc_2d.build(&mut gizmos);
        builder_ring_capsule_2d.build(&mut gizmos);
        builder_ring_circle.build(&mut gizmos);
        builder_ring_circular_sector.build(&mut gizmos);
        builder_ring_circular_segment.build(&mut gizmos);
        builder_ring_ellipse.build(&mut gizmos);
        builder_ring_line_2d.build(&mut gizmos);
        builder_ring_plane_2d.build(&mut gizmos);
        builder_ring_polygon.build(&mut gizmos);
        builder_ring_polyline_2d.build(&mut gizmos);
        builder_ring_rectangle.build(&mut gizmos);
        builder_ring_regular_polygon.build(&mut gizmos);
        builder_ring_rhombus.build(&mut gizmos);
        builder_ring_segment_2d.build(&mut gizmos);
        builder_ring_triangle_2d.build(&mut gizmos);
    }

    #[test]
    fn can_generate_assets() {
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

        let builder_annulus = annulus.to_blueprint_2d(isometry, color).into();
        let builder_arc_2d = arc_2d.to_blueprint_2d(isometry, color).into();
        let builder_capsule_2d = capsule_2d.to_blueprint_2d(isometry, color).into();
        let builder_circle = circle.to_blueprint_2d(isometry, color).into();
        let builder_circular_sector = circular_sector.to_blueprint_2d(isometry, color).into();
        let builder_circular_segment = circular_segment.to_blueprint_2d(isometry, color).into();
        let builder_ellipse = ellipse.to_blueprint_2d(isometry, color).into();
        let builder_line_2d = line_2d.to_blueprint_2d(isometry, color).into();
        let builder_plane_2d = plane_2d.to_blueprint_2d(isometry, color).into();
        let builder_polygon = polygon.to_blueprint_2d(isometry, color).into();
        let builder_polyline_2d = polyline_2d.to_blueprint_2d(isometry, color).into();
        let builder_rectangle = rectangle.to_blueprint_2d(isometry, color).into();
        let builder_regular_polygon = regular_polygon.to_blueprint_2d(isometry, color).into();
        let builder_rhombus = rhombus.to_blueprint_2d(isometry, color).into();
        let builder_segment_2d = segment_2d.to_blueprint_2d(isometry, color).into();
        let builder_triangle_2d = triangle_2d.to_blueprint_2d(isometry, color).into();

        let ring_annulus = Ring::new(annulus, annulus);
        let ring_arc_2d = Ring::new(arc_2d, arc_2d);
        let ring_capsule_2d = Ring::new(capsule_2d, capsule_2d);
        let ring_circle = Ring::new(circle, circle);
        let ring_circular_sector = Ring::new(circular_sector, circular_sector);
        let ring_circular_segment = Ring::new(circular_segment, circular_segment);
        let ring_ellipse = Ring::new(ellipse, ellipse);
        let ring_line_2d = Ring::new(line_2d, line_2d);
        let ring_plane_2d = Ring::new(plane_2d, plane_2d);
        let ring_polygon = Ring::new(polygon.clone(), polygon.clone());
        let ring_polyline_2d = Ring::new(polyline_2d.clone(), polyline_2d.clone());
        let ring_rectangle = Ring::new(rectangle, rectangle);
        let ring_regular_polygon = Ring::new(regular_polygon, regular_polygon);
        let ring_rhombus = Ring::new(rhombus, rhombus);
        let ring_segment_2d = Ring::new(segment_2d, segment_2d);
        let ring_triangle_2d = Ring::new(triangle_2d, triangle_2d);

        let builder_ring_annulus = ring_annulus.to_blueprint_2d(isometry, color).into();
        let builder_ring_arc_2d = ring_arc_2d.to_blueprint_2d(isometry, color).into();
        let builder_ring_capsule_2d = ring_capsule_2d.to_blueprint_2d(isometry, color).into();
        let builder_ring_circle = ring_circle.to_blueprint_2d(isometry, color).into();
        let builder_ring_circular_sector =
            ring_circular_sector.to_blueprint_2d(isometry, color).into();
        let builder_ring_circular_segment = ring_circular_segment
            .to_blueprint_2d(isometry, color)
            .into();
        let builder_ring_ellipse = ring_ellipse.to_blueprint_2d(isometry, color).into();
        let builder_ring_line_2d = ring_line_2d.to_blueprint_2d(isometry, color).into();
        let builder_ring_plane_2d = ring_plane_2d.to_blueprint_2d(isometry, color).into();
        let builder_ring_polygon = ring_polygon.to_blueprint_2d(isometry, color).into();
        let builder_ring_polyline_2d = ring_polyline_2d.to_blueprint_2d(isometry, color).into();
        let builder_ring_rectangle = ring_rectangle.to_blueprint_2d(isometry, color).into();
        let builder_ring_regular_polygon =
            ring_regular_polygon.to_blueprint_2d(isometry, color).into();
        let builder_ring_rhombus = ring_rhombus.to_blueprint_2d(isometry, color).into();
        let builder_ring_segment_2d = ring_segment_2d.to_blueprint_2d(isometry, color).into();
        let builder_ring_triangle_2d = ring_triangle_2d.to_blueprint_2d(isometry, color).into();

        let _assets: Vec<GizmoAsset> = vec![
            builder_annulus,
            builder_arc_2d,
            builder_capsule_2d,
            builder_circle,
            builder_circular_sector,
            builder_circular_segment,
            builder_ellipse,
            builder_line_2d,
            builder_plane_2d,
            builder_polygon,
            builder_polyline_2d,
            builder_rectangle,
            builder_regular_polygon,
            builder_rhombus,
            builder_segment_2d,
            builder_triangle_2d,
            builder_ring_annulus,
            builder_ring_arc_2d,
            builder_ring_capsule_2d,
            builder_ring_circle,
            builder_ring_circular_sector,
            builder_ring_circular_segment,
            builder_ring_ellipse,
            builder_ring_line_2d,
            builder_ring_plane_2d,
            builder_ring_polygon,
            builder_ring_polyline_2d,
            builder_ring_rectangle,
            builder_ring_regular_polygon,
            builder_ring_rhombus,
            builder_ring_segment_2d,
            builder_ring_triangle_2d,
        ];
    }

    #[test]
    fn can_be_boxed() {
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

        let builder_annulus = Box::new(annulus.to_blueprint_2d(isometry, color));
        let builder_arc_2d = Box::new(arc_2d.to_blueprint_2d(isometry, color));
        let builder_capsule_2d = Box::new(capsule_2d.to_blueprint_2d(isometry, color));
        let builder_circle = Box::new(circle.to_blueprint_2d(isometry, color));
        let builder_circular_sector = Box::new(circular_sector.to_blueprint_2d(isometry, color));
        let builder_circular_segment = Box::new(circular_segment.to_blueprint_2d(isometry, color));
        let builder_ellipse = Box::new(ellipse.to_blueprint_2d(isometry, color));
        let builder_line_2d = Box::new(line_2d.to_blueprint_2d(isometry, color));
        let builder_plane_2d = Box::new(plane_2d.to_blueprint_2d(isometry, color));
        let builder_polygon = Box::new(polygon.to_blueprint_2d(isometry, color));
        let builder_polyline_2d = Box::new(polyline_2d.to_blueprint_2d(isometry, color));
        let builder_rectangle = Box::new(rectangle.to_blueprint_2d(isometry, color));
        let builder_regular_polygon = Box::new(regular_polygon.to_blueprint_2d(isometry, color));
        let builder_rhombus = Box::new(rhombus.to_blueprint_2d(isometry, color));
        let builder_segment_2d = Box::new(segment_2d.to_blueprint_2d(isometry, color));
        let builder_triangle_2d = Box::new(triangle_2d.to_blueprint_2d(isometry, color));

        let ring_annulus = Ring::new(annulus, annulus);
        let ring_arc_2d = Ring::new(arc_2d, arc_2d);
        let ring_capsule_2d = Ring::new(capsule_2d, capsule_2d);
        let ring_circle = Ring::new(circle, circle);
        let ring_circular_sector = Ring::new(circular_sector, circular_sector);
        let ring_circular_segment = Ring::new(circular_segment, circular_segment);
        let ring_ellipse = Ring::new(ellipse, ellipse);
        let ring_line_2d = Ring::new(line_2d, line_2d);
        let ring_plane_2d = Ring::new(plane_2d, plane_2d);
        let ring_polygon = Ring::new(polygon.clone(), polygon.clone());
        let ring_polyline_2d = Ring::new(polyline_2d.clone(), polyline_2d.clone());
        let ring_rectangle = Ring::new(rectangle, rectangle);
        let ring_regular_polygon = Ring::new(regular_polygon, regular_polygon);
        let ring_rhombus = Ring::new(rhombus, rhombus);
        let ring_segment_2d = Ring::new(segment_2d, segment_2d);
        let ring_triangle_2d = Ring::new(triangle_2d, triangle_2d);

        let builder_ring_annulus = Box::new(ring_annulus.to_blueprint_2d(isometry, color));
        let builder_ring_arc_2d = Box::new(ring_arc_2d.to_blueprint_2d(isometry, color));
        let builder_ring_capsule_2d = Box::new(ring_capsule_2d.to_blueprint_2d(isometry, color));
        let builder_ring_circle = Box::new(ring_circle.to_blueprint_2d(isometry, color));
        let builder_ring_circular_sector =
            Box::new(ring_circular_sector.to_blueprint_2d(isometry, color));
        let builder_ring_circular_segment =
            Box::new(ring_circular_segment.to_blueprint_2d(isometry, color));
        let builder_ring_ellipse = Box::new(ring_ellipse.to_blueprint_2d(isometry, color));
        let builder_ring_line_2d = Box::new(ring_line_2d.to_blueprint_2d(isometry, color));
        let builder_ring_plane_2d = Box::new(ring_plane_2d.to_blueprint_2d(isometry, color));
        let builder_ring_polygon = Box::new(ring_polygon.to_blueprint_2d(isometry, color));
        let builder_ring_polyline_2d = Box::new(ring_polyline_2d.to_blueprint_2d(isometry, color));
        let builder_ring_rectangle = Box::new(ring_rectangle.to_blueprint_2d(isometry, color));
        let builder_ring_regular_polygon =
            Box::new(ring_regular_polygon.to_blueprint_2d(isometry, color));
        let builder_ring_rhombus = Box::new(ring_rhombus.to_blueprint_2d(isometry, color));
        let builder_ring_segment_2d = Box::new(ring_segment_2d.to_blueprint_2d(isometry, color));
        let builder_ring_triangle_2d = Box::new(ring_triangle_2d.to_blueprint_2d(isometry, color));

        let blueprints: Vec<Box<dyn DynGizmoBlueprint<DefaultGizmoConfigGroup, ()>>> = vec![
            builder_annulus,
            builder_arc_2d,
            builder_capsule_2d,
            builder_circle,
            builder_circular_sector,
            builder_circular_segment,
            builder_ellipse,
            builder_line_2d,
            builder_plane_2d,
            builder_polygon,
            builder_polyline_2d,
            builder_rectangle,
            builder_regular_polygon,
            builder_rhombus,
            builder_segment_2d,
            builder_triangle_2d,
            builder_ring_annulus,
            builder_ring_arc_2d,
            builder_ring_capsule_2d,
            builder_ring_circle,
            builder_ring_circular_sector,
            builder_ring_circular_segment,
            builder_ring_ellipse,
            builder_ring_line_2d,
            builder_ring_plane_2d,
            builder_ring_polygon,
            builder_ring_polyline_2d,
            builder_ring_rectangle,
            builder_ring_regular_polygon,
            builder_ring_rhombus,
            builder_ring_segment_2d,
            builder_ring_triangle_2d,
        ];

        let mut gizmos = GizmoBuffer::default();
        for mut blueprint in blueprints {
            blueprint.build_dyn(&mut gizmos);
        }
    }

    #[test]
    fn custom_primitive_simple() {
        #[derive(Clone, Copy)]
        struct House {
            width: f32,
            plate_height: f32,
            roof_height: f32,
        }

        impl Primitive2d for House {}

        /// By implementing the `SimpleGizmoBlueprint2d` trait, the type automatically implements `ToGizmoBlueprint2d`
        /// `ToGizmoBlueprint2d::to_blueprint_2d` produces a `SimpleGizmoBuilder2d<House>`, which itself implements `GizmoBlueprint`
        impl SimpleGizmoBlueprint2d for House {
            fn build_2d<Config, Clear>(
                &self,
                gizmos: &mut GizmoBuffer<Config, Clear>,
                isometry: Isometry2d,
                color: Color,
            ) where
                Config: GizmoConfigGroup,
                Clear: 'static + Send + Sync,
            {
                // the walls
                gizmos.rect_2d(isometry, Vec2::new(self.width, self.plate_height), color);
                // the roof
                gizmos.primitive_2d(
                    &Triangle2d::new(
                        Vec2::new(-self.width / 2.0, self.plate_height / 2.0),
                        Vec2::new(0.0, self.plate_height / 2.0 + self.roof_height),
                        Vec2::new(self.width / 2.0, self.plate_height / 2.0),
                    ),
                    isometry,
                    color,
                );
            }
        }

        let house = House {
            width: 100.0,
            plate_height: 100.0,
            roof_height: 30.0,
        };

        let mut gizmos: GizmoBuffer<DefaultGizmoConfigGroup, ()> = GizmoBuffer::default();

        gizmos.primitive_2d(&house, Vec2::ZERO, ORANGE_RED);

        let _asset: GizmoAsset = house.to_blueprint_2d(Vec2::ZERO, ORANGE_RED).into();
    }

    #[test]
    fn custom_primitive_builder() {
        #[derive(Clone, Copy)]
        struct Smiley {
            radius: f32,
        }

        impl Primitive2d for Smiley {}

        impl ToGizmoBlueprint2d for Smiley {
            type Blueprint2d<'primitive>
                = SmileyBuilder
            where
                Self: 'primitive;

            /// Construct the blueprint type
            fn to_blueprint_2d(
                &self,
                isometry: impl Into<Isometry2d>,
                color: impl Into<Color>,
            ) -> Self::Blueprint2d<'_> {
                SmileyBuilder::new(self.radius, isometry.into(), color.into())
            }
        }

        struct SmileyBuilder {
            radius: f32,
            isometry: Isometry2d,
            color: Color,
            resolution: u32,
        }

        impl SmileyBuilder {
            fn new(radius: f32, isometry: Isometry2d, color: Color) -> Self {
                Self {
                    radius,
                    isometry,
                    color,
                    resolution: 50,
                }
            }

            fn resolution(&mut self, resolution: u32) -> &mut Self {
                if resolution >= 3 {
                    self.resolution = resolution;
                }
                self
            }
        }

        impl GizmoBlueprint for SmileyBuilder {
            fn build<Config, Clear>(&mut self, gizmos: &mut GizmoBuffer<Config, Clear>)
            where
                Config: GizmoConfigGroup,
                Clear: 'static + Send + Sync,
            {
                // head
                gizmos
                    .circle_2d(self.isometry, self.radius, self.color)
                    .resolution(self.resolution);
                // eyes
                gizmos
                    .circle_2d(
                        self.isometry * Vec2::new(-self.radius / 3.0, self.radius / 3.0),
                        self.radius / 4.0,
                        self.color,
                    )
                    .resolution(self.resolution / 2);
                gizmos
                    .circle_2d(
                        self.isometry * Vec2::new(self.radius / 3.0, self.radius / 3.0),
                        self.radius / 4.0,
                        self.color,
                    )
                    .resolution(self.resolution / 2);
                // mouth
                gizmos
                    .arc_2d(
                        self.isometry
                            * Isometry2d::new(
                                Vec2::new(0.0, 0.0),
                                Rot2::radians(FRAC_PI_2 + FRAC_PI_8),
                            ),
                        FRAC_PI_2 + FRAC_PI_4,
                        self.radius * 3.0 / 4.0,
                        self.color,
                    )
                    .resolution(self.resolution / 2);
            }
        }

        let mut gizmos: GizmoBuffer<DefaultGizmoConfigGroup, ()> = GizmoBuffer::default();

        let smiley = Smiley { radius: 50.0 };

        gizmos
            .primitive_2d(&smiley, Vec2::ZERO, GREEN)
            .resolution(20);

        let _asset: GizmoAsset = smiley.to_blueprint_2d(Vec2::ZERO, ORANGE_RED).into();
    }
}
