#![allow(missing_docs)]

use std::collections::VecDeque;

use bevy_app::Plugin;
use bevy_app::PostUpdate;
use bevy_asset::Assets;
use bevy_asset::Handle;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::Changed;
use bevy_ecs::query::Or;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::schedule::SystemSet;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::Ref;
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_log::info_once;
use bevy_math::IVec2;
use bevy_math::Rect;
use bevy_math::UVec2;
use bevy_math::Vec2;
use cosmic_text::Action;
use cosmic_text::BorrowedWithFontSystem;
use cosmic_text::Buffer;
use cosmic_text::Edit;
use cosmic_text::Editor;
use cosmic_text::FontSystem;
use cosmic_text::Metrics;
pub use cosmic_text::Motion;
use cosmic_text::Selection;
use cosmic_text::SwashCache;
use cosmic_text::Wrap;
use tracing::info;

use crate::buffer_dimensions;
use crate::input;
use crate::load_font_to_fontdb;
use crate::CosmicFontSystem;
use crate::Font;
use crate::FontAtlas;
use crate::FontAtlasSets;
use crate::FontSmoothing;
use crate::Justify;
use crate::LineBreak;
use crate::LineHeight;
use crate::PositionedGlyph;
use crate::TextBounds;
use crate::TextError;
use crate::TextFont;
use crate::TextLayoutInfo;
use crate::TextPipeline;

pub struct TextInputPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct TextInputSystems;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                update_text_input_buffers,
                apply_text_input_actions,
                update_text_input_layouts,
            )
                .chain()
                .in_set(TextInputSystems),
        );
    }
}

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

#[derive(Component, PartialEq, Debug, Default)]
pub struct TextInputTarget {
    pub size: Vec2,
    pub scale_factor: f32,
}

#[derive(Component)]
pub struct TextInputAttributes {
    font: Handle<Font>,
    font_size: f32,
    line_height: f32,
    wrap: LineBreak,
    justify: Justify,
    font_smoothing: FontSmoothing,
}

impl Default for TextInputAttributes {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.,
            line_height: 1.2,
            font_smoothing: Default::default(),
            justify: Default::default(),
            wrap: Default::default(),
        }
    }
}

/// Text input commands queue
#[derive(Component, Default)]
pub struct TextInputActions {
    pub queue: VecDeque<TextInputAction>,
}

impl TextInputActions {
    pub fn queue(&mut self, command: TextInputAction) {
        info!("queue action: {:?}", command);
        self.queue.push_back(command);
    }
}

/// Text input commands
#[derive(Debug)]
pub enum TextInputAction {
    Submit,
    Copy,
    Cut,
    Paste,
    /// Move the cursor with some motion
    Motion {
        motion: Motion,
        with_select: bool,
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
    SelectLine,
    Escape,
}

impl TextInputAction {
    pub fn motion(motion: Motion, with_select: bool) -> Self {
        Self::Motion {
            motion,
            with_select,
        }
    }
}

/// apply a motion action to the editor buffer
pub fn apply_motion<'a>(
    editor: &mut BorrowedWithFontSystem<Editor<'a>>,
    shift_pressed: bool,
    motion: Motion,
) {
    if shift_pressed {
        if editor.selection() == Selection::None {
            let cursor = editor.cursor();
            editor.set_selection(Selection::Normal(cursor));
        }
    } else {
        editor.action(Action::Escape);
    }
    editor.action(Action::Motion(motion));
}

pub fn cursor_at_line_end(editor: &mut BorrowedWithFontSystem<Editor<'_>>) -> bool {
    let cursor = editor.cursor();
    editor.with_buffer(|buffer| {
        buffer
            .lines
            .get(cursor.line)
            .map(|line| cursor.index == line.text().len())
            .unwrap_or(false)
    })
}

pub fn apply_text_input_actions(
    mut font_system: ResMut<CosmicFontSystem>,
    mut text_input_query: Query<(Entity, &mut TextInputBuffer, &mut TextInputActions)>,
) {
    info_once!("apply_text_input_actions");
    for (entity, mut buffer, mut text_input_actions) in text_input_query.iter_mut() {
        let mut editor = buffer.editor.borrow_with(&mut font_system);

        while let Some(action) = text_input_actions.queue.pop_front() {
            editor.start_change();

            match action {
                TextInputAction::Submit => {}
                TextInputAction::Copy => {}
                TextInputAction::Cut => {}
                TextInputAction::Paste => {}
                TextInputAction::Motion {
                    motion,
                    with_select,
                } => {
                    apply_motion(&mut editor, with_select, motion);
                }
                TextInputAction::Insert(ch) => {
                    // else if max_chars
                    //     .is_none_or(|max_chars| editor.with_buffer(buffer_len) < max_chars)
                    editor.action(Action::Insert(ch));
                }
                TextInputAction::Overwrite(ch) => {
                    match editor.selection() {
                        Selection::None => {
                            if !cursor_at_line_end(&mut editor) {
                                editor.action(Action::Delete);
                                editor.action(Action::Insert(ch));
                            } else {
                                // else if max_chars
                                //     .is_none_or(|max_chars| editor.with_buffer(buffer_len) < max_chars)
                                editor.action(Action::Insert(ch));
                            }
                        }
                        _ => editor.action(Action::Insert(ch)),
                    }
                }
                TextInputAction::Enter => {
                    editor.action(Action::Enter);
                }
                TextInputAction::Backspace => {
                    if editor.delete_selection() {
                        editor.set_redraw(true);
                    } else {
                        editor.action(Action::Backspace);
                    }
                }
                TextInputAction::Delete => {
                    if editor.delete_selection() {
                        editor.set_redraw(true);
                    } else {
                        editor.action(Action::Delete);
                    }
                }
                TextInputAction::Indent => {
                    editor.action(Action::Indent);
                }
                TextInputAction::Unindent => {
                    editor.action(Action::Unindent);
                }
                TextInputAction::Click(point) => {
                    editor.action(Action::Click {
                        x: point.x,
                        y: point.y,
                    });
                }
                TextInputAction::DoubleClick(point) => {
                    editor.action(Action::DoubleClick {
                        x: point.x,
                        y: point.y,
                    });
                }
                TextInputAction::TripleClick(point) => {
                    editor.action(Action::TripleClick {
                        x: point.x,
                        y: point.y,
                    });
                }
                TextInputAction::Drag(point) => {
                    editor.action(Action::Drag {
                        x: point.x,
                        y: point.y,
                    });
                }
                TextInputAction::Scroll { lines } => {
                    editor.action(Action::Scroll { lines });
                }
                TextInputAction::Undo => {}
                TextInputAction::Redo => {}
                TextInputAction::SelectAll => {
                    editor.action(Action::Motion(Motion::BufferStart));
                    let cursor = editor.cursor();
                    editor.set_selection(Selection::Normal(cursor));
                    editor.action(Action::Motion(Motion::BufferEnd));
                }
                TextInputAction::SelectLine => {
                    editor.action(Action::Motion(Motion::Home));
                    let cursor = editor.cursor();
                    editor.set_selection(Selection::Normal(cursor));
                    editor.action(Action::Motion(Motion::End));
                }
                TextInputAction::Escape => {
                    editor.set_selection(Selection::None);
                }
            }

            if let Some(_change) = editor.finish_change() {
                editor.set_redraw(true);
            }
        }
    }
}

/// update editor
pub fn update_text_input_buffers(
    mut text_input_query: Query<(
        &mut TextInputBuffer,
        Ref<TextInputTarget>,
        Ref<TextInputAttributes>,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut text_pipeline: ResMut<TextPipeline>,
    fonts: Res<Assets<Font>>,
) {
    info_once!(" update_text_input_buffers");
    let font_system = &mut font_system.0;
    let font_id_map = &mut text_pipeline.map_handle_to_font_id;
    for (mut input_buffer, target, attributes) in text_input_query.iter_mut() {
        if target.is_changed() || attributes.is_changed() {
            let _ = input_buffer.editor.with_buffer_mut(|buffer| {
                let metrics = Metrics::relative(attributes.font_size, attributes.line_height)
                    .scale(target.scale_factor);

                buffer.set_metrics_and_size(
                    font_system,
                    metrics,
                    Some(target.size.x),
                    Some(target.size.y),
                );
                buffer.set_wrap(font_system, attributes.wrap.into());

                if !fonts.contains(attributes.font.id()) {
                    return Err(TextError::NoSuchFont);
                }

                let face_info =
                    load_font_to_fontdb(attributes.font.clone(), font_system, font_id_map, &fonts);

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
                let align = Some(attributes.justify.into());
                for buffer_line in buffer.lines.iter_mut() {
                    buffer_line.set_align(align);
                }

                buffer.set_redraw(true);
                Ok(())
            });
        }
    }
}

/// Update text input buffers
pub fn update_text_input_layouts(
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut text_query: Query<(
        &mut TextLayoutInfo,
        &mut TextInputBuffer,
        &mut TextInputAttributes,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<crate::pipeline::SwashCache>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
) {
    info_once!(" update_text_input_layouts");
    let font_system = &mut font_system.0;
    for (mut layout_info, mut buffer, attributes) in text_query.iter_mut() {
        let editor = &mut buffer.editor;
        let selection = editor.selection_bounds();
        editor.shape_as_needed(font_system, false);

        if editor.redraw() {
            info!("** redraw editor **");
            layout_info.glyphs.clear();
            layout_info.section_rects.clear();
            layout_info.selection_rects.clear();

            let result = editor.with_buffer_mut(|buffer| {
                let box_size = buffer_dimensions(buffer);
                info!("box_size = {}", box_size);
                let result = buffer.layout_runs().try_for_each(|run| {
                    if let Some(selection) = selection {
                        if let Some((x0, w)) = run.highlight(selection.0, selection.1) {
                            let y0 = run.line_top;
                            let y1 = y0 + run.line_height;
                            let x1 = x0 + w;
                            let r = Rect::new(x0, y0, x1, y1);
                            layout_info.selection_rects.push(r);
                        }
                    }

                    let result = run
                        .glyphs
                        .iter()
                        .map(move |layout_glyph| (layout_glyph, run.line_y, run.line_i))
                        .try_for_each(|(layout_glyph, line_y, line_i)| {
                            let mut temp_glyph;
                            let span_index = layout_glyph.metadata;
                            let font_id = attributes.font.id();
                            let font_smoothing = attributes.font_smoothing;

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

                            let font_atlas_set = font_atlas_sets.sets.entry(font_id).or_default();

                            let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                            let atlas_info = font_atlas_set
                                .get_glyph_atlas_info(physical_glyph.cache_key, font_smoothing)
                                .map(Ok)
                                .unwrap_or_else(|| {
                                    font_atlas_set.add_glyph_to_atlas(
                                        &mut texture_atlases,
                                        &mut textures,
                                        font_system,
                                        &mut swash_cache.0,
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

                            let pos_glyph = PositionedGlyph {
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
                    layout_info.size.x = layout_info.size.x;
                    layout_info.size.y = layout_info.size.y;
                    editor.set_redraw(false);
                }
            }
        }
    }
}
