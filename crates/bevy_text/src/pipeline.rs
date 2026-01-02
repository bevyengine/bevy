use alloc::sync::Arc;

use bevy_asset::{AssetId, Assets};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Query, ResMut},
};
use bevy_image::prelude::*;
use bevy_log::{once, warn};
use bevy_math::{Rect, UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use crate::{
    add_glyph_to_atlas, error::TextError, get_glyph_atlas_info, ComputedTextBlock, Font,
    FontAtlasKey, FontAtlasSet, FontHinting, FontSmoothing, FontSource, Justify, LineBreak,
    LineHeight, PositionedGlyph, TextBounds, TextEntity, TextFont, TextLayout,
};
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};

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
    spans_buffer: Vec<(
        usize,
        &'static str,
        &'static TextFont,
        FontFaceInfo,
        LineHeight,
    )>,
    /// Buffered vec for collecting info for glyph assembly.
    glyph_info: Vec<(AssetId<Font>, FontSmoothing, f32, f32, f32, f32, u16)>,
}

impl TextPipeline {
    /// Utilizes [`cosmic_text::Buffer`] to shape and layout text
    ///
    /// Negative or 0.0 font sizes will not be laid out.
    pub fn update_buffer<'a>(
        &mut self,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color, LineHeight)>,
        linebreak: LineBreak,
        justify: Justify,
        bounds: TextBounds,
        scale_factor: f64,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
        hinting: FontHinting,
    ) -> Result<(), TextError> {
        computed.needs_rerender = false;

        let font_system = &mut font_system.0;

        // Collect span information into a vec. This is necessary because font loading requires mut access
        // to FontSystem, which the cosmic-text Buffer also needs.
        let mut spans: Vec<(usize, &str, &TextFont, FontFaceInfo, Color, LineHeight)> =
            core::mem::take(&mut self.spans_buffer)
                .into_iter()
                .map(
                    |_| -> (usize, &str, &TextFont, FontFaceInfo, Color, LineHeight) {
                        unreachable!()
                    },
                )
                .collect();

        computed.entities.clear();

        for (span_index, (entity, depth, span, text_font, color, line_height)) in
            text_spans.enumerate()
        {
            // Save this span entity in the computed text block.
            computed.entities.push(TextEntity { entity, depth });

            if span.is_empty() {
                continue;
            }

            let family_name = match &text_font.font {
                FontSource::Handle(handle) => {
                    if let Some(font) = fonts.get(handle.id()) {
                        let data = Arc::clone(&font.data);
                        let ids = font_system
                            .db_mut()
                            .load_font_source(cosmic_text::fontdb::Source::Binary(data));

                        // TODO: it is assumed this is the right font face
                        let face_id = *ids.last().unwrap();
                        let face = font_system.db().face(face_id).unwrap();
                        Arc::from(face.families[0].0.as_str())
                    } else {
                        // Return early if a font is not loaded yet.
                        spans.clear();
                        self.spans_buffer = spans
                            .into_iter()
                            .map(
                                |_| -> (
                                    usize,
                                    &'static str,
                                    &'static TextFont,
                                    FontFaceInfo,
                                    LineHeight,
                                ) { unreachable!() },
                            )
                            .collect();
                        return Err(TextError::NoSuchFont);
                    }
                }
                FontSource::Family(family) => Arc::from(family.as_str()),
            };

            let face_info = FontFaceInfo { family_name };

            // Save spans that aren't zero-sized.
            if scale_factor <= 0.0 || text_font.font_size <= 0.0 {
                once!(warn!(
                    "Text span {entity} has a font size <= 0.0. Nothing will be displayed.",
                ));

                continue;
            }
            spans.push((span_index, span, text_font, face_info, color, line_height));
        }

        // Map text sections to cosmic-text spans, and ignore sections with negative or zero fontsizes,
        // since they cannot be rendered by cosmic-text.
        //
        // The section index is stored in the metadata of the spans, and could be used
        // to look up the section the span came from and is not used internally
        // in cosmic-text.
        let spans_iter = spans.iter().map(
            |(span_index, span, text_font, font_info, color, line_height)| {
                (
                    *span,
                    get_attrs(
                        *span_index,
                        text_font,
                        *line_height,
                        *color,
                        font_info,
                        scale_factor,
                    ),
                )
            },
        );

        // Update the buffer.
        let buffer = &mut computed.buffer;

        // Set the metrics hinting strategy
        buffer.set_hinting(font_system, hinting.into());

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

        // Workaround for alignment not working for unbounded text.
        // See https://github.com/pop-os/cosmic-text/issues/343
        let width = (bounds.width.is_none() && justify != Justify::Left)
            .then(|| buffer_dimensions(buffer).x)
            .or(bounds.width);
        buffer.set_size(font_system, width, bounds.height);

        // Recover the spans buffer.
        spans.clear();
        self.spans_buffer = spans
            .into_iter()
            .map(
                |_| -> (
                    usize,
                    &'static str,
                    &'static TextFont,
                    FontFaceInfo,
                    LineHeight,
                ) { unreachable!() },
            )
            .collect();

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
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color, LineHeight)>,
        scale_factor: f64,
        layout: &TextLayout,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
        hinting: FontHinting,
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
            hinting,
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

    /// Update [`TextLayoutInfo`] with the new [`PositionedGlyph`] layout.
    pub fn update_text_layout_info<'a>(
        &mut self,
        layout_info: &mut TextLayoutInfo,
        text_font_query: Query<&'a TextFont>,
        scale_factor: f64,
        font_atlas_set: &mut FontAtlasSet,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
        swash_cache: &mut SwashCache,
        bounds: TextBounds,
        justify: Justify,
    ) -> Result<(), TextError> {
        computed.needs_rerender = false;

        layout_info.glyphs.clear();
        layout_info.run_geometry.clear();
        layout_info.size = Default::default();

        self.glyph_info.clear();

        for text_font in text_font_query.iter_many(computed.entities.iter().map(|e| e.entity)) {
            let mut section_info = (
                AssetId::default(),
                text_font.font_smoothing,
                text_font.font_size,
                0.0,
                0.0,
                0.0,
                text_font.weight.clamp().0,
            );

            if let FontSource::Handle(handle) = &text_font.font
                && let Some((id, _)) = self.map_handle_to_font_id.get(&handle.id())
                && let Some(font) = font_system.get_font(*id, cosmic_text::Weight(section_info.6))
            {
                let swash = font.as_swash();
                let metrics = swash.metrics(&[]);
                let upem = metrics.units_per_em as f32;
                let scalar = section_info.2 * scale_factor as f32 / upem;
                section_info.3 = (metrics.strikeout_offset * scalar).round();
                section_info.4 = (metrics.stroke_size * scalar).round().max(1.);
                section_info.5 = (metrics.underline_offset * scalar).round();
            }
            self.glyph_info.push(section_info);
        }

        let buffer = &mut computed.buffer;

        // Workaround for alignment not working for unbounded text.
        // See https://github.com/pop-os/cosmic-text/issues/343
        let width = (bounds.width.is_none() && justify != Justify::Left)
            .then(|| buffer_dimensions(buffer).x)
            .or(bounds.width);
        buffer.set_size(font_system, width, bounds.height);
        let mut box_size = Vec2::ZERO;

        let result = buffer.layout_runs().try_for_each(|run| {
            box_size.x = box_size.x.max(run.line_w);
            box_size.y += run.line_height;
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
                                layout_info.run_geometry.push(RunGeometry {
                                    span_index: section,
                                    bounds: Rect::new(
                                        start,
                                        run.line_top,
                                        end,
                                        run.line_top + run.line_height,
                                    ),
                                    strikethrough_y: (run.line_y - self.glyph_info[section].3)
                                        .round(),
                                    strikethrough_thickness: self.glyph_info[section].4,
                                    underline_y: (run.line_y - self.glyph_info[section].5).round(),
                                    underline_thickness: self.glyph_info[section].4,
                                });
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
                    let font_smoothing = self.glyph_info[span_index].1;

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

                    let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                    let font_atlases = font_atlas_set
                        .entry(FontAtlasKey {
                            id: physical_glyph.cache_key.font_id,
                            font_size_bits: physical_glyph.cache_key.font_size_bits,
                            font_smoothing,
                        })
                        .or_default();

                    let atlas_info = get_glyph_atlas_info(font_atlases, physical_glyph.cache_key)
                        .map(Ok)
                        .unwrap_or_else(|| {
                            add_glyph_to_atlas(
                                font_atlases,
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
                layout_info.run_geometry.push(RunGeometry {
                    span_index: section,
                    bounds: Rect::new(start, run.line_top, end, run.line_top + run.line_height),
                    strikethrough_y: (run.line_y - self.glyph_info[section].3).round(),
                    strikethrough_thickness: self.glyph_info[section].4,
                    underline_y: (run.line_y - self.glyph_info[section].5).round(),
                    underline_thickness: self.glyph_info[section].4,
                });
            }

            result
        });

        // Check result.
        result?;

        layout_info.size = box_size.ceil();
        Ok(())
    }
}

/// Render information for a corresponding text block.
///
/// Contains scaled glyphs and their size. Generated via [`TextPipeline::update_text_layout_info`] when an entity has
/// [`TextLayout`] and [`ComputedTextBlock`] components.
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextLayoutInfo {
    /// The target scale factor for this text layout
    pub scale_factor: f32,
    /// Scaled and positioned glyphs in screenspace
    pub glyphs: Vec<PositionedGlyph>,
    /// Geometry of each text run used to render text decorations like background colors, strikethrough, and underline.
    /// A run in `bevy_text` is a contiguous sequence of glyphs on a line that share the same text attributes like font,
    /// font size, and line height. A text entity that extends over multiple lines will have multiple corresponding runs.
    ///
    /// The coordinates are unscaled and relative to the top left corner of the text layout.
    pub run_geometry: Vec<RunGeometry>,
    /// The glyphs resulting size
    pub size: Vec2,
}

impl TextLayoutInfo {
    /// Clear the layout, retaining capacity
    pub fn clear(&mut self) {
        self.scale_factor = 1.;
        self.glyphs.clear();
        self.run_geometry.clear();
        self.size = Vec2::ZERO;
    }
}

/// Geometry of a text run used to render text decorations like background colors, strikethrough, and underline.
/// A run in `bevy_text` is a contiguous sequence of glyphs on a line that share the same text attributes like font,
/// font size, and line height.
#[derive(Default, Debug, Clone, Reflect)]
pub struct RunGeometry {
    /// The index of the text entity in [`ComputedTextBlock`] that this run belongs to.
    pub span_index: usize,
    /// Bounding box around the text run
    pub bounds: Rect,
    /// Y position of the strikethrough in the text layout.
    pub strikethrough_y: f32,
    /// Strikethrough stroke thickness.
    pub strikethrough_thickness: f32,
    /// Y position of the underline  in the text layout.
    pub underline_y: f32,
    /// Underline stroke thickness.
    pub underline_thickness: f32,
}

impl RunGeometry {
    /// Returns the center of the strikethrough in the text layout.
    pub fn strikethrough_position(&self) -> Vec2 {
        Vec2::new(
            self.bounds.center().x,
            self.strikethrough_y + 0.5 * self.strikethrough_thickness,
        )
    }

    /// Returns the size of the strikethrough.
    pub fn strikethrough_size(&self) -> Vec2 {
        Vec2::new(self.bounds.size().x, self.strikethrough_thickness)
    }

    /// Get the center of the underline in the text layout.
    pub fn underline_position(&self) -> Vec2 {
        Vec2::new(
            self.bounds.center().x,
            self.underline_y + 0.5 * self.underline_thickness,
        )
    }

    /// Returns the size of the underline.
    pub fn underline_size(&self) -> Vec2 {
        Vec2::new(self.bounds.size().x, self.underline_thickness)
    }
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

/// Translates [`TextFont`] to [`Attrs`].
fn get_attrs<'a>(
    span_index: usize,
    text_font: &TextFont,
    line_height: LineHeight,
    color: Color,
    face_info: &'a FontFaceInfo,
    scale_factor: f64,
) -> Attrs<'a> {
    Attrs::new()
        .metadata(span_index)
        .family(Family::Name(&face_info.family_name))
        .stretch(text_font.width.into())
        .style(text_font.style.into())
        .weight(text_font.weight.into())
        .metrics(
            Metrics {
                font_size: text_font.font_size,
                line_height: line_height.eval(text_font.font_size),
            }
            .scale(scale_factor as f32),
        )
        .font_features((&text_font.font_features).into())
        .color(cosmic_text::Color(color.to_linear().as_u32()))
}

/// Calculate the size of the text area for the given buffer.
fn buffer_dimensions(buffer: &Buffer) -> Vec2 {
    let mut size = Vec2::ZERO;
    for run in buffer.layout_runs() {
        size.x = size.x.max(run.line_w);
        size.y += run.line_height;
    }
    size.ceil()
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
