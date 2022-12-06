use std::{f32::consts::TAU, iter};

use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::prelude::Color;
use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::{ColorItem, PositionItem, GIZMO};

pub struct GizmoDraw {
    sender: Sender<Vec<(PositionItem, ColorItem)>>,
    pub(crate) receiver: Receiver<Vec<(PositionItem, ColorItem)>>,
    s_sender: Sender<[(PositionItem, ColorItem); 2]>,
    pub(crate) s_receiver: Receiver<[(PositionItem, ColorItem); 2]>,
}

impl GizmoDraw {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = unbounded();
        let (s_sender, s_receiver) = unbounded();
        GizmoDraw { sender, receiver, s_sender, s_receiver }
    }
}

const CIRCLE_SEGMENTS: usize = 32;

impl GizmoDraw {
    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line(&self, start: Vec3, end: Vec3, color: Color) {
        self.line_gradient(start, end, color, color);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_gradient(&self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        let _ = self.s_sender.send([
            (start.to_array(), start_color.as_linear_rgba_f32()),
            (end.to_array(), end_color.as_linear_rgba_f32()),
        ]);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray(&self, start: Vec3, vector: Vec3, color: Color) {
        self.line(start, start + vector, color);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_gradient(&self, start: Vec3, vector: Vec3, start_color: Color, end_color: Color) {
        self.line_gradient(start, start + vector, start_color, end_color);
    }

    #[inline]
    pub fn linestrip(&self, positions: impl IntoIterator<Item = Vec3>, color: Color) {
        self.linestrip_gradient(positions.into_iter().zip(iter::repeat(color)));
    }

    #[inline]
    pub fn linestrip_gradient(&self, strip: impl IntoIterator<Item = (Vec3, Color)>) {
        let _iter = strip
            .into_iter()
            .map(|(p, c)| (p.to_array(), c.as_linear_rgba_f32()));
        // let _ = self.sender.send(SendItem::Strip(iter.unzip()));
    }

    #[inline]
    fn linelist(&self, list: impl IntoIterator<Item = Vec3>, color: Color) {
        let iter = list
            .into_iter()
            .map(|p| p.to_array())
            .zip(iter::repeat(color.as_linear_rgba_f32()));
        let _ = self.sender.send(iter.collect());
    }

    /// Draw a circle at `position` with the flat side facing `normal`.
    #[inline]
    pub fn circle(&self, position: Vec3, normal: Vec3, radius: f32, color: Color) -> DrawCircle {
        DrawCircle {
            position,
            normal,
            radius,
            color,
            segments: CIRCLE_SEGMENTS,
        }
    }

    /// Draw a sphere.
    #[inline]
    pub fn sphere(&self, position: Vec3, radius: f32, color: Color) -> DrawSphere {
        DrawSphere {
            position,
            radius,
            color,
            circle_segments: CIRCLE_SEGMENTS,
        }
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect(&self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2.extend(0.));
        self.linestrip([tl, tr, br, bl, tl], color);
    }

    /// Draw a box.
    #[inline]
    pub fn cuboid(&self, position: Vec3, rotation: Quat, size: Vec3, color: Color) {
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
        self.linelist(positions, color);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_2d(&self, start: Vec2, end: Vec2, color: Color) {
        self.line(start.extend(0.), end.extend(0.), color);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_gradient_2d(&self, start: Vec2, end: Vec2, start_color: Color, end_color: Color) {
        self.line_gradient(start.extend(0.), end.extend(0.), start_color, end_color);
    }

    #[inline]
    pub fn linestrip_2d(&self, positions: impl IntoIterator<Item = Vec2>, color: Color) {
        self.linestrip(positions.into_iter().map(|vec2| vec2.extend(0.)), color);
    }

    #[inline]
    pub fn linestrip_gradient_2d(&self, positions: impl IntoIterator<Item = (Vec2, Color)>) {
        self.linestrip_gradient(
            positions
                .into_iter()
                .map(|(vec2, color)| (vec2.extend(0.), color)),
        );
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_2d(&self, start: Vec2, vector: Vec2, color: Color) {
        self.line_2d(start, start + vector, color);
    }

    /// Draw a line from `start` to `start + vector`.
    #[inline]
    pub fn ray_gradient_2d(&self, start: Vec2, vector: Vec2, start_color: Color, end_color: Color) {
        self.line_gradient_2d(start, start + vector, start_color, end_color);
    }

    // Draw a circle.
    #[inline]
    pub fn circle_2d(&self, position: Vec2, radius: f32, color: Color) -> DrawCircle2d {
        DrawCircle2d {
            position,
            radius,
            color,
            segments: CIRCLE_SEGMENTS,
        }
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect_2d(&self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        let rotation = Mat2::from_angle(rotation);
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_2d([tl, tr, br, bl, tl], color);
    }
}

pub struct DrawCircle {
    position: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    segments: usize,
}

impl DrawCircle {
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for DrawCircle {
    fn drop(&mut self) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, self.normal);
        let positions = circle_inner(self.radius, self.segments)
            .map(|vec2| (self.position + rotation * vec2.extend(0.)));
        GIZMO.linestrip(positions, self.color);
    }
}
pub struct DrawCircle2d {
    position: Vec2,
    radius: f32,
    color: Color,
    segments: usize,
}

impl DrawCircle2d {
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for DrawCircle2d {
    fn drop(&mut self) {
        let positions = circle_inner(self.radius, self.segments).map(|vec2| (vec2 + self.position));
        GIZMO.linestrip_2d(positions, self.color);
    }
}

pub struct DrawSphere {
    position: Vec3,
    radius: f32,
    color: Color,
    circle_segments: usize,
}

impl DrawSphere {
    pub fn circle_segments(mut self, segments: usize) -> Self {
        self.circle_segments = segments;
        self
    }
}

impl Drop for DrawSphere {
    fn drop(&mut self) {
        for axis in Vec3::AXES {
            GIZMO
                .circle(self.position, axis, self.radius, self.color)
                .segments(self.circle_segments);
        }
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
