use std::collections::VecDeque;

use bevy_asset::Assets;
use bevy_asset::Handle;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::component::Component;
use bevy_ecs::query::Changed;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::Ref;
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_math::IVec2;
use bevy_math::Vec2;
use cosmic_text::Buffer;
use cosmic_text::Edit;
use cosmic_text::Editor;
use cosmic_text::Metrics;
use cosmic_text::Motion;
use cosmic_text::Wrap;

use crate::input;
use crate::load_font_to_fontdb;
use crate::CosmicFontSystem;
use crate::Font;
use crate::Justify;
use crate::LineBreak;
use crate::LineHeight;
use crate::TextBounds;
use crate::TextError;
use crate::TextFont;
use crate::TextLayoutInfo;
use crate::TextPipeline;

/// Text input buffer
#[derive(Component, Debug)]
pub struct TextInputBuffer {
    pub editor: Editor<'static>,
}

impl Default for TextInputBuffer {
    fn default() -> Self {
        Self {
            editor: Editor::new(Buffer::new_empty(Metrics::new(20.0, 20.0))),
        }
    }
}

#[derive(Component)]
pub struct TextInputData {
    width: f32,
    height: f32,
    font: Handle<Font>,
    font_size: f32,
    line_height: f32,
    wrap: LineBreak,
    justify: Justify,
}

/// Text input commands queue
#[derive(Component, Default)]
pub struct TextInputCommands {
    pub commands_queue: VecDeque<TextInputCommand>,
}

/// Text input commands
pub enum TextInputCommand {
    Submit,
    Copy,
    Cut,
    Paste,
    /// Move the cursor with some motion
    Motion {
        motion: Motion,
        select: bool,
    },
    Insert(char),
    Overwrite(char),
    Enter,
    Backspace,
    Delete,
    Indent,
    Unindent,
    Click(IVec2),
    DoubleClick(IVec2),
    TripleClick(IVec2),
    Drag(IVec2),
    Scroll {
        lines: i32,
    },
    Undo,
    Redo,
    SelectAll,
}

pub fn apply_text_input_commands() {}

/// update editor
pub fn update_text_input_buffers(
    mut text_input_query: Query<(&mut TextInputBuffer, &TextInputData), Changed<TextInputData>>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut text_pipeline: ResMut<TextPipeline>,
    fonts: Res<Assets<Font>>,
) {
    let font_system = &mut font_system.0;
    let font_id_map = &mut text_pipeline.map_handle_to_font_id;
    for (mut input_buffer, data) in text_input_query.iter_mut() {
        input_buffer.editor.with_buffer_mut(|buffer| {
            let metrics = Metrics::new(data.font_size, data.line_height);

            buffer.set_metrics_and_size(font_system, metrics, Some(data.width), Some(data.height));
            buffer.set_wrap(font_system, data.wrap.into());

            if !fonts.contains(data.font.id()) {
                return Err(TextError::NoSuchFont);
            }

            let face_info =
                load_font_to_fontdb(data.font.clone(), font_system, font_id_map, &fonts);

            let attrs = cosmic_text::Attrs::new()
                .metadata(0)
                .family(cosmic_text::Family::Name(&face_info.family_name))
                .stretch(face_info.stretch)
                .style(face_info.style)
                .weight(face_info.weight)
                .metrics(metrics);

            let text = buffer
                .lines
                .iter()
                .map(|buffer_line| buffer_line.text())
                .fold(String::new(), |mut out, line| {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(line);
                    out
                });
            buffer.set_text(font_system, &text, &attrs, cosmic_text::Shaping::Advanced);
            let align = Some(data.justify.into());
            for buffer_line in buffer.lines.iter_mut() {
                buffer_line.set_align(align);
            }

            buffer.set_redraw(true);
            Ok(())
        });

        input_buffer.editor.shape_as_needed(font_system, false);
    }
}

/// Update text input buffers
pub fn update_text_input_layout(
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut text_input_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(&mut TextLayoutInfo, &mut TextInputBuffer)>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    let font_system = &mut font_system.0;
    for (layout_info, mut buffer) in text_query.iter_mut() {
        let editor = &mut buffer.editor;
        let selection = editor.selection_bounds();

        if editor.redraw() {
            layout_info.glyphs.clear();
            layout_info.section_rects.clear();
            layout_info.selection_rects.clear();

            let result = editor.with_buffer_mut(|buffer| {
                let box_size = buffer_dimensions(buffer);
                let result = buffer.layout_runs().try_for_each(|run| {
                    if let Some(selection) = selection {
                        if let Some((x0, w)) = run.highlight(selection.0, selection.1) {
                            let y0 = run.line_top;
                            let y1 = y0 + run.line_height;
                            let x1 = x0 + w;
                            let r = Rect::new(x0, y0, x1, y1);
                            selection_rects.push(r);
                        }
                    }

                    let result = run
                        .glyphs
                        .iter()
                        .map(move |layout_glyph| (layout_glyph, run.line_y, run.line_i))
                        .try_for_each(|(layout_glyph, line_y, line_i)| {
                            let mut temp_glyph;
                            let span_index = layout_glyph.metadata;
                            let font_id = text_font.font.id();
                            let font_smoothing = text_font.font_smoothing;

                            let layout_glyph = if font_smoothing == FontSmoothing::None {
                                // If font smoothing is disabled, round the glyph positions and sizes,
                                // effectively discarding all subpixel layout.
                                temp_glyph = layout_glyph.clone();
                                temp_glyph.x = temp_glyph.x.round();
                                temp_glyph.y = temp_glyph.y.round();
                                temp_glyph.w = temp_glyph.w.round();
                                temp_glyph.x_offset = temp_glyph.x_offset.round();
                                temp_glyph.y_offset = temp_glyph.y_offset.round();
                                temp_glyph.line_height_opt =
                                    temp_glyph.line_height_opt.map(f32::round);

                                &temp_glyph
                            } else {
                                layout_glyph
                            };

                            let TextInputPipeline {
                                font_system,
                                swash_cache,
                                font_atlas_sets,
                                ..
                            } = &mut *text_input_pipeline;

                            let font_atlas_set = font_atlas_sets.entry(font_id).or_default();

                            let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                            let atlas_info = font_atlas_set
                                .get_glyph_atlas_info(physical_glyph.cache_key, font_smoothing)
                                .map(Ok)
                                .unwrap_or_else(|| {
                                    font_atlas_set.add_glyph_to_atlas(
                                        &mut texture_atlases,
                                        &mut textures,
                                        font_system,
                                        swash_cache,
                                        layout_glyph,
                                        font_smoothing,
                                    )
                                })?;

                            let texture_atlas =
                                texture_atlases.get(&atlas_info.texture_atlas).unwrap();
                            let location = atlas_info.location;
                            let glyph_rect = texture_atlas.textures[location.glyph_index];
                            let left = location.offset.x as f32;
                            let top = location.offset.y as f32;
                            let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());

                            // offset by half the size because the origin is center
                            let x = glyph_size.x as f32 / 2.0 + left + physical_glyph.x as f32;
                            let y = line_y.round() + physical_glyph.y as f32 - top
                                + glyph_size.y as f32 / 2.0;

                            let position = Vec2::new(x, y);

                            let pos_glyph = TextInputGlyph {
                                position,
                                size: glyph_size.as_vec2(),
                                atlas_info,
                                span_index,
                                byte_index: layout_glyph.start,
                                byte_length: layout_glyph.end - layout_glyph.start,
                                line_index: line_i,
                            };
                            layout_info.glyphs.push(pos_glyph);
                            Ok(())
                        });

                    result
                });

                // Check result.
                result?;

                layout_info.size = box_size;
                Ok(())
            });

            match result {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, try again next frame
                }
                Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(()) => {
                    layout_info.size.x = layout_info.size.x * node.inverse_scale_factor();
                    layout_info.size.y = layout_info.size.y * node.inverse_scale_factor();
                    editor.set_redraw(false);
                }
            }
        }
    }
}
