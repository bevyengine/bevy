use std::{f32::consts::TAU, iter};

use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::prelude::Color;
use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::SendItem;

pub struct GizmoDraw {
    sender: Sender<SendItem>,
    pub(crate) receiver: Receiver<SendItem>,
    circle_segments: usize,
}

impl GizmoDraw {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = unbounded();
        GizmoDraw {
            sender,
            receiver,
            circle_segments: 32,
        }
    }
}

impl GizmoDraw {
    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line(&self, start: Vec3, end: Vec3, color: Color) {
        self.send_line([start, end], [color, color]);
    }

    /// Draw a line from `start` to `end`.
    #[inline]
    pub fn line_gradient(&self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        self.send_line([start, end], [start_color, end_color]);
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
        let iter = strip
            .into_iter()
            .map(|(p, c)| (p.to_array(), c.as_linear_rgba_f32()));
        let _ = self.sender.send(SendItem::Strip(iter.unzip()));
    }

    /// Draw a circle at `position` with the flat side facing `normal`.
    #[inline]
    pub fn circle(&self, position: Vec3, normal: Vec3, radius: f32, color: Color) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

        let positions = self
            .circle_inner(radius)
            .map(|vec2| (position + rotation * vec2.extend(0.)));

        self.linestrip(positions, color);
    }

    /// Draw a sphere.
    #[inline]
    pub fn sphere(&self, position: Vec3, radius: f32, color: Color) {
        self.circle(position, Vec3::X, radius, color);
        self.circle(position, Vec3::Y, radius, color);
        self.circle(position, Vec3::Z, radius, color);
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect(&self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        let [tl, tr, br, bl] = self
            .rect_inner(size)
            .map(|vec2| position + rotation * vec2.extend(0.));
        self.linestrip([tl, tr, br, bl, tl], color);
    }

    /// Draw a box.
    #[inline]
    pub fn cuboid(&self, position: Vec3, rotation: Quat, size: Vec3, color: Color) {
        let rect = self.rect_inner(size.truncate());
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
    pub fn circle_2d(&self, position: Vec2, radius: f32, color: Color) {
        let positions = self.circle_inner(radius).map(|vec2| (vec2 + position));
        self.linestrip_2d(positions, color);
    }

    /// Draw a rectangle.
    #[inline]
    pub fn rect_2d(&self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        let rotation = Mat2::from_angle(rotation);
        let [tl, tr, br, bl] = self.rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_2d([tl, tr, br, bl, tl], color);
    }

    fn circle_inner(&self, radius: f32) -> impl Iterator<Item = Vec2> {
        let circle_segments = self.circle_segments;
        (0..(circle_segments + 1)).into_iter().map(move |i| {
            let angle = i as f32 * TAU / circle_segments as f32;
            Vec2::from(angle.sin_cos()) * radius
        })
    }

    fn rect_inner(&self, size: Vec2) -> [Vec2; 4] {
        let half_size = size / 2.;
        let tl = Vec2::new(-half_size.x, half_size.y);
        let tr = Vec2::new(half_size.x, half_size.y);
        let bl = Vec2::new(-half_size.x, -half_size.y);
        let br = Vec2::new(half_size.x, -half_size.y);
        [tl, tr, br, bl]
    }

    #[inline]
    fn send_line(&self, [p0, p1]: [Vec3; 2], [c0, c1]: [Color; 2]) {
        let _ = self.sender.send(SendItem::Single((
            [p0.to_array(), p1.to_array()],
            [c0.as_linear_rgba_f32(), c1.as_linear_rgba_f32()],
        )));
    }

    #[inline]
    fn linelist(&self, list: impl IntoIterator<Item = Vec3>, color: Color) {
        let iter = list
            .into_iter()
            .map(|p| p.to_array())
            .zip(iter::repeat(color.as_linear_rgba_f32()));
        let _ = self.sender.send(SendItem::List(iter.unzip()));
    }
}
