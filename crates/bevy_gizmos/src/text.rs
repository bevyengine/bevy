//! Module that generates simple text layouts for text gizmos.

use std::ops::Range;

/// Simple stroke font
pub struct StrokeFont {
    pub advance: f32,
    pub points: Vec<Vec2>,
    pub strokes: Vec<Range<usize>>,
    /// A glyph consists of a number of strokes
    pub glyphs: Vec<Range<usize>>,
}

const ADVANCE: f32 = 0.6;
const LINEHEIGHT: f32 = 1.2;
const POSITIONS: [[f32; 2]; 9] = [
    [0., 1.], //0
    [0.5, 1.],
    [0.5, 0.],
    [0., 0.],
    [0., 1.],   // 4
    [0., 0.5],  // 5
    [0.5, 1.],  // 6
    [0., 1.],   // 7
    [0.5, 0.5], // 8
];
const STROKES: [Range<usize>; 3] = [0..5, 5..7, 7..9];
const GLYPHS: [Range<usize>; 2] = [0..1, 1..3];

use bevy_color::Color;
use bevy_math::{
    curve::{Curve, CurveExt},
    Isometry2d, Vec2, Vec3,
};

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    pub fn text_2d(
        &mut self,
        isometry: impl Into<Isometry2d>,
        text: &str,
        size: f32,
        line_height: f32,
        color: impl Into<Color>,
    ) {
        let isometry = isometry.into();
        let color = color.into();
        let advance = ADVANCE * size;
        let line_height = LINEHEIGHT * size;

        let mut rx = 0.;
        let mut ry = 0.;

        for c in text.chars() {
            if !c.is_ascii() {
                rx += advance;
                continue;
            }

            if c == '\n' {
                rx = 0.;
                ry -= line_height;
                continue;
            }

            if c == ' ' {
                rx += advance;
                continue;
            }

            if !c.is_ascii_alphanumeric() {
                rx += advance;
                continue;
            }

            let i = c.is_ascii_lowercase() as usize;

            let glyph = GLYPHS[i].clone();
            for stroke in &STROKES[glyph] {
                let positions = POSITIONS[stroke.clone()]
                    .iter()
                    .map(|&[x, y]| isometry * Vec2::new(rx + size * x, ry - size * y));
                self.linestrip_2d(positions, color);
            }
            rx += advance;
        }
    }
}
