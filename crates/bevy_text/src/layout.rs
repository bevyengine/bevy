use crate::context::FontCx;
use crate::ComputedTextBlock;
use crate::Font;
use crate::LineBreak;
use crate::TextAlign;
use crate::TextBounds;
use crate::TextFont;
use bevy_asset::Assets;
use bevy_color::Color;
use parley::swash::FontRef;
use parley::FontFamily;
use parley::FontStack;
use parley::LineHeight;
use parley::StyleProperty;

pub fn update_buffer(
    fonts: &Assets<Font>,
    text: String,
    text_font: TextFont,
    linebreak: LineBreak,
    justify: TextAlign,
    bounds: TextBounds,
    scale_factor: f32,
    context: &mut FontCx,
) {
    let font = fonts.get(text_font.font.id()).unwrap();
    let (family_id, info) = &font.collection[0];

    let FontCx {
        font_cx,
        layout_cx,
        scale_cx,
    } = context;

    let family_name = font_cx
        .collection
        .family_name(*family_id)
        .unwrap()
        .to_string();

    let mut builder = layout_cx.ranged_builder(font_cx, &text, scale_factor, true);

    builder.push_default(StyleProperty::FontSize(text_font.font_size));
    builder.push_default(LineHeight::Absolute(
        text_font.line_height.eval(text_font.font_size),
    ));

    let stack = FontStack::from(family_name.as_str());
    builder.push_default(stack);

    let layout = builder.build(&text);

    for line in layout.lines() {
        for item in line.items() {
            match item {
                parley::PositionedLayoutItem::GlyphRun(glyph_run) => {
                    let mut run_x = glyph_run.offset();
                    let run_y = glyph_run.baseline();
                    let style = glyph_run.style();

                    let run = glyph_run.run();
                    let font = run.font();
                    let font_size = run.font_size();
                    let normalized_coords = run.normalized_coords();

                    // Convert from parley::Font to swash::FontRef
                    let font_ref =
                        FontRef::from_index(font.data.as_ref(), font.index as usize).unwrap();
                }
                parley::PositionedLayoutItem::InlineBox(positioned_inline_box) => {}
            }
        }
    }
}
