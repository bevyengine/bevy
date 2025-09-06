use bevy_asset::Assets;
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    system::{Query, ResMut},
};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{IVec2, Rect, UVec2, Vec2};
use cosmic_text::{Buffer, Edit};

use crate::{
    buffer_dimensions, CosmicFontSystem, FontAtlasSets, FontSmoothing, PasswordMask,
    PositionedGlyph, TextError, TextInputAttributes, TextInputBuffer, TextLayoutInfo,
};

/// Based on `LayoutRunIter` from cosmic-text but fixes a bug where the
/// bottom line should be visible but gets cropped when scrolling upwards.
#[derive(Debug)]
pub struct ScrollingLayoutRunIter<'b> {
    /// Cosmic text buffer
    buffer: &'b Buffer,
    /// Index of the current `BufferLine` (The paragraphs of text before line-breaking)
    paragraph_index: usize,
    /// Index of the current `LayoutLine`, a horizontal line of glyphs from the current `BufferLine` (The individual lines of a paragraph after line-breaking)
    broken_line_index: usize,
    /// Total height of the lines iterated so far
    total_height: f32,
    /// The y-coordinate of the top of the current `LayoutLine`.
    line_top: f32,
}

impl<'b> ScrollingLayoutRunIter<'b> {
    /// Returns a new iterator that iterates the visible lines of the `buffer`.
    pub fn new(buffer: &'b Buffer) -> Self {
        Self {
            buffer,
            paragraph_index: buffer.scroll().line,
            broken_line_index: 0,
            total_height: 0.0,
            line_top: 0.0,
        }
    }
}

impl<'b> Iterator for ScrollingLayoutRunIter<'b> {
    type Item = cosmic_text::LayoutRun<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        // Iterate paragraphs
        while let Some(line) = self.buffer.lines.get(self.paragraph_index) {
            let shape = line.shape_opt()?;
            let layout = line.layout_opt()?;

            // Iterate the paragraph's lines after line-breaking
            while let Some(layout_line) = layout.get(self.broken_line_index) {
                self.broken_line_index += 1;

                let line_height = layout_line
                    .line_height_opt
                    .unwrap_or(self.buffer.metrics().line_height);
                self.total_height += line_height;

                let line_top = self.line_top - self.buffer.scroll().vertical;
                let glyph_height = layout_line.max_ascent + layout_line.max_descent;
                let centering_offset = (line_height - glyph_height) / 2.0;
                let line_bottom = line_top + centering_offset + layout_line.max_ascent;
                if let Some(height) = self.buffer.size().1
                    && height + line_height < line_bottom
                {
                    // The line is below the target bound's bottom edge.
                    // No more lines are visible, return `None` to end the iteration.
                    return None;
                }
                self.line_top += line_height;
                if line_bottom < 0.0 {
                    // The bottom of the line is above the target's bounds top edge and not visible. Skip it.
                    continue;
                }

                return Some(cosmic_text::LayoutRun {
                    line_i: self.paragraph_index,
                    text: line.text(),
                    rtl: shape.rtl,
                    glyphs: &layout_line.glyphs,
                    line_y: line_bottom,
                    line_top,
                    line_height,
                    line_w: layout_line.w,
                });
            }
            self.paragraph_index += 1;
            self.broken_line_index = 0;
        }

        None
    }
}

/// Updates the `TextLayoutInfo` for each text input for rendering.
pub fn update_text_input_layouts(
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut text_query: Query<(
        &mut TextLayoutInfo,
        &mut TextInputBuffer,
        &TextInputAttributes,
        Option<&mut PasswordMask>,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<crate::pipeline::SwashCache>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
) {
    let font_system = &mut font_system.0;
    for (mut layout_info, mut buffer, attributes, mut maybe_password_mask) in text_query.iter_mut()
    {
        // Force a redraw when a password is revealed or hidden
        let force_redraw = maybe_password_mask
            .as_mut()
            .map(|mask| mask.is_changed() && mask.show_password)
            .unwrap_or(false);

        let space_advance = buffer.space_advance;
        let editor = if let Some(password_mask) = maybe_password_mask
            .as_mut()
            .filter(|mask| !mask.show_password)
        {
            // The underlying buffer is hidden, so set redraw to false to avoid unnecessary reupdates.
            buffer.editor.set_redraw(false);
            &mut password_mask.bypass_change_detection().editor
        } else {
            &mut buffer.editor
        };
        editor.shape_as_needed(font_system, false);

        if editor.redraw() || force_redraw {
            layout_info.glyphs.clear();
            layout_info.section_rects.clear();
            layout_info.selection_rects.clear();
            layout_info.cursor_index = None;
            layout_info.cursor = None;

            let selection = editor.selection_bounds();
            let cursor_position = editor.cursor_position();
            let cursor = editor.cursor();

            let result = editor.with_buffer_mut(|buffer| {
                let box_size = buffer_dimensions(buffer);
                let line_height = buffer.metrics().line_height;
                if let Some((x, y)) = cursor_position {
                    let size = Vec2::new(space_advance, line_height);
                    layout_info.cursor = Some((
                        IVec2::new(x, y).as_vec2() + 0.5 * size,
                        size,
                        cursor.affinity.after(),
                    ));
                }
                let result = ScrollingLayoutRunIter::new(buffer).try_for_each(|run| {
                    if let Some(selection) = selection
                        && let Some((x0, w)) = run.highlight(selection.0, selection.1)
                    {
                        let y0 = run.line_top;
                        let y1 = y0 + run.line_height;
                        let x1 = x0 + w;
                        let r = Rect::new(x0, y0, x1, y1);
                        layout_info.selection_rects.push(r);
                    }

                    run.glyphs
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
                                texture_atlases.get(atlas_info.texture_atlas).unwrap();
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
                            if cursor.line == line_i && cursor.index == layout_glyph.start {
                                layout_info.cursor_index = Some(layout_info.glyphs.len() - 1);
                                if let Some((ref mut position, ref mut size, ..)) =
                                    layout_info.cursor
                                {
                                    size.x = layout_glyph.w;
                                    if let Some(cursor_position) = cursor_position {
                                        *position =
                                            IVec2::from(cursor_position).as_vec2() + 0.5 * *size;
                                    }
                                }
                            }

                            Ok(())
                        })
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
                    layout_info.scroll =
                        editor.with_buffer(|buffer| Vec2::new(buffer.scroll().horizontal, 0.));

                    editor.set_redraw(false);
                }
            }
        }
    }
}
