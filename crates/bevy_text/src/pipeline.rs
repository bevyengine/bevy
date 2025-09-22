use alloc::sync::Arc;

use bevy_asset::{AssetId, Assets};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component, entity::Entity, reflect::ReflectComponent, resource::Resource,
    system::ResMut,
};
use bevy_image::prelude::*;
use bevy_log::{once, warn};
use bevy_math::{Rect, UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};

use crate::{
    error::TextError, ComputedTextBlock, Font, FontAtlasSets, FontSmoothing, Justify, LineBreak,
    PositionedGlyph, TextBounds, TextEntity, TextFont, TextLayout,
};

/// A wrapper resource around a [`cosmic_text::FontSystem`]
///
/// The font system is used to retrieve fonts and their information, including glyph outlines.
///
/// This resource is updated by the [`TextPipeline`] resource.
#[derive(Resource, Deref, DerefMut)]
pub struct CosmicFontSystem(pub cosmic_text::FontSystem);

impl Default for CosmicFontSystem {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        let db = cosmic_text::fontdb::Database::new();
        // TODO: consider using `cosmic_text::FontSystem::new()` (load system fonts by default)
        Self(cosmic_text::FontSystem::new_with_locale_and_db(locale, db))
    }
}

/// A wrapper resource around a [`cosmic_text::SwashCache`]
///
/// The swash cache rasterizer is used to rasterize glyphs
///
/// This resource is updated by the [`TextPipeline`] resource.
#[derive(Resource)]
pub struct SwashCache(pub cosmic_text::SwashCache);

impl Default for SwashCache {
    fn default() -> Self {
        Self(cosmic_text::SwashCache::new())
    }
}

/// Information about a font collected as part of preparing for text layout.
#[derive(Clone)]
pub struct FontFaceInfo {
    /// Width class: <https://docs.microsoft.com/en-us/typography/opentype/spec/os2#uswidthclass>
    pub stretch: cosmic_text::fontdb::Stretch,
    /// Allows italic or oblique faces to be selected
    pub style: cosmic_text::fontdb::Style,
    /// The degree of blackness or stroke thickness
    pub weight: cosmic_text::fontdb::Weight,
    /// Font family name
    pub family_name: Arc<str>,
}

/// The `TextPipeline` is used to layout and render text blocks (see `Text`/`Text2d`).
///
/// See the [crate-level documentation](crate) for more information.
#[derive(Default, Resource)]
pub struct TextPipeline {
    /// Identifies a font [`ID`](cosmic_text::fontdb::ID) by its [`Font`] [`Asset`](bevy_asset::Asset).
    pub map_handle_to_font_id: HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, Arc<str>)>,
    /// Buffered vec for collecting spans.
    ///
    /// See [this dark magic](https://users.rust-lang.org/t/how-to-cache-a-vectors-capacity/94478/10).
    spans_buffer: Vec<(usize, &'static str, &'static TextFont, FontFaceInfo)>,
    /// Buffered vec for collecting info for glyph assembly.
    glyph_info: Vec<(AssetId<Font>, FontSmoothing)>,
}

impl TextPipeline {
    /// Utilizes [`cosmic_text::Buffer`] to shape and layout text
    ///
    /// Negative or 0.0 font sizes will not be laid out.
    pub fn update_buffer<'a>(
        &mut self,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color)>,
        linebreak: LineBreak,
        justify: Justify,
        bounds: TextBounds,
        scale_factor: f64,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
    ) -> Result<(), TextError> {
        let font_system = &mut font_system.0;

        // Collect span information into a vec. This is necessary because font loading requires mut access
        // to FontSystem, which the cosmic-text Buffer also needs.
        let mut max_font_size: f32 = 0.;
        let mut max_line_height: f32 = 0.0;
        let mut spans: Vec<(usize, &str, &TextFont, FontFaceInfo, Color)> =
            core::mem::take(&mut self.spans_buffer)
                .into_iter()
                .map(|_| -> (usize, &str, &TextFont, FontFaceInfo, Color) { unreachable!() })
                .collect();

        computed.entities.clear();

        for (span_index, (entity, depth, span, text_font, color)) in text_spans.enumerate() {
            // Save this span entity in the computed text block.
            computed.entities.push(TextEntity { entity, depth });

            if span.is_empty() {
                continue;
            }
            // Return early if a font is not loaded yet.
            if !fonts.contains(text_font.font.id()) {
                spans.clear();
                self.spans_buffer = spans
                    .into_iter()
                    .map(
                        |_| -> (usize, &'static str, &'static TextFont, FontFaceInfo) {
                            unreachable!()
                        },
                    )
                    .collect();

                return Err(TextError::NoSuchFont);
            }

            // Get max font size for use in cosmic Metrics.
            max_font_size = max_font_size.max(text_font.font_size);
            max_line_height = max_line_height.max(text_font.line_height.eval(text_font.font_size));

            // Load Bevy fonts into cosmic-text's font system.
            let face_info = load_font_to_fontdb(
                text_font,
                font_system,
                &mut self.map_handle_to_font_id,
                fonts,
            );

            // Save spans that aren't zero-sized.
            if scale_factor <= 0.0 || text_font.font_size <= 0.0 {
                once!(warn!(
                    "Text span {entity} has a font size <= 0.0. Nothing will be displayed.",
                ));

                continue;
            }
            spans.push((span_index, span, text_font, face_info, color));
        }

        let mut metrics = Metrics::new(max_font_size, max_line_height).scale(scale_factor as f32);
        // Metrics of 0.0 cause `Buffer::set_metrics` to panic. We hack around this by 'falling
        // through' to call `Buffer::set_rich_text` with zero spans so any cached text will be cleared without
        // deallocating the buffer.
        metrics.font_size = metrics.font_size.max(0.000001);
        metrics.line_height = metrics.line_height.max(0.000001);

        // Map text sections to cosmic-text spans, and ignore sections with negative or zero fontsizes,
        // since they cannot be rendered by cosmic-text.
        //
        // The section index is stored in the metadata of the spans, and could be used
        // to look up the section the span came from and is not used internally
        // in cosmic-text.
        let spans_iter = spans
            .iter()
            .map(|(span_index, span, text_font, font_info, color)| {
                (
                    *span,
                    get_attrs(*span_index, text_font, *color, font_info, scale_factor),
                )
            });

        // Update the buffer.
        let buffer = &mut computed.buffer;
        buffer.set_metrics_and_size(font_system, metrics, bounds.width, bounds.height);

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
            spans_iter,
            &Attrs::new(),
            Shaping::Advanced,
            Some(justify.into()),
        );

        buffer.shape_until_scroll(font_system, false);

        // Workaround for alignment not working for unbounded text.
        // See https://github.com/pop-os/cosmic-text/issues/343
        if bounds.width.is_none() && justify != Justify::Left {
            let dimensions = buffer_dimensions(buffer);
            // `set_size` causes a re-layout to occur.
            buffer.set_size(font_system, Some(dimensions.x), bounds.height);
        }

        // Recover the spans buffer.
        spans.clear();
        self.spans_buffer = spans
            .into_iter()
            .map(|_| -> (usize, &'static str, &'static TextFont, FontFaceInfo) { unreachable!() })
            .collect();

        Ok(())
    }

    /// Queues text for rendering
    ///
    /// Produces a [`TextLayoutInfo`], containing [`PositionedGlyph`]s
    /// which contain information for rendering the text.
    pub fn queue_text<'a>(
        &mut self,
        layout_info: &mut TextLayoutInfo,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color)>,
        scale_factor: f64,
        layout: &TextLayout,
        bounds: TextBounds,
        font_atlas_sets: &mut FontAtlasSets,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
        swash_cache: &mut SwashCache,
    ) -> Result<(), TextError> {
        layout_info.glyphs.clear();
        layout_info.section_rects.clear();
        layout_info.size = Default::default();

        // Clear this here at the focal point of text rendering to ensure the field's lifecycle has strong boundaries.
        computed.needs_rerender = false;

        // Extract font ids from the iterator while traversing it.
        let mut glyph_info = core::mem::take(&mut self.glyph_info);
        glyph_info.clear();
        let text_spans = text_spans.inspect(|(_, _, _, text_font, _)| {
            glyph_info.push((text_font.font.id(), text_font.font_smoothing));
        });

        let update_result = self.update_buffer(
            fonts,
            text_spans,
            layout.linebreak,
            layout.justify,
            bounds,
            scale_factor,
            computed,
            font_system,
        );
        if let Err(err) = update_result {
            self.glyph_info = glyph_info;
            return Err(err);
        }

        let buffer = &mut computed.buffer;
        let box_size = buffer_dimensions(buffer);

        let result = buffer.layout_runs().try_for_each(|run| {
            let mut current_section: Option<usize> = None;
            let mut start = 0.;
            let mut end = 0.;
            let result = run
                .glyphs
                .iter()
                .map(move |layout_glyph| (layout_glyph, run.line_y, run.line_i))
                .try_for_each(|(layout_glyph, line_y, line_i)| {
                    match current_section {
                        Some(section) => {
                            if section != layout_glyph.metadata {
                                layout_info.section_rects.push((
                                    computed.entities[section].entity,
                                    Rect::new(
                                        start,
                                        run.line_top,
                                        end,
                                        run.line_top + run.line_height,
                                    ),
                                ));
                                start = end.max(layout_glyph.x);
                                current_section = Some(layout_glyph.metadata);
                            }
                            end = layout_glyph.x + layout_glyph.w;
                        }
                        None => {
                            current_section = Some(layout_glyph.metadata);
                            start = layout_glyph.x;
                            end = start + layout_glyph.w;
                        }
                    }

                    let mut temp_glyph;
                    let span_index = layout_glyph.metadata;
                    let font_id = glyph_info[span_index].0;
                    let font_smoothing = glyph_info[span_index].1;

                    let layout_glyph = if font_smoothing == FontSmoothing::None {
                        // If font smoothing is disabled, round the glyph positions and sizes,
                        // effectively discarding all subpixel layout.
                        temp_glyph = layout_glyph.clone();
                        temp_glyph.x = temp_glyph.x.round();
                        temp_glyph.y = temp_glyph.y.round();
                        temp_glyph.w = temp_glyph.w.round();
                        temp_glyph.x_offset = temp_glyph.x_offset.round();
                        temp_glyph.y_offset = temp_glyph.y_offset.round();
                        temp_glyph.line_height_opt = temp_glyph.line_height_opt.map(f32::round);

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
                                texture_atlases,
                                textures,
                                &mut font_system.0,
                                &mut swash_cache.0,
                                layout_glyph,
                                font_smoothing,
                            )
                        })?;

                    let texture_atlas = texture_atlases.get(atlas_info.texture_atlas).unwrap();
                    let location = atlas_info.location;
                    let glyph_rect = texture_atlas.textures[location.glyph_index];
                    let left = location.offset.x as f32;
                    let top = location.offset.y as f32;
                    let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());

                    // offset by half the size because the origin is center
                    let x = glyph_size.x as f32 / 2.0 + left + physical_glyph.x as f32;
                    let y =
                        line_y.round() + physical_glyph.y as f32 - top + glyph_size.y as f32 / 2.0;

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
            if let Some(section) = current_section {
                layout_info.section_rects.push((
                    computed.entities[section].entity,
                    Rect::new(start, run.line_top, end, run.line_top + run.line_height),
                ));
            }

            result
        });

        // Return the scratch vec.
        self.glyph_info = glyph_info;

        // Check result.
        result?;

        layout_info.size = box_size;
        Ok(())
    }

    /// Queues text for measurement
    ///
    /// Produces a [`TextMeasureInfo`] which can be used by a layout system
    /// to measure the text area on demand.
    pub fn create_text_measure<'a>(
        &mut self,
        entity: Entity,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color)>,
        scale_factor: f64,
        layout: &TextLayout,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
    ) -> Result<TextMeasureInfo, TextError> {
        const MIN_WIDTH_CONTENT_BOUNDS: TextBounds = TextBounds::new_horizontal(0.0);

        // Clear this here at the focal point of measured text rendering to ensure the field's lifecycle has
        // strong boundaries.
        computed.needs_rerender = false;

        self.update_buffer(
            fonts,
            text_spans,
            layout.linebreak,
            layout.justify,
            MIN_WIDTH_CONTENT_BOUNDS,
            scale_factor,
            computed,
            font_system,
        )?;

        let buffer = &mut computed.buffer;
        let min_width_content_size = buffer_dimensions(buffer);

        let max_width_content_size = {
            let font_system = &mut font_system.0;
            buffer.set_size(font_system, None, None);
            buffer_dimensions(buffer)
        };

        Ok(TextMeasureInfo {
            min: min_width_content_size,
            max: max_width_content_size,
            entity,
        })
    }

    /// Returns the [`cosmic_text::fontdb::ID`] for a given [`Font`] asset.
    pub fn get_font_id(&self, asset_id: AssetId<Font>) -> Option<cosmic_text::fontdb::ID> {
        self.map_handle_to_font_id
            .get(&asset_id)
            .cloned()
            .map(|(id, _)| id)
    }
}

/// Render information for a corresponding text block.
///
/// Contains scaled glyphs and their size. Generated via [`TextPipeline::queue_text`] when an entity has
/// [`TextLayout`] and [`ComputedTextBlock`] components.
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextLayoutInfo {
    /// The target scale factor for this text layout
    pub scale_factor: f32,
    /// Scaled and positioned glyphs in screenspace
    pub glyphs: Vec<PositionedGlyph>,
    /// Rects bounding the text block's text sections.
    /// A text section spanning more than one line will have multiple bounding rects.
    pub section_rects: Vec<(Entity, Rect)>,
    /// The glyphs resulting size
    pub size: Vec2,
}

/// Size information for a corresponding [`ComputedTextBlock`] component.
///
/// Generated via [`TextPipeline::create_text_measure`].
#[derive(Debug)]
pub struct TextMeasureInfo {
    /// Minimum size for a text area in pixels, to be used when laying out widgets with taffy
    pub min: Vec2,
    /// Maximum size for a text area in pixels, to be used when laying out widgets with taffy
    pub max: Vec2,
    /// The entity that is measured.
    pub entity: Entity,
}

impl TextMeasureInfo {
    /// Computes the size of the text area within the provided bounds.
    pub fn compute_size(
        &mut self,
        bounds: TextBounds,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
    ) -> Vec2 {
        // Note that this arbitrarily adjusts the buffer layout. We assume the buffer is always 'refreshed'
        // whenever a canonical state is required.
        computed
            .buffer
            .set_size(&mut font_system.0, bounds.width, bounds.height);
        buffer_dimensions(&computed.buffer)
    }
}

/// Add the font to the cosmic text's `FontSystem`'s in-memory font database
pub fn load_font_to_fontdb(
    text_font: &TextFont,
    font_system: &mut cosmic_text::FontSystem,
    map_handle_to_font_id: &mut HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, Arc<str>)>,
    fonts: &Assets<Font>,
) -> FontFaceInfo {
    let font_handle = text_font.font.clone();
    let (face_id, family_name) = map_handle_to_font_id
        .entry(font_handle.id())
        .or_insert_with(|| {
            let font = fonts.get(font_handle.id()).expect(
                "Tried getting a font that was not available, probably due to not being loaded yet",
            );
            let data = Arc::clone(&font.data);
            let ids = font_system
                .db_mut()
                .load_font_source(cosmic_text::fontdb::Source::Binary(data));

            // TODO: it is assumed this is the right font face
            let face_id = *ids.last().unwrap();
            let face = font_system.db().face(face_id).unwrap();
            let family_name = Arc::from(face.families[0].0.as_str());

            (face_id, family_name)
        });
    let face = font_system.db().face(*face_id).unwrap();

    FontFaceInfo {
        stretch: face.stretch,
        style: face.style,
        weight: face.weight,
        family_name: family_name.clone(),
    }
}

/// Translates [`TextFont`] to [`Attrs`].
fn get_attrs<'a>(
    span_index: usize,
    text_font: &TextFont,
    color: Color,
    face_info: &'a FontFaceInfo,
    scale_factor: f64,
) -> Attrs<'a> {
    Attrs::new()
        .metadata(span_index)
        .family(Family::Name(&face_info.family_name))
        .stretch(face_info.stretch)
        .style(face_info.style)
        .weight(face_info.weight)
        .metrics(
            Metrics {
                font_size: text_font.font_size,
                line_height: text_font.line_height.eval(text_font.font_size),
            }
            .scale(scale_factor as f32),
        )
        .color(cosmic_text::Color(color.to_linear().as_u32()))
}

/// Calculate the size of the text area for the given buffer.
fn buffer_dimensions(buffer: &Buffer) -> Vec2 {
    let (width, height) = buffer
        .layout_runs()
        .map(|run| (run.line_w, run.line_height))
        .reduce(|(w1, h1), (w2, h2)| (w1.max(w2), h1 + h2))
        .unwrap_or((0.0, 0.0));

    Vec2::new(width, height).ceil()
}

/// Discards stale data cached in `FontSystem`.
pub(crate) fn trim_cosmic_cache(mut font_system: ResMut<CosmicFontSystem>) {
    // A trim age of 2 was found to reduce frame time variance vs age of 1 when tested with dynamic text.
    // See https://github.com/bevyengine/bevy/pull/15037
    //
    // We assume only text updated frequently benefits from the shape cache (e.g. animated text, or
    // text that is dynamically measured for UI).
    font_system.0.shape_run_cache.trim(2);
}
