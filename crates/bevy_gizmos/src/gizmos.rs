//! A module for the [`Gizmos`](crate::gizmos::Gizmos) [`SystemParam`](bevy_ecs::system::SystemParam).

use std::{f32::consts::TAU, iter};

use bevy_ecs::{
    system::{Deferred, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::World,
};
use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::prelude::Color;

type PositionItem = [f32; 3];
type ColorItem = [f32; 4];

const DEFAULT_CIRCLE_SEGMENTS: usize = 32;

#[derive(Resource, Default)]
pub(crate) struct GizmoStorage {
    pub list_positions: Vec<PositionItem>,
    pub list_colors: Vec<ColorItem>,
    pub strip_positions: Vec<PositionItem>,
    pub strip_colors: Vec<ColorItem>,
}

/// A [`SystemParam`](bevy_ecs::system::SystemParam) for drawing gizmos.
#[derive(SystemParam)]
pub struct Gizmos<'s> {
    buffer: Deferred<'s, GizmoBuffer>,
}

#[derive(Default)]
struct GizmoBuffer {
    list_positions: Vec<PositionItem>,
    list_colors: Vec<ColorItem>,
    strip_positions: Vec<PositionItem>,
    strip_colors: Vec<ColorItem>,
}

impl SystemBuffer for GizmoBuffer {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let mut storage = world.resource_mut::<GizmoStorage>();
        storage.list_positions.append(&mut self.list_positions);
        storage.list_colors.append(&mut self.list_colors);
        storage.strip_positions.append(&mut self.strip_positions);
        storage.strip_colors.append(&mut self.strip_colors);
    }
}

impl<'s> Gizmos<'s> {
    /// Draw a line from `start` to `end`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line(Vec3::ZERO, Vec3::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.extend_list_positions([start, end]);
        self.add_list_color(color, 2);
    }

    /// Draw a line with a color gradient from `start` to `end`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::ZERO, Vec3::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient(&mut self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        self.extend_list_positions([start, end]);
        self.extend_list_colors([start_color, end_color]);
    }

    /// Draw a line from `start` to `start + vector`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray(Vec3::Y, Vec3::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.line(start, start + vector, color);
    }

    /// Draw a line with a color gradient from `start` to `start + vector`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_gradient(Vec3::Y, Vec3::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient(
        &mut self,
        start: Vec3,
        vector: Vec3,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient(start, start + vector, start_color, end_color);
    }

    /// Draw lines between a list of points.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip([Vec3::ZERO, Vec3::X, Vec3::Y], Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip(&mut self, positions: impl IntoIterator<Item = Vec3>, color: Color) {
        self.extend_strip_positions(positions.into_iter());
        let len = self.buffer.strip_positions.len();
        self.buffer
            .strip_colors
            .resize(len - 1, color.as_linear_rgba_f32());
        self.buffer.strip_colors.push([f32::NAN; 4]);
    }

    /// Draw lines between a list of points with a color gradient.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient([
    ///         (Vec3::ZERO, Color::GREEN),
    ///         (Vec3::X, Color::RED),
    ///         (Vec3::Y, Color::BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient(&mut self, points: impl IntoIterator<Item = (Vec3, Color)>) {
        let points = points.into_iter();

        let GizmoBuffer {
            strip_positions,
            strip_colors,
            ..
        } = &mut *self.buffer;

        let (min, _) = points.size_hint();
        strip_positions.reserve(min);
        strip_colors.reserve(min);

        for (position, color) in points {
            strip_positions.push(position.to_array());
            strip_colors.push(color.as_linear_rgba_f32());
        }

        strip_positions.push([f32::NAN; 3]);
        strip_colors.push([f32::NAN; 4]);
    }

    /// Draw a circle at `position` with the flat side facing `normal`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle(Vec3::ZERO, Vec3::Z, 1., Color::GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle(Vec3::ZERO, Vec3::Z, 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle(
        &mut self,
        position: Vec3,
        normal: Vec3,
        radius: f32,
        color: Color,
    ) -> CircleBuilder<'_, 's> {
        CircleBuilder {
            gizmos: self,
            position,
            normal,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a wireframe sphere made out of 3 circles.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.sphere(Vec3::ZERO, Quat::IDENTITY, 1., Color::BLACK);
    ///
    ///     // Each circle has 32 line-segments by default.
    ///     // You may want to increase this for larger spheres.
    ///     gizmos
    ///         .sphere(Vec3::ZERO, Quat::IDENTITY, 5., Color::BLACK)
    ///         .circle_segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn sphere(
        &mut self,
        position: Vec3,
        rotation: Quat,
        radius: f32,
        color: Color,
    ) -> SphereBuilder<'_, 's> {
        SphereBuilder {
            gizmos: self,
            position,
            rotation,
            radius,
            color,
            circle_segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a wireframe rectangle.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect(Vec3::ZERO, Quat::IDENTITY, Vec2::ONE, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2.extend(0.));
        self.linestrip([tl, tr, br, bl, tl], color);
    }

    /// Draw a wireframe cube.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cuboid(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn cuboid(&mut self, position: Vec3, rotation: Quat, size: Vec3, color: Color) {
        let rect = rect_inner(size.truncate());
        // Front
        let [tlf, trf, brf, blf] = rect.map(|vec2| position + rotation * vec2.extend(size.z / 2.));
        // Back
        let [tlb, trb, brb, blb] = rect.map(|vec2| position + rotation * vec2.extend(-size.z / 2.));

        let positions = [
            tlf, trf, trf, brf, brf, blf, blf, tlf, // Front
            tlb, trb, trb, brb, brb, blb, blb, tlb, // Back
            tlf, tlb, trf, trb, brf, brb, blf, blb, // Front to back
        ];
        self.extend_list_positions(positions);
        self.add_list_color(color, 24);
    }

    /// Draw a line from `start` to `end`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_2d(Vec2::ZERO, Vec2::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: Color) {
        self.line(start.extend(0.), end.extend(0.), color);
    }

    /// Draw a line with a color gradient from `start` to `end`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient_2d(Vec2::ZERO, Vec2::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient_2d(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient(start.extend(0.), end.extend(0.), start_color, end_color);
    }

    /// Draw lines between a list of points.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_2d([Vec2::ZERO, Vec2::X, Vec2::Y], Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_2d(&mut self, positions: impl IntoIterator<Item = Vec2>, color: Color) {
        self.linestrip(positions.into_iter().map(|vec2| vec2.extend(0.)), color);
    }

    /// Draw lines between a list of points with a color gradient.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient_2d([
    ///         (Vec2::ZERO, Color::GREEN),
    ///         (Vec2::X, Color::RED),
    ///         (Vec2::Y, Color::BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient_2d(&mut self, positions: impl IntoIterator<Item = (Vec2, Color)>) {
        self.linestrip_gradient(
            positions
                .into_iter()
                .map(|(vec2, color)| (vec2.extend(0.), color)),
        );
    }

    /// Draw a line from `start` to `start + vector`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_2d(Vec2::Y, Vec2::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: Color) {
        self.line_2d(start, start + vector, color);
    }

    /// Draw a line with a color gradient from `start` to `start + vector`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::Y, Vec3::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient_2d(
        &mut self,
        start: Vec2,
        vector: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient_2d(start, start + vector, start_color, end_color);
    }

    /// Draw a circle.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle_2d(Vec2::ZERO, 1., Color::GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle_2d(Vec2::ZERO, 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle_2d(
        &mut self,
        position: Vec2,
        radius: f32,
        color: Color,
    ) -> Circle2dBuilder<'_, 's> {
        Circle2dBuilder {
            gizmos: self,
            position,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a wireframe rectangle.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect_2d(Vec2::ZERO, 0., Vec2::ONE, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect_2d(&mut self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        let rotation = Mat2::from_angle(rotation);
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_2d([tl, tr, br, bl, tl], color);
    }

    #[inline]
    fn extend_list_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer
            .list_positions
            .extend(positions.into_iter().map(|vec3| vec3.to_array()));
    }

    #[inline]
    fn extend_list_colors(&mut self, colors: impl IntoIterator<Item = Color>) {
        self.buffer
            .list_colors
            .extend(colors.into_iter().map(|color| color.as_linear_rgba_f32()));
    }

    #[inline]
    fn add_list_color(&mut self, color: Color, count: usize) {
        self.buffer
            .list_colors
            .extend(iter::repeat(color.as_linear_rgba_f32()).take(count));
    }

    #[inline]
    fn extend_strip_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer.strip_positions.extend(
            positions
                .into_iter()
                .map(|vec3| vec3.to_array())
                .chain(iter::once([f32::NAN; 3])),
        );
    }
}

/// A builder returned by [`Gizmos::circle`].
pub struct CircleBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    position: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    segments: usize,
}

impl CircleBuilder<'_, '_> {
    /// Set the number of line-segements for this circle.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for CircleBuilder<'_, '_> {
    fn drop(&mut self) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, self.normal);
        let positions = circle_inner(self.radius, self.segments)
            .map(|vec2| (self.position + rotation * vec2.extend(0.)));
        self.gizmos.linestrip(positions, self.color);
    }
}

/// A builder returned by [`Gizmos::sphere`].
pub struct SphereBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    position: Vec3,
    rotation: Quat,
    radius: f32,
    color: Color,
    circle_segments: usize,
}

impl SphereBuilder<'_, '_> {
    /// Set the number of line-segements per circle for this sphere.
    pub fn circle_segments(mut self, segments: usize) -> Self {
        self.circle_segments = segments;
        self
    }
}

impl Drop for SphereBuilder<'_, '_> {
    fn drop(&mut self) {
        for axis in Vec3::AXES {
            self.gizmos
                .circle(self.position, self.rotation * axis, self.radius, self.color)
                .segments(self.circle_segments);
        }
    }
}

/// A builder returned by [`Gizmos::circle_2d`].
pub struct Circle2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    position: Vec2,
    radius: f32,
    color: Color,
    segments: usize,
}

impl Circle2dBuilder<'_, '_> {
    /// Set the number of line-segements for this circle.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for Circle2dBuilder<'_, '_> {
    fn drop(&mut self) {
        let positions = circle_inner(self.radius, self.segments).map(|vec2| (vec2 + self.position));
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

fn circle_inner(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let angle = i as f32 * TAU / segments as f32;
        Vec2::from(angle.sin_cos()) * radius
    })
}

fn rect_inner(size: Vec2) -> [Vec2; 4] {
    let half_size = size / 2.;
    let tl = Vec2::new(-half_size.x, half_size.y);
    let tr = Vec2::new(half_size.x, half_size.y);
    let bl = Vec2::new(-half_size.x, -half_size.y);
    let br = Vec2::new(half_size.x, -half_size.y);
    [tl, tr, br, bl]
}
