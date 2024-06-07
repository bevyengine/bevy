use std::sync::{Arc, Mutex};

use bevy_asset::{AssetId, Assets};
use bevy_ecs::{component::Component, reflect::ReflectComponent, system::Resource};
use bevy_math::{UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlasLayout;
use bevy_utils::HashMap;

use cosmic_text::{Attrs, Buffer, Metrics, Shaping, Wrap};

use crate::{
    error::TextError, BreakLineOn, Font, FontAtlasSets, JustifyText, PositionedGlyph, TextSection, YAxisOrientation,
};

/// A wrapper around a [`cosmic_text::FontSystem`]
struct CosmicFontSystem(Arc<Mutex<cosmic_text::FontSystem>>);

impl Default for CosmicFontSystem {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        let db = cosmic_text::fontdb::Database::new();
        // TODO: consider using `cosmic_text::FontSystem::new()` (load system fonts by default)
        Self(Arc::new(Mutex::new(
            cosmic_text::FontSystem::new_with_locale_and_db(locale, db),
        )))
    }
}

/// A wrapper around a [`cosmic_text::SwashCache`]
struct SwashCache(cosmic_text::SwashCache);

impl Default for SwashCache {
    fn default() -> Self {
        Self(cosmic_text::SwashCache::new())
    }
}

/// The `TextPipeline` is used to layout and render [`Text`](crate::Text).
///
/// See the [crate-level documentation](crate) for more information.
#[derive(Default, Resource)]
pub struct TextPipeline {
    /// Identifies a font [`ID`](cosmic_text::fontdb::ID) by its [`Font`] [`Asset`](bevy_asset::Asset).
    map_handle_to_font_id: HashMap<AssetId<Font>, cosmic_text::fontdb::ID>,
    /// The font system is used to retrieve fonts and their information, including glyph outlines.
    ///
    /// See [`cosmic_text::FontSystem`] for more information.
    font_system: CosmicFontSystem,
    /// The swash cache rasterizer is used to rasterize glyphs
    ///
    /// See [`cosmic_text::SwashCache`] for more information.
    swash_cache: SwashCache,
}

impl TextPipeline {
    /// Utilizes [cosmic_text::Buffer] to shape and layout text
    ///
    /// Negative or 0.0 font sizes will not be laid out, and an empty buffer will be returned.
    pub fn create_buffer(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        linebreak_behavior: BreakLineOn,
        bounds: Vec2,
        scale_factor: f64,
    ) -> Result<Buffer, TextError> {
        // TODO: Support multiple section font sizes, pending upstream implementation in cosmic_text
        // For now, just use the first section's size or a default
        let font_size = sections
            .get(0)
            .map(|s| s.style.font_size)
            .unwrap_or_else(|| crate::TextStyle::default().font_size)
            as f64
            * scale_factor;

        // TODO: maybe we would like to render negative fontsizes or scaling upside down or something? for now, no text is rendered
        if font_size <= 0.0 {
            // return empty buffer, making sure that the line height is not zero,
            // since that results in a panic in cosmic-text
            let metrics = Metrics::new(0.0, 0.000001);
            return Ok(Buffer::new_empty(metrics));
        };
        // TODO: Support line height as an option. Unitless `1.2` is the default used in browsers (1.2x font size).
        let line_height = font_size * 1.2;
        let (font_size, line_height) = (font_size as f32, line_height as f32);
        let metrics = Metrics::new(font_size, line_height);

        let font_system = &mut acquire_font_system(&mut self.font_system)?;

        // return early if the fonts are not loaded yet
        for section in sections {
            fonts
                .get(section.style.font.id())
                .ok_or(TextError::NoSuchFont)?;
        }

        let spans: Vec<(&str, Attrs)> = sections
            .iter()
            .enumerate()
            .map(|(section_index, section)| {
                (
                    &section.value[..],
                    get_attrs(
                        section,
                        section_index,
                        font_system,
                        &mut self.map_handle_to_font_id,
                        fonts,
                    ),
                )
            })
            .collect();

        // TODO: cache buffers (see Iced / glyphon)
        let mut buffer = Buffer::new_empty(metrics);
        let buffer_height = f32::INFINITY;
        buffer.set_size(font_system, bounds.x.ceil(), buffer_height);

        buffer.set_wrap(
            font_system,
            match linebreak_behavior {
                BreakLineOn::WordBoundary => Wrap::Word,
                BreakLineOn::AnyCharacter => Wrap::Glyph,
                BreakLineOn::NoWrap => Wrap::None,
            },
        );

        // TODO: other shaping methods?
        let default_attrs = Attrs::new();
        buffer.set_rich_text(font_system, spans, default_attrs, Shaping::Advanced);

        if buffer.visible_lines() == 0 {
            // Presumably the font(s) are not available yet
            return Err(TextError::NoSuchFont);
        }

        Ok(buffer)
    }

    /// Queues text for rendering
    ///
    /// Produces a [`TextLayoutInfo`], containing [`PositionedGlyph`]s
    /// which contain information for rendering the text.
    #[allow(clippy::too_many_arguments)]
    pub fn queue_text(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        // TODO: Implement text alignment properly
        text_alignment: JustifyText,
        linebreak_behavior: BreakLineOn,
        bounds: Vec2,
        font_atlas_sets: &mut FontAtlasSets,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        y_axis_orientation: YAxisOrientation,
    ) -> Result<TextLayoutInfo, TextError> {
        if sections.is_empty() {
            return Ok(TextLayoutInfo::default());
        }

        let buffer =
            self.create_buffer(fonts, sections, linebreak_behavior, bounds, scale_factor)?;

        let box_size = buffer_dimensions(&buffer);
        let h_limit = if bounds.x.is_finite() {
            bounds.x
        } else {
            box_size.x
        };

        let h_anchor = match text_alignment {
            JustifyText::Left => 0.0,
            JustifyText::Center => h_limit * 0.5,
            JustifyText::Right => h_limit * 1.0,
        }
        .floor();
        let font_system = &mut acquire_font_system(&mut self.font_system)?;
        let swash_cache = &mut self.swash_cache.0;

        let glyphs = buffer
            .layout_runs()
            .flat_map(|run| {
                run.glyphs
                    .iter()
                    .map(move |layout_glyph| (layout_glyph, run.line_w, run.line_y))
            })
            .map(|(layout_glyph, line_w, line_y)| {
                let section_index = layout_glyph.metadata;

                let font_handle = sections[section_index].style.font.clone_weak();
                let font_atlas_set = font_atlas_sets.sets.entry(font_handle.id()).or_default();

                let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                let atlas_info = font_atlas_set
                    .get_glyph_atlas_info(physical_glyph.cache_key)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        font_atlas_set.add_glyph_to_atlas(
                            texture_atlases,
                            textures,
                            font_system,
                            swash_cache,
                            layout_glyph,
                        )
                    })?;

                let texture_atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();
                let location = atlas_info.location;
                let glyph_rect = texture_atlas.textures[location.glyph_index];
                let left = location.offset.x as f32;
                let top = location.offset.y as f32;
                let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());

                // offset by half the size because the origin is center
                let x = glyph_size.x as f32 / 2.0 + left + physical_glyph.x as f32;
                let y = line_y + physical_glyph.y as f32 - top + glyph_size.y as f32 / 2.0;
                // TODO: use cosmic text's implementation (per-BufferLine alignment) as it will be editor aware
                // see https://github.com/pop-os/cosmic-text/issues/130 (currently bugged)
                let x = x + match text_alignment {
                    JustifyText::Left => 0.0,
                    JustifyText::Center => (box_size.x - line_w) / 2.0,
                    JustifyText::Right => box_size.x - line_w,
                };
                let y = match y_axis_orientation {
                    YAxisOrientation::TopToBottom => y,
                    YAxisOrientation::BottomToTop => box_size.y - y,
                };

                // TODO: confirm whether we need to offset by glyph baseline
                // (this should be testable with a single line of text with
                // fonts of different sizes and/or baselines)

                let position = Vec2::new(x, y);

                // TODO: recreate the byte index, that keeps track of where a cursor is,
                // when glyphs are not limited to single byte representation, relevant for #1319
                let pos_glyph =
                    PositionedGlyph::new(position, glyph_size.as_vec2(), atlas_info, section_index);
                Ok(pos_glyph)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TextLayoutInfo {
            glyphs,
            size: box_size,
        })
    }

    /// Queues text for measurement
    ///
    /// Produces a [`TextMeasureInfo`] which can be used by a layout system
    /// to measure the text area on demand.
    pub fn create_text_measure(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        // TODO: not currently required
        _text_alignment: JustifyText,
        linebreak_behavior: BreakLineOn,
    ) -> Result<TextMeasureInfo, TextError> {
        const MIN_WIDTH_CONTENT_BOUNDS: Vec2 = Vec2::new(0.0, f32::INFINITY);
        const MAX_WIDTH_CONTENT_BOUNDS: Vec2 = Vec2::new(f32::INFINITY, f32::INFINITY);

        let mut buffer = self.create_buffer(
            fonts,
            sections,
            linebreak_behavior,
            MIN_WIDTH_CONTENT_BOUNDS,
            scale_factor,
        )?;

        let min_width_content_size = buffer_dimensions(&buffer);

        let max_width_content_size = {
            let font_system = &mut acquire_font_system(&mut self.font_system)?;

            buffer.set_size(
                font_system,
                MAX_WIDTH_CONTENT_BOUNDS.x,
                MAX_WIDTH_CONTENT_BOUNDS.y,
            );

            buffer_dimensions(&buffer)
        };

        Ok(TextMeasureInfo {
            min: min_width_content_size,
            max: max_width_content_size,
            font_system: Arc::clone(&self.font_system.0),
            buffer: Mutex::new(buffer),
        })
    }
}

/// Render information for a corresponding [`Text`](crate::Text) component.
///
/// Contains scaled glyphs and their size. Generated via [`TextPipeline::queue_text`].
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct TextLayoutInfo {
    pub glyphs: Vec<PositionedGlyph>,
    pub size: Vec2,
}

// TODO: is there a way to do this without mutexes?
/// Size information for a corresponding [`Text`](crate::Text) component.
///
/// Generated via [`TextPipeline::create_text_measure`].
pub struct TextMeasureInfo {
    pub min: Vec2,
    pub max: Vec2,
    buffer: Mutex<cosmic_text::Buffer>,
    font_system: Arc<Mutex<cosmic_text::FontSystem>>,
}

impl std::fmt::Debug for TextMeasureInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextMeasureInfo")
            .field("min", &self.min)
            .field("max", &self.max)
            .field("buffer", &"_")
            .field("font_system", &"_")
            .finish()
    }
}

impl TextMeasureInfo {
    pub fn compute_size(&self, bounds: Vec2) -> Vec2 {
        let font_system = &mut self.font_system.try_lock().expect("Failed to acquire lock");
        let mut buffer = self.buffer.lock().expect("Failed to acquire the lock");
        buffer.set_size(font_system, bounds.x.ceil(), bounds.y.ceil());
        buffer_dimensions(&buffer)
    }
}

/// get attr for from textstyle
/// loading fonts into the [`Database`](cosmic_text::fontdb::Database) if required.
fn get_attrs<'a>(
    section: &'a TextSection,
    section_index: usize,
    font_system: &mut cosmic_text::FontSystem,
    map_handle_to_font_id: &mut HashMap<AssetId<Font>, cosmic_text::fontdb::ID>,
    fonts: &Assets<Font>,
) -> Attrs<'a> {
    let font_handle = section.style.font.clone();
    let face_id = map_handle_to_font_id
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
            *ids.last().unwrap()

            // TODO: below may be required if we need to offset by the baseline (TBC)
            // see https://github.com/pop-os/cosmic-text/issues/123
            // let font = font_system.get_font(face_id).unwrap();
            // map_font_id_to_metrics
            //     .entry(face_id)
            //     .or_insert_with(|| font.as_swash().metrics(&[]));
        });
    let face = font_system.db().face(*face_id).unwrap();
    // TODO: validate this is the correct string to extract
    // let family_name = &face.families[0].0;
    let attrs = Attrs::new()
        // TODO: validate that we can use metadata
        .metadata(section_index)
        // TODO: this reference, becomes owned by the font system, which is not really wanted...
        // .family(Family::Name(family_name))
        .stretch(face.stretch)
        .style(face.style)
        .weight(face.weight)
        .color(cosmic_text::Color(section.style.color.linear().as_u32()));
    attrs
}

/// Calculate the size of the text area for the given buffer.
fn buffer_dimensions(buffer: &Buffer) -> Vec2 {
    // TODO: see https://github.com/pop-os/cosmic-text/issues/70 Let a Buffer figure out its height during set_size
    // TODO: see https://github.com/pop-os/cosmic-text/issues/42 Request: Allow buffer dimensions to be undefined
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .reduce(|max_w, w| max_w.max(w))
        .unwrap_or_else(|| 0.0);
    // TODO: support multiple line heights / font sizes (once supported by cosmic text), see https://github.com/pop-os/cosmic-text/issues/64
    let line_height = buffer.metrics().line_height.ceil();
    let height = buffer.layout_runs().count() as f32 * line_height;

    // `width.ceil() + 0.001` gets around a rare text layout bug in the tonemapping example.
    // See https://github.com/pop-os/cosmic-text/issues/134
    Vec2::new(width.ceil() + 0.001, height).ceil()
}

/// An iterator over the paragraphs in the input text.
/// It is equivalent to [`core::str::Lines`] but follows [`unicode_bidi`] behavior.
// TODO: upstream to cosmic_text, see https://github.com/pop-os/cosmic-text/pull/124
// TODO: create separate iterator that keeps the ranges, or simply use memory address introspection (as_ptr())
// TODO: this breaks for lines ending in newlines, e.g. "foo\n" should split into ["foo", ""] but we actually get ["foo"]
pub struct BidiParagraphs<'text> {
    text: &'text str,
    info: std::vec::IntoIter<unicode_bidi::ParagraphInfo>,
}

impl<'text> BidiParagraphs<'text> {
    /// Create an iterator to split the input text into paragraphs
    /// in accordance with [`unicode_bidi`] behavior.
    pub fn new(text: &'text str) -> Self {
        let info = unicode_bidi::BidiInfo::new(text, None);
        let info = info.paragraphs.into_iter();
        Self { text, info }
    }
}

impl<'text> Iterator for BidiParagraphs<'text> {
    type Item = &'text str;

    fn next(&mut self) -> Option<Self::Item> {
        let para = self.info.next()?;
        let paragraph = &self.text[para.range];
        // `para.range` includes the newline that splits the line, so remove it if present
        let mut char_indices = paragraph.char_indices();
        if let Some(i) = char_indices.next_back().and_then(|(i, c)| {
            // `BidiClass::B` is a Paragraph_Separator (various newline characters)
            (unicode_bidi::BidiClass::B == unicode_bidi::bidi_class(c)).then_some(i)
        }) {
            Some(&paragraph[0..i])
        } else {
            Some(paragraph)
        }
    }
}

/// Helper method to acquire a font system mutex.
#[inline(always)]
fn acquire_font_system(
    font_system: &mut CosmicFontSystem,
) -> Result<std::sync::MutexGuard<'_, cosmic_text::FontSystem>, TextError> {
    font_system
        .0
        .try_lock()
        .map_err(|_| TextError::FailedToAcquireMutex)
}
