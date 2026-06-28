//! This module draws text gizmos using a stroke font.

use crate::simplex_stroke_font::*;
use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};
use bevy_color::Color;
use bevy_math::{vec2, Isometry2d, Isometry3d, Vec2};
use core::ops::Range;

const UNSUPPORTED_BOX: [[i8; 2]; 5] = [[2, 16], [12, 16], [12, 2], [2, 2], [2, 16]];
/// A stroke font containing glyphs for the 95 printable ASCII codes.
pub struct StrokeFont<'a> {
    /// Baseline-to-baseline line height ratio.
    pub line_height: f32,
    /// Full glyph height (cap + descender) in font units.
    pub height: f32,
    /// Cap height in font units.
    pub cap_height: f32,
    /// Advance used for unsupported glyphs.
    pub advance: i8,
    /// Raw glyph point positions.
    pub positions: &'a [[i8; 2]],
    /// Stroke ranges into `positions`.
    pub strokes: &'a [Range<usize>],
    /// Glyph advances and stroke ranges.
    pub glyphs: &'a [(i8, Range<usize>); 95],
    /// Extended glyph point positions.
    pub extended_positions: &'a [[i8; 2]],
    /// Stroke ranges for extended glyphs
    pub extended_strokes: &'a [Range<usize>],
    /// Extended glyphs
    pub extended: &'a [(char, i8, Range<usize>, Range<usize>)],
}

enum GlyphSource {
    Standard(i8, Range<usize>),
    Extended(i8, Range<usize>, Range<usize>),
    Unsupported(i8, ()),
}

impl<'a> StrokeFont<'a> {
    /// Builds a `StrokeTextLayout` for `sections` at the requested `font_size`.
    pub fn layout(
        &'a self,
        sections: &'a [(&'a str, Color)],
        font_size: f32,
    ) -> StrokeTextLayout<'a> {
        let scale = font_size / SIMPLEX_CAP_HEIGHT;
        let glyph_height = SIMPLEX_HEIGHT * scale;
        let line_height = LINE_HEIGHT * glyph_height;
        let margin_top = line_height - glyph_height;
        StrokeTextLayout {
            font: self,
            sections,
            scale,
            line_height,
            margin_top,
        }
    }

    fn get_glyph_index(&self, c: char) -> Option<usize> {
        let code = c as u32;
        if (0x20..=0x7E).contains(&code) {
            Some(code as usize - 0x20)
        } else {
            None
        }
    }

    /// Get the advance and stroke point ranges for a glyph.
    pub fn get_glyph(&self, c: char) -> Option<(i8, Range<usize>)> {
        if let Some(idx) = self.get_glyph_index(c) {
            Some(self.glyphs[idx].clone())
        } else {
            self.extended
                .binary_search_by_key(&c, |entry| entry.0)
                .ok()
                .map(|i| (self.extended[i].1, self.extended[i].2.clone()))
        }
    }

    fn resolve_glyph(&self, c: char) -> GlyphSource {
        if let Some(idx) = self.get_glyph_index(c) {
            GlyphSource::Standard(self.glyphs[idx].0, self.glyphs[idx].1.clone())
        } else if let Ok(i) = self.extended.binary_search_by_key(&c, |entry| entry.0) {
            GlyphSource::Extended(
                self.extended[i].1,
                self.extended[i].2.clone(),
                self.extended[i].3.clone(),
            )
        } else {
            GlyphSource::Unsupported(self.advance, ())
        }
    }

    /// Get the advance for a glyph.
    pub fn get_glyph_advance(&self, c: char) -> Option<i8> {
        self.get_glyph(c).map(|(advance, _)| advance)
    }
}

/// Stroke text layout
pub struct StrokeTextLayout<'a> {
    /// The unscaled font
    font: &'a StrokeFont<'a>,
    /// The text sections with per-section colors.
    sections: &'a [(&'a str, Color)],
    /// Scale applied to the raw glyph positions.
    scale: f32,
    /// Height of each line of text.
    line_height: f32,
    /// Space between top of line and cap height.
    margin_top: f32,
}

impl<'a> StrokeTextLayout<'a> {
    /// Computes the width and height of the text layout.
    ///
    /// Returns the layout size in pixels.
    pub fn measure(&self) -> Vec2 {
        let mut layout_size = vec2(0., self.line_height);

        let mut line_width = 0.;
        for (c, _) in colored_chars(self.sections) {
            if c == '\n' {
                layout_size.x = layout_size.x.max(line_width);
                line_width = 0.;
                layout_size.y += self.line_height;
                continue;
            }
            let advance = match self.font.resolve_glyph(c) {
                GlyphSource::Standard(advance, _)
                | GlyphSource::Extended(advance, _, _)
                | GlyphSource::Unsupported(advance, _) => advance,
            };
            line_width += advance as f32 * self.scale;
        }

        layout_size.x = layout_size.x.max(line_width);
        layout_size
    }

    /// Returns an iterator over the font strokes for this text layout, grouped into polylines
    /// of `Vec2` points, each paired with its color from the text sections.
    pub fn render(
        &'a self,
    ) -> impl Iterator<Item = (Color, Box<dyn Iterator<Item = Vec2> + 'a>)> + 'a {
        let mut chars = colored_chars(self.sections);
        let mut x = 0.0_f32;
        let mut y = -self.margin_top;
        let mut current_main_strokes: Range<usize> = 0..0;
        let mut current_extended_strokes: Range<usize> = 0..0;
        let mut current_x = 0.0_f32;
        let mut current_color = Color::WHITE;
        let mut current_unsupported = false;

        core::iter::from_fn(move || loop {
            if current_unsupported {
                current_unsupported = false;
                let (color, cx) = (current_color, current_x);
                let inner: Box<dyn Iterator<Item = Vec2> + 'a> =
                    Box::new(UNSUPPORTED_BOX.iter().map(move |[p, q]| {
                        Vec2::new(
                            cx + self.scale * *p as f32,
                            y - self.scale * (self.font.cap_height - *q as f32),
                        )
                    }));
                return Some((color, inner));
            }
            for stroke_idx in current_main_strokes.by_ref() {
                let stroke = self.font.strokes[stroke_idx].clone();
                if stroke.len() < 2 {
                    continue;
                }
                // If this stroke is a closed loop, append one extra point to add a join at the
                // seam.
                let join = (self.font.positions[stroke.start]
                    == self.font.positions[stroke.end - 1])
                    .then_some(stroke.start + 1);
                let (color, cx) = (current_color, current_x);
                let inner: Box<dyn Iterator<Item = Vec2> + 'a> =
                    Box::new(stroke.chain(join).map(move |idx| {
                        let [p, q] = self.font.positions[idx];
                        Vec2::new(
                            cx + self.scale * p as f32,
                            y - self.scale * (self.font.cap_height - q as f32),
                        )
                    }));
                return Some((color, inner));
            }

            for stroke_idx in current_extended_strokes.by_ref() {
                let stroke = self.font.extended_strokes[stroke_idx].clone();
                if stroke.len() < 2 {
                    continue;
                }
                let join = (self.font.extended_positions[stroke.start]
                    == self.font.extended_positions[stroke.end - 1])
                    .then_some(stroke.start + 1);
                let (color, cx) = (current_color, current_x);
                let inner: Box<dyn Iterator<Item = Vec2> + 'a> =
                    Box::new(stroke.chain(join).map(move |idx| {
                        let [p, q] = self.font.extended_positions[idx];
                        Vec2::new(
                            cx + self.scale * p as f32,
                            y - self.scale * (self.font.cap_height - q as f32),
                        )
                    }));
                return Some((color, inner));
            }
            let (c, char_color) = chars.next()?;
            if c == '\n' {
                x = 0.;
                y -= self.line_height;
                continue;
            }
            current_color = char_color;
            current_x = x;
            match self.font.resolve_glyph(c) {
                GlyphSource::Standard(advance, stroke_range) => {
                    current_main_strokes = stroke_range;
                    x += advance as f32 * self.scale;
                    current_extended_strokes = 0..0;
                }
                GlyphSource::Extended(advance, main_stroke_range, extended_stroke_range) => {
                    current_main_strokes = main_stroke_range;
                    current_extended_strokes = extended_stroke_range;
                    x += advance as f32 * self.scale;
                }
                GlyphSource::Unsupported(advance, _) => {
                    current_main_strokes = 0..0;
                    current_extended_strokes = 0..0;
                    current_unsupported = true;
                    x += advance as f32 * self.scale;
                }
            }
        })
    }
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw text using a stroke font with the given isometry applied.
    ///
    /// Only ASCII characters in the range 32–126 are supported.
    /// Unsupported characters will be rendered as '?'.
    /// # Arguments
    ///
    /// - `isometry`: defines the translation and rotation of the text.
    /// - `text`: the text to be drawn.
    /// - `size`: the size of the text in pixels.
    /// - `anchor`: normalized anchor point relative to the text bounds,
    ///   where `(0, 0)` is centered, `(-0.5, 0.5)` is top-left,
    ///   and `(0.5, -0.5)` is bottom-right.
    /// - `color`: the color of the text.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.text(Isometry3d::IDENTITY, "text gizmo", 25., Vec2::ZERO, Color::WHITE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn text(
        &mut self,
        isometry: impl Into<Isometry3d>,
        text: &str,
        font_size: f32,
        anchor: Vec2,
        color: impl Into<Color>,
    ) {
        let color: Color = color.into();
        self.text_sections(isometry, &[(text, color)], font_size, anchor);
    }

    /// Draw text using a stroke font with the given isometry applied, coloring each section
    /// independently.
    ///
    /// Only ASCII characters in the range 32–126 are supported.
    /// Unsupported characters will be rendered as '?'.
    /// # Arguments
    ///
    /// - `isometry`: defines the translation and rotation of the text.
    /// - `sections`: a slice of `(text, color)` pairs. Each section's characters are drawn
    ///   in its color. Sections are concatenated left-to-right on the same baseline.
    /// - `font_size`: the size of the text in pixels.
    /// - `anchor`: normalized anchor point relative to the combined text bounds,
    ///   where `(0, 0)` is centered, `(-0.5, 0.5)` is top-left,
    ///   and `(0.5, -0.5)` is bottom-right.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.text_sections(
    ///         Isometry3d::IDENTITY,
    ///         &[("Hello ", Color::WHITE), ("World!", Color::srgb(1., 0.3, 0.))],
    ///         25.,
    ///         Vec2::ZERO,
    ///     );
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn text_sections(
        &mut self,
        isometry: impl Into<Isometry3d>,
        sections: &[(&str, Color)],
        font_size: f32,
        anchor: Vec2,
    ) {
        let isometry: Isometry3d = isometry.into();
        let layout = SIMPLEX_STROKE_FONT.layout(sections, font_size);
        let layout_anchor = layout.measure() * (vec2(-0.5, 0.5) - anchor);
        for (color, points) in layout.render() {
            self.linestrip(
                points.map(|point| isometry * (layout_anchor + point).extend(0.)),
                color,
            );
        }
    }

    /// Draw text using a stroke font in 2d with the given isometry applied.
    ///
    /// Only ASCII characters in the range 32–126 are supported.
    /// Unsupported characters will be rendered as '?'.
    /// # Arguments
    ///
    /// - `isometry`: defines the translation and rotation of the text.
    /// - `text`: the text to be drawn.
    /// - `size`: the size of the text.
    /// - `anchor`: normalized anchor point relative to the text bounds,
    ///   where `(0., 0.)` is centered, `(-0.5, 0.5)` is top-left,
    ///   and `(0.5, -0.5)` is bottom-right.
    /// - `color`: the color of the text.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.text_2d(Isometry2d::IDENTITY, "2D text gizmo", 25., Vec2::ZERO, Color::WHITE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn text_2d(
        &mut self,
        isometry: impl Into<Isometry2d>,
        text: &str,
        font_size: f32,
        anchor: Vec2,
        color: impl Into<Color>,
    ) {
        let color: Color = color.into();
        self.text_sections_2d(isometry, &[(text, color)], font_size, anchor);
    }

    /// Draw text using a stroke font in 2d with the given isometry applied, coloring each section
    /// independently.
    ///
    /// Only ASCII characters in the range 32–126 are supported.
    /// Unsupported characters will be rendered as '?'.
    /// # Arguments
    ///
    /// - `isometry`: defines the translation and rotation of the text.
    /// - `sections`: a slice of `(text, color)` pairs. Each section's characters are drawn
    ///   in its color. Sections are concatenated left-to-right on the same baseline.
    /// - `font_size`: the size of the text.
    /// - `anchor`: normalized anchor point relative to the combined text bounds,
    ///   where `(0., 0.)` is centered, `(-0.5, 0.5)` is top-left,
    ///   and `(0.5, -0.5)` is bottom-right.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.text_sections_2d(
    ///         Isometry2d::IDENTITY,
    ///         &[("Hello ", Color::WHITE), ("World!", Color::srgb(1., 0.3, 0.))],
    ///         25.,
    ///         Vec2::ZERO,
    ///     );
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn text_sections_2d(
        &mut self,
        isometry: impl Into<Isometry2d>,
        sections: &[(&str, Color)],
        font_size: f32,
        anchor: Vec2,
    ) {
        let isometry: Isometry2d = isometry.into();
        let layout = SIMPLEX_STROKE_FONT.layout(sections, font_size);
        let layout_anchor = layout.measure() * (vec2(-0.5, 0.5) - anchor);
        for (color, points) in layout.render() {
            self.linestrip_2d(
                points.map(|point| isometry * (layout_anchor + point)),
                color,
            );
        }
    }
}

/// Iterates the characters across all sections, each paired with its section color.
fn colored_chars<'a>(sections: &'a [(&'a str, Color)]) -> impl Iterator<Item = (char, Color)> + 'a {
    sections
        .iter()
        .flat_map(|&(text, color)| text.chars().map(move |c| (c, color)))
}
