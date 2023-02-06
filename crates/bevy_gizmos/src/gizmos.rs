use std::{f32::consts::TAU, iter};

use bevy_ecs::{
    system::{Buffer, Resource, SystemBuffer},
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

pub type Gizmos<'s> = Buffer<'s, GizmoBuffer>;

#[derive(Default)]
pub struct GizmoBuffer {
    list_positions: Vec<PositionItem>,
    list_colors: Vec<ColorItem>,
    strip_positions: Vec<PositionItem>,
    strip_colors: Vec<ColorItem>,
}

impl SystemBuffer for GizmoBuffer {
    fn apply(&mut self, world: &mut World) {
        let mut storage = world.resource_mut::<GizmoStorage>();
        storage.list_positions.append(&mut self.list_positions);
        storage.list_colors.append(&mut self.list_colors);
        storage.strip_positions.append(&mut self.strip_positions);
        storage.strip_colors.append(&mut self.strip_colors);
    }
}

impl GizmoBuffer {
    #[inline]
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.extend_list_positions([start, end]);
        self.add_list_color(color, 2);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_gradient(&mut self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        self.extend_list_positions([start, end]);
        self.extend_list_colors([start_color, end_color]);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.line(start, start + vector, color);
    }

    /// Draw a line from `start` to `start + vector`.
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

    #[inline]
    pub fn linestrip(&mut self, positions: impl IntoIterator<Item = Vec3>, color: Color) {
        self.extend_strip_positions(positions.into_iter());
        self.strip_colors
            .resize(self.strip_positions.len() - 1, color.as_linear_rgba_f32());
        self.strip_colors.push([f32::NAN; 4]);
    }

    #[inline]
    pub fn linestrip_gradient(&mut self, points: impl IntoIterator<Item = (Vec3, Color)>) {
        let points = points.into_iter();

        let (min, _) = points.size_hint();
        self.strip_positions.reserve(min);
        self.strip_colors.reserve(min);

        for (position, color) in points {
            self.strip_positions.push(position.to_array());
            self.strip_colors.push(color.as_linear_rgba_f32());
        }

        self.strip_positions.push([f32::NAN; 3]);
        self.strip_colors.push([f32::NAN; 4]);
    }

    /// Draw a circle at `position` with the flat side facing `normal`.
    #[inline]
    pub fn circle(
        &mut self,
        position: Vec3,
        normal: Vec3,
        radius: f32,
        color: Color,
    ) -> CircleBuilder {
        CircleBuilder {
            buffer: self,
            position,
            normal,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a sphere.
    #[inline]
    pub fn sphere(&mut self, position: Vec3, radius: f32, color: Color) -> SphereBuilder {
        SphereBuilder {
            buffer: self,
            position,
            radius,
            color,
            circle_segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2.extend(0.));
        self.linestrip([tl, tr, br, bl, tl], color);
    }

    /// Draw a box.
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
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: Color) {
        self.line(start.extend(0.), end.extend(0.), color);
    }

    /// Draw a line from `start` to `end`.
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

    #[inline]
    pub fn linestrip_2d(&mut self, positions: impl IntoIterator<Item = Vec2>, color: Color) {
        self.linestrip(positions.into_iter().map(|vec2| vec2.extend(0.)), color);
    }

    #[inline]
    pub fn linestrip_gradient_2d(&mut self, positions: impl IntoIterator<Item = (Vec2, Color)>) {
        self.linestrip_gradient(
            positions
                .into_iter()
                .map(|(vec2, color)| (vec2.extend(0.), color)),
        );
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: Color) {
        self.line_2d(start, start + vector, color);
    }

    /// Draw a line from `start` to `start + vector`.
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

    // Draw a circle.
    #[inline]
    pub fn circle_2d(&mut self, position: Vec2, radius: f32, color: Color) -> Circle2dBuilder {
        Circle2dBuilder {
            buffer: self,
            position,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect_2d(&mut self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        let rotation = Mat2::from_angle(rotation);
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_2d([tl, tr, br, bl, tl], color);
    }

    #[inline]
    fn extend_list_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.list_positions
            .extend(positions.into_iter().map(|vec3| vec3.to_array()));
    }

    #[inline]
    fn extend_list_colors(&mut self, colors: impl IntoIterator<Item = Color>) {
        self.list_colors
            .extend(colors.into_iter().map(|color| color.as_linear_rgba_f32()));
    }

    #[inline]
    fn add_list_color(&mut self, color: Color, count: usize) {
        self.list_colors
            .extend(iter::repeat(color.as_linear_rgba_f32()).take(count));
    }

    #[inline]
    fn extend_strip_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.strip_positions.extend(
            positions
                .into_iter()
                .map(|vec3| vec3.to_array())
                .chain(iter::once([f32::NAN; 3])),
        );
    }
}

pub struct CircleBuilder<'a> {
    buffer: &'a mut GizmoBuffer,
    position: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    segments: usize,
}

impl<'a> CircleBuilder<'a> {
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<'a> Drop for CircleBuilder<'a> {
    fn drop(&mut self) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, self.normal);
        let positions = circle_inner(self.radius, self.segments)
            .map(|vec2| (self.position + rotation * vec2.extend(0.)));
        self.buffer.linestrip(positions, self.color);
    }
}

pub struct SphereBuilder<'a> {
    buffer: &'a mut GizmoBuffer,
    position: Vec3,
    radius: f32,
    color: Color,
    circle_segments: usize,
}

impl SphereBuilder<'_> {
    pub fn circle_segments(mut self, segments: usize) -> Self {
        self.circle_segments = segments;
        self
    }
}

impl Drop for SphereBuilder<'_> {
    fn drop(&mut self) {
        for axis in Vec3::AXES {
            self.buffer
                .circle(self.position, axis, self.radius, self.color)
                .segments(self.circle_segments);
        }
    }
}

pub struct Circle2dBuilder<'a> {
    buffer: &'a mut GizmoBuffer,
    position: Vec2,
    radius: f32,
    color: Color,
    segments: usize,
}

impl Circle2dBuilder<'_> {
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for Circle2dBuilder<'_> {
    fn drop(&mut self) {
        let positions = circle_inner(self.radius, self.segments).map(|vec2| (vec2 + self.position));
        self.buffer.linestrip_2d(positions, self.color);
    }
}

fn circle_inner(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).into_iter().map(move |i| {
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
