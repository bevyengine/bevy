use crate::*;
use bevy_math::Vec2;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};

/// Find the size of the text when rendered with the given parameters.
///
/// Assumes fonts are already loaded.
pub fn measure_text<'a>(
    font_system: &mut cosmic_text::FontSystem,
    scale_factor: f32,
    line_height: f32,
    alignment: Justify,
    width: Option<f32>,
    height: Option<f32>,
    linebreak: LineBreak,
    spans_iter: impl Iterator<Item = (&'a str, &'a FontFaceInfo, f32)>,
) -> Vec2 {
    let mut buffer = Buffer::new(
        font_system,
        Metrics {
            font_size: line_height,
            line_height,
        }
        .scale(scale_factor),
    );

    buffer.set_size(font_system, width, height);
    buffer.set_wrap(
        font_system,
        match linebreak {
            LineBreak::WordBoundary => Wrap::Word,
            LineBreak::AnyCharacter => Wrap::Glyph,
            LineBreak::WordOrCharacter => Wrap::WordOrGlyph,
            LineBreak::NoWrap => Wrap::None,
        },
    );
    buffer.set_rich_text(
        font_system,
        spans_iter.map(|(text, font_face_info, font_size)| {
            (
                text,
                Attrs::new()
                    .family(Family::Name(&font_face_info.family_name))
                    .stretch(font_face_info.stretch)
                    .style(font_face_info.style)
                    .weight(font_face_info.weight)
                    .metrics(
                        Metrics {
                            font_size,
                            line_height,
                        }
                        .scale(scale_factor),
                    ),
            )
        }),
        &Attrs::new(),
        Shaping::Advanced,
        Some(alignment.into()),
    );
    buffer.shape_until_scroll(font_system, false);
    let (width, height) = buffer
        .layout_runs()
        .map(|run| (run.line_w, run.line_height))
        .reduce(|(w1, h1), (w2, h2)| (w1.max(w2), h1 + h2))
        .unwrap_or((0.0, 0.0));
    (Vec2::new(width, height) * scale_factor.recip()).ceil()
}
