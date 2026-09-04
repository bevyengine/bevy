//! `EditableText` scrolling logic
//!
//! - [`TextViewport`] is a rectangle aligned to the text layout's axis representing
//!   the user's view of the text layout.
//! - Coordinates are in text layout space, increasing right and downwards.
//! - If the text layout is smaller than the viewport on an axis, the viewport is
//!   given an offset of zero on that the axis.
//! - An origin-size representation is used because the size is generally fixed.
//!   This avoids floating point error accumulation that might happen with min-max coords.
//!
//! # Scrolling rules
//!
//! - The text viewport is controlled exclusively through `TextEdit`s.
//! - Displacement scrolling is continuous and unquantized.
//! - Scrolling to a point moves each axis by the minimum amount needed to make
//!   that point visible.
//! - Scrolling by lines moves by the distance between visual-line starts.
//!   Fractional amounts interpolate across the next line interval in the scroll
//!   direction. This supports wrapped and variable-height lines without
//!   snapping the viewport to line bounds.
//!
//! Text and keyboard edits reveal the caret, including edits that do not change
//! the editor state. Pointer-driven edits and explicit scroll edits do not.
//! Caret reveal follows these rules:
//!
//! - Normalized horizontal and vertical margins inset a caret reveal region from every
//!   viewport edge. Caret movement within this region leaves the viewport unchanged.
//! - If the caret leaves the caret reveal region horizontally, the viewport scrolls
//!   sideways by the smallest amount that brings it back inside, keeping one
//!   `0`-advance of space visible from the caret position onward to leave room for the
//!   next typed character.
//! - Vertically, the viewport scrolls to reveal Parley's caret rectangle,
//!   which spans the whole visual line. The margin-aware offset is clamped
//!   toward the next visual-line start in the scroll direction, so a caret
//!   that moved up or down leaves the viewport aligned to a line in that direction.
//! - If the caret or its visual line is too large to fit inside the caret
//!   reveal region on an axis, then the viewport is centered on it on that axis instead.
//! - The viewport never scrolls outside the layout bounds (and when the
//!   viewport is larger than the layout on an axis, it does not scroll on
//!   that axis at all), even when that leaves the caret closer to the
//!   viewport edge than the margin requests.

use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_reflect::Reflect;

use crate::LineBreak;

/// The region of the editable text layout visible to the user.
///
/// Scrolling changes the offset, size depends on the layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Reflect)]
pub struct TextViewport {
    /// The top-left corner of the text viewport in text-layout coordinates.
    pub offset: Vec2,
    /// The size of the viewport in text-layout coordinates.
    pub size: Vec2,
}

impl TextViewport {
    /// Returns the viewport as a `Rect`.
    pub fn rect(&self) -> Rect {
        Rect {
            min: self.offset,
            max: self.offset + self.size,
        }
    }

    /// Clamp the scroll offset to fit inside `max`.
    pub fn clamp_inside(&mut self, max: Vec2) {
        self.offset = self
            .offset
            .clamp(Vec2::ZERO, (max - self.size).max(Vec2::ZERO));
    }

    /// Scroll by a displacement
    pub fn scroll_by(&mut self, displacement: Vec2, max: Vec2) {
        self.offset += displacement;
        self.clamp_inside(max);
    }

    /// Scroll to a position
    pub fn scroll_to(&mut self, point: Vec2, max: Vec2) {
        self.offset = Vec2::new(
            scroll_to_axis(self.offset.x, self.size.x, point.x),
            scroll_to_axis(self.offset.y, self.size.y, point.y),
        );
        self.clamp_inside(max);
    }

    /// Moves the viewport by the minimum amount needed to reveal the caret.
    pub fn reveal_caret(
        &mut self,
        caret: Rect,
        max: Vec2,
        caret_margin: Vec2,
        lines: impl IntoIterator<Item = TextLineYBounds>,
    ) {
        let mut line_bounds = lines.into_iter().peekable();
        let caret_max = max.max(caret.max.max(Vec2::ZERO));

        self.clamp_inside(caret_max);
        let view = self.rect();
        let margin = caret_margin.clamp(Vec2::ZERO, Vec2::splat(0.5)) * self.size;
        let caret_reveal_region = Rect {
            min: view.min + margin,
            max: view.max - margin,
        };
        if caret_reveal_region.min.cmple(caret.min).all()
            && caret_reveal_region.max.cmpge(caret.max).all()
        {
            return;
        }

        self.offset.x = min_scroll_axis(
            caret_reveal_region.min.x,
            caret_reveal_region.max.x,
            caret.min.x,
            caret.max.x,
        ) - margin.x;
        let vertical_offset = min_scroll_axis(
            caret_reveal_region.min.y,
            caret_reveal_region.max.y,
            caret.min.y,
            caret.max.y,
        ) - margin.y;
        if line_bounds.peek().is_none() {
            self.offset.y = vertical_offset;
        } else if vertical_offset < self.offset.y {
            self.offset.y = line_bounds
                .filter(|line| line.min <= vertical_offset)
                .last()
                .map_or(0.0, |line| line.min);
        } else if self.offset.y < vertical_offset {
            self.offset.y = line_bounds
                .find(|line| vertical_offset <= line.min)
                .map_or((caret_max - self.size).max(Vec2::ZERO).y, |line| line.min);
        }

        self.clamp_inside(caret_max);
    }

    /// Scroll vertically by a number of lines.
    pub fn scroll_by_lines<I>(&mut self, scroll_lines: f32, content_size: Vec2, line_bounds: I)
    where
        I: IntoIterator<Item = TextLineYBounds>,
        I::IntoIter: Clone + DoubleEndedIterator + ExactSizeIterator,
    {
        let line_bounds = line_bounds.into_iter();

        if line_bounds.clone().next().is_none() {
            return;
        }

        let content_size = Vec2::new(content_size.x, line_bounds.clone().next_back().unwrap().max);

        self.clamp_inside(content_size);
        if scroll_lines == 0.0 {
            return;
        }

        let current_line = line_bounds
            .clone()
            .rposition(|line| line.min <= self.offset.y)
            .unwrap_or(0);
        let line_delta = scroll_lines.abs();
        let whole_lines = line_delta.floor() as usize;
        let fraction = line_delta.fract();
        let current_line_bounds = line_bounds.clone().nth(current_line).unwrap();

        if scroll_lines.is_sign_positive() {
            if line_bounds.len() - 1 - current_line < whole_lines {
                self.offset.y = (content_size - self.size).max(Vec2::ZERO).y;
                return;
            }

            let target_line = current_line + whole_lines;
            let target_line_bounds = line_bounds.clone().nth(target_line).unwrap();
            self.offset.y += target_line_bounds.min - current_line_bounds.min
                + fraction
                    * (line_bounds
                        .clone()
                        .nth(target_line + 1)
                        .map_or(target_line_bounds.max, |line| line.min)
                        - target_line_bounds.min);
        } else {
            if current_line < whole_lines {
                self.offset.y = 0.0;
                return;
            }

            let target_line = current_line - whole_lines;
            let target_line_bounds = line_bounds.clone().nth(target_line).unwrap();
            self.offset.y += target_line_bounds.min
                - current_line_bounds.min
                - fraction
                    * (if target_line == 0 {
                        target_line_bounds.max - target_line_bounds.min
                    } else {
                        target_line_bounds.min
                            - line_bounds.clone().nth(target_line - 1).unwrap().min
                    });
        }
        self.clamp_inside(content_size);
    }
}

/// The vertical bounds of one visual line in text-layout coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct TextLineYBounds {
    /// Top edge of the visual line.
    pub min: f32,
    /// Bottom edge of the visual line.
    pub max: f32,
}

impl TextLineYBounds {
    /// Creates line bounds from a top and bottom edge.
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// Creates line bounds from a Parley `Line`.
    pub fn from_line<'a, B: parley::Brush>(line: &parley::Line<'a, B>) -> Self {
        Self {
            min: line.metrics().block_min_coord,
            max: line.metrics().block_max_coord,
        }
    }
}

/// The horizontal extent an editable text viewport may scroll across.
pub fn scrollable_text_layout_width(
    linebreak: LineBreak,
    layout_width: f32,
    viewport_width: f32,
    caret: Option<Rect>,
) -> f32 {
    let scrollable_width = match linebreak {
        LineBreak::NoWrap | LineBreak::WordBoundary => layout_width.max(viewport_width),
        LineBreak::AnyCharacter | LineBreak::WordOrCharacter => viewport_width,
    };

    if linebreak == LineBreak::NoWrap || viewport_width < scrollable_width {
        caret.map_or(scrollable_width, |caret| scrollable_width.max(caret.max.x))
    } else {
        scrollable_width
    }
}

fn scroll_to_axis(view_min: f32, view_size: f32, point: f32) -> f32 {
    if point < view_min {
        point
    } else if view_min + view_size < point {
        point - view_size
    } else {
        view_min
    }
}

fn min_scroll_axis(view_min: f32, view_max: f32, target_min: f32, target_max: f32) -> f32 {
    let view_size = view_max - view_min;
    let target_size = target_max - target_min;

    if view_size < target_size {
        target_min + (target_size - view_size) / 2.
    } else if target_min < view_min {
        target_min
    } else if view_max < target_max {
        target_max - view_size
    } else {
        view_min
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_utils::default;

    const VARIABLE_LINE_BOUNDS: [TextLineYBounds; 5] = [
        TextLineYBounds::new(0.0, 10.0),
        TextLineYBounds::new(10.0, 25.0),
        TextLineYBounds::new(25.0, 45.0),
        TextLineYBounds::new(45.0, 70.0),
        TextLineYBounds::new(70.0, 100.0),
    ];

    fn make_lines(line_count: usize, line_height: f32) -> Vec<TextLineYBounds> {
        (0..line_count)
            .map(move |index| {
                let min = index as f32 * line_height;
                TextLineYBounds::new(min, min + line_height)
            })
            .collect()
    }

    #[test]
    fn scroll_by_lines() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 60.0),
            ..default()
        };
        let lines = make_lines(10, 20.);
        view.scroll_by_lines(2.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        assert_eq!(view.offset, Vec2::new(0.0, 40.0));

        view.scroll_by_lines(3.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        view.scroll_by_lines(-2.0, Vec2::new(100.0, 200.0), lines);
        assert_eq!(view.offset, Vec2::new(0.0, 60.0));
    }

    #[test]
    fn scroll_by_lines_preserves_partial_line_offset() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 55.0),
            ..default()
        };
        let lines = make_lines(10, 20.);

        view.scroll_by(Vec2::new(0.0, 5.0), Vec2::new(100.0, 200.0));

        view.scroll_by_lines(1.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        assert_eq!(view.offset, Vec2::new(0.0, 25.0));

        view.scroll_by_lines(1.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        assert_eq!(view.offset, Vec2::new(0.0, 45.0));

        view.scroll_to(Vec2::new(0.0, 45.0), Vec2::new(100.0, 200.0));

        view.scroll_by_lines(-1.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        assert_eq!(view.offset, Vec2::new(0.0, 25.0));

        view.scroll_by_lines(-1.0, Vec2::new(100.0, 200.0), lines);
        assert_eq!(view.offset, Vec2::new(0.0, 5.0));
    }

    #[test]
    fn scroll_by_fractional_lines() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };
        let lines = make_lines(10, 20.);
        view.scroll_by(Vec2::new(0.0, 45.0), Vec2::new(100.0, 200.0));

        view.scroll_by_lines(0.5, Vec2::new(100.0, 200.0), lines.iter().cloned());
        assert_eq!(view.offset, Vec2::new(0.0, 55.0));

        view.scroll_by_lines(-0.25, Vec2::new(100.0, 200.0), lines);
        assert_eq!(view.offset, Vec2::new(0.0, 50.0));
    }

    #[test]
    fn text_view_zero_line_scroll_clamp() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 60.0),
            ..default()
        };
        let lines = make_lines(10, 20.);
        view.scroll_by_lines(10.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        view.size = Vec2::new(100.0, 180.0);
        view.scroll_by_lines(0.0, Vec2::new(100.0, 200.0), lines);

        assert_eq!(view.offset, Vec2::new(0.0, 20.0));
    }

    #[test]
    fn text_view_scroll_lines_clamps_large_deltas() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 60.0),
            ..default()
        };
        let lines = make_lines(10, 20.);
        view.scroll_by_lines(1000.0, Vec2::new(100.0, 200.0), lines.iter().cloned());
        assert_eq!(view.offset, Vec2::new(0.0, 140.0));

        view.scroll_by_lines(-1000.0, Vec2::new(100.0, 200.0), lines);
        assert_eq!(view.offset, Vec2::ZERO);
    }

    #[test]
    fn text_view_scroll_lines_keeps_offset_zero_when_content_fits() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 200.0),
            ..default()
        };
        view.scroll_by_lines(5.0, Vec2::new(100.0, 80.0), make_lines(4, 20.0));

        assert_eq!(view.offset, Vec2::ZERO);
    }

    #[test]
    fn text_view_scrolls_by_variable_lines() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 40.0),
            ..default()
        };
        view.scroll_by(Vec2::new(0.0, 5.0), Vec2::new(100.0, 100.0));
        view.scroll_by_lines(3.0, Vec2::new(100.0, 100.0), VARIABLE_LINE_BOUNDS);

        assert_eq!(view.offset, Vec2::new(0.0, 50.0));

        view.scroll_by_lines(-1.0, Vec2::new(100.0, 100.0), VARIABLE_LINE_BOUNDS);
        assert_eq!(view.offset, Vec2::new(0.0, 30.0));
    }

    #[test]
    fn text_view_scrolls_by_fractional_variable_lines() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 20.0),
            ..default()
        };
        view.scroll_by(Vec2::new(0.0, 5.0), Vec2::new(100.0, 100.0));

        view.scroll_by_lines(1.5, Vec2::new(100.0, 100.0), VARIABLE_LINE_BOUNDS);
        assert_eq!(view.offset, Vec2::new(0.0, 22.5));

        view.scroll_by_lines(-0.5, Vec2::new(100.0, 100.0), VARIABLE_LINE_BOUNDS);
        assert_eq!(view.offset, Vec2::new(0.0, 17.5));
    }

    #[test]
    fn text_view_fractional_line_scroll_clamps_at_start() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };
        let content_size = Vec2::new(100.0, 200.0);

        view.scroll_by(Vec2::new(0.0, 5.0), content_size);
        view.scroll_by_lines(-0.5, content_size, make_lines(10, 20.));
        assert_eq!(view.offset, Vec2::ZERO);
    }

    #[test]
    fn text_view_scroll_by_clamping() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };

        view.scroll_by(Vec2::new(20.0, 30.0), Vec2::new(80.0, 40.0));
        assert_eq!(view.offset, Vec2::ZERO);

        view.scroll_by(Vec2::new(30.0, 40.0), Vec2::new(250.0, 180.0));
        assert_eq!(view.offset, Vec2::new(30.0, 40.0));

        view.scroll_by(Vec2::new(1000.0, 1000.0), Vec2::new(180.0, 90.0));
        assert_eq!(view.offset, Vec2::new(80.0, 40.0));

        view.scroll_by(Vec2::new(-1000.0, -1000.0), Vec2::new(180.0, 90.0));
        assert_eq!(view.offset, Vec2::ZERO);
    }

    #[test]
    fn text_view_scroll_to_moves_min_distance() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };
        let content_size = Vec2::new(250.0, 180.0);
        view.scroll_by(Vec2::new(30.0, 40.0), content_size);

        view.scroll_to(Vec2::new(80.0, 60.0), content_size);
        assert_eq!(view.offset, Vec2::new(30.0, 40.0));

        view.scroll_by(Vec2::new(20.0, 10.0), content_size);

        view.scroll_to(Vec2::new(40.0, 40.0), content_size);
        assert_eq!(view.offset, Vec2::new(40.0, 40.0));

        view.scroll_to(Vec2::new(160.0, 100.0), content_size);
        assert_eq!(view.offset, Vec2::new(60.0, 50.0));

        view.scroll_to(Vec2::new(60.0, 110.0), content_size);
        assert_eq!(view.offset, Vec2::new(60.0, 60.0));
    }

    #[test]
    fn text_view_scroll_to_clamps_to_content_bounds() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };
        let content_size = Vec2::new(250.0, 180.0);

        view.scroll_to(Vec2::new(1000.0, 1000.0), content_size);
        assert_eq!(view.offset, Vec2::new(150.0, 130.0));

        view.scroll_to(Vec2::new(-1000.0, -1000.0), content_size);
        assert_eq!(view.offset, Vec2::ZERO);
    }

    #[test]
    fn text_view_reveal_caret_keeps_visible_caret_in_view() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 55.0),
            ..default()
        };
        let content_size = Vec2::new(100.0, 200.0);
        view.scroll_by(Vec2::new(0.0, 25.0), content_size);

        view.reveal_caret(
            Rect::new(20.0, 40.0, 22.0, 60.0),
            content_size,
            Vec2::ZERO,
            make_lines(10, 20.0),
        );

        assert_eq!(view.offset, Vec2::new(0.0, 25.0));
    }

    #[test]
    fn text_view_reveal_caret_quantizes_vertical_scroll_to_lines() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 55.0),
            ..default()
        };
        let lines = make_lines(10, 20.);
        let content_size = Vec2::new(100.0, 200.0);

        view.reveal_caret(
            Rect::new(20.0, 60.0, 22.0, 80.0),
            content_size,
            Vec2::ZERO,
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(0.0, 40.0));

        view.scroll_by(Vec2::new(0.0, 100.0), content_size);
        view.reveal_caret(
            Rect::new(20.0, 20.0, 22.0, 40.0),
            content_size,
            Vec2::ZERO,
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(0.0, 20.0));

        view.reveal_caret(
            Rect::new(20.0, 180.0, 22.0, 200.0),
            content_size,
            Vec2::ZERO,
            lines,
        );
        assert_eq!(view.offset, Vec2::new(0.0, 145.0));
    }

    #[test]
    fn text_view_reveal_caret_rounds_margin_scroll_in_direction() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 90.0),
            ..default()
        };
        let lines = make_lines(15, 20.);
        let content_size = Vec2::new(100.0, 300.0);
        view.scroll_by(Vec2::new(0.0, 50.0), content_size);

        view.reveal_caret(
            Rect::new(50.0, 45.0, 52.0, 55.0),
            content_size,
            Vec2::splat(0.2),
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(0.0, 20.0));

        view.scroll_by(Vec2::new(0.0, 30.0), content_size);
        view.reveal_caret(
            Rect::new(50.0, 145.0, 52.0, 155.0),
            content_size,
            Vec2::splat(0.2),
            lines,
        );
        assert_eq!(view.offset, Vec2::new(0.0, 100.0));
    }

    #[test]
    fn text_view_reveal_caret_rounds_to_variable_line_starts() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };
        let content_size = Vec2::new(100.0, 135.0);
        let lines = [
            TextLineYBounds::new(0.0, 10.0),
            TextLineYBounds::new(10.0, 25.0),
            TextLineYBounds::new(25.0, 45.0),
            TextLineYBounds::new(45.0, 70.0),
            TextLineYBounds::new(70.0, 100.0),
            TextLineYBounds::new(100.0, 135.0),
        ];
        view.scroll_by(Vec2::new(0.0, 45.0), content_size);

        view.reveal_caret(
            Rect::new(50.0, 25.0, 52.0, 45.0),
            content_size,
            Vec2::splat(0.2),
            lines,
        );
        assert_eq!(view.offset, Vec2::new(0.0, 10.0));

        view.scroll_by(Vec2::new(0.0, 35.0), content_size);
        view.reveal_caret(
            Rect::new(50.0, 70.0, 52.0, 100.0),
            content_size,
            Vec2::splat(0.2),
            lines,
        );
        assert_eq!(view.offset, Vec2::new(0.0, 70.0));
    }

    #[test]
    fn text_view_reveal_caret_without_line_bounds() {
        let mut view = TextViewport {
            size: Vec2::new(100.0, 50.0),
            ..default()
        };

        view.reveal_caret(
            Rect::new(50.0, 75.0, 52.0, 85.0),
            Vec2::new(100.0, 200.0),
            Vec2::splat(0.2),
            [],
        );

        assert_eq!(view.offset, Vec2::new(0.0, 45.0));
    }

    #[test]
    fn text_view_reveal_caret_scrolls_horizontally_and_clamps() {
        let mut view = TextViewport {
            size: Vec2::new(50.0, 20.0),
            ..default()
        };

        view.reveal_caret(
            Rect::new(95.0, 0.0, 110.0, 20.0),
            Vec2::new(100.0, 20.0),
            Vec2::ZERO,
            make_lines(1, 20.0),
        );

        assert_eq!(view.offset, Vec2::new(60.0, 0.0));
    }

    #[test]
    fn text_view_reveal_caret_keeps_target_inside_margin_safe_region() {
        let mut view = TextViewport {
            size: Vec2::splat(100.0),
            ..default()
        };
        let content_size = Vec2::splat(300.0);
        view.scroll_by(Vec2::splat(50.0), content_size);

        view.reveal_caret(
            Rect::new(80.0, 80.0, 82.0, 100.0),
            content_size,
            Vec2::splat(0.2),
            make_lines(15, 20.0),
        );

        assert_eq!(view.offset, Vec2::splat(50.0));
    }

    #[test]
    fn text_view_reveal_caret_scrolls_minimally_at_each_margin_edge() {
        let mut view = TextViewport {
            size: Vec2::splat(100.0),
            ..default()
        };
        let lines = make_lines(15, 20.);
        let content_size = Vec2::splat(300.0);

        view.scroll_by(Vec2::splat(50.0), content_size);
        view.reveal_caret(
            Rect::new(60.0, 80.0, 62.0, 100.0),
            content_size,
            Vec2::splat(0.2),
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(40.0, 50.0));

        view.scroll_by(Vec2::new(10.0, 0.0), content_size);
        view.reveal_caret(
            Rect::new(140.0, 80.0, 142.0, 100.0),
            content_size,
            Vec2::splat(0.2),
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(62.0, 50.0));

        view.reveal_caret(
            Rect::new(100.0, 45.0, 102.0, 55.0),
            content_size,
            Vec2::splat(0.2),
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(62.0, 20.0));

        view.scroll_by(Vec2::new(0.0, 30.0), content_size);
        view.reveal_caret(
            Rect::new(100.0, 145.0, 102.0, 155.0),
            content_size,
            Vec2::splat(0.2),
            lines,
        );
        assert_eq!(view.offset, Vec2::new(62.0, 80.0));
    }

    #[test]
    fn text_view_reveal_caret_independent_margins() {
        let mut view = TextViewport {
            size: Vec2::splat(100.0),
            ..default()
        };
        let lines = make_lines(15, 20.);
        let content_size = Vec2::splat(300.0);
        view.scroll_by(Vec2::splat(50.0), content_size);

        view.reveal_caret(
            Rect::new(140.0, 75.0, 142.0, 95.0),
            content_size,
            Vec2::new(0.2, 0.0),
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::new(62.0, 50.0));

        view.reveal_caret(
            Rect::new(80.0, 120.0, 82.0, 140.0),
            content_size,
            Vec2::new(0.0, 0.2),
            lines,
        );
        assert_eq!(view.offset, Vec2::new(62.0, 60.0));
    }

    #[test]
    fn text_view_reveal_caret_margin_clamped_at_layout_edges() {
        let mut view = TextViewport {
            size: Vec2::splat(100.0),
            ..default()
        };
        let lines = make_lines(10, 20.);
        let content_size = Vec2::splat(200.0);

        view.reveal_caret(
            Rect::new(0.0, 0.0, 2.0, 20.0),
            content_size,
            Vec2::splat(0.2),
            lines.iter().cloned(),
        );
        assert_eq!(view.offset, Vec2::ZERO);

        view.scroll_by(Vec2::splat(100.0), content_size);
        view.reveal_caret(
            Rect::new(198.0, 180.0, 200.0, 200.0),
            content_size,
            Vec2::splat(0.2),
            lines,
        );
        assert_eq!(view.offset, Vec2::splat(100.0));
    }

    #[test]
    fn character_wrapped_text_returns_viewport_width() {
        for linebreak in [LineBreak::AnyCharacter, LineBreak::WordOrCharacter] {
            assert_eq!(
                scrollable_text_layout_width(
                    linebreak,
                    102.0,
                    100.0,
                    Some(Rect::new(95.0, 0.0, 110.0, 20.0))
                ),
                100.0
            );
        }
    }

    #[test]
    fn word_boundary_wrapping_preserves_overflow() {
        assert_eq!(
            scrollable_text_layout_width(LineBreak::WordBoundary, 150.0, 100.0, None,),
            150.0
        );
    }

    #[test]
    fn overflowing_wrapped_text_includes_trailing_caret() {
        assert_eq!(
            scrollable_text_layout_width(
                LineBreak::WordBoundary,
                102.0,
                100.0,
                Some(Rect::new(95.0, 0.0, 110.0, 20.0)),
            ),
            110.0
        );
    }

    #[test]
    fn no_wrap_text_includes_trailing_caret() {
        assert_eq!(
            scrollable_text_layout_width(
                LineBreak::NoWrap,
                102.0,
                100.0,
                Some(Rect::new(95.0, 0.0, 110.0, 20.0)),
            ),
            110.0
        );
    }

    #[test]
    fn scrollable_text_word_boundary() {
        assert_eq!(
            scrollable_text_layout_width(
                LineBreak::WordBoundary,
                150.,
                100.,
                Some(Rect::new(150., 0., 160.0, 10.0)),
            ),
            160.
        );
    }
}
