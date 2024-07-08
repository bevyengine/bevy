use std::sync::Arc;

use bevy_asset::{AssetId, Assets};
use bevy_ecs::{component::Component, reflect::ReflectComponent, system::Resource};
use bevy_math::{UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlasLayout;
use bevy_utils::HashMap;

use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};

use crate::{
    error::TextError, BreakLineOn, CosmicBuffer, Font, FontAtlasSets, JustifyText, PositionedGlyph,
    TextBounds, TextSection, YAxisOrientation,
};

/// A wrapper around a [`cosmic_text::FontSystem`]
struct CosmicFontSystem(cosmic_text::FontSystem);

impl Default for CosmicFontSystem {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        let db = cosmic_text::fontdb::Database::new();
        // TODO: consider using `cosmic_text::FontSystem::new()` (load system fonts by default)
        Self(cosmic_text::FontSystem::new_with_locale_and_db(locale, db))
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
    map_handle_to_font_id: HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, String)>,
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
    /// Utilizes [`cosmic_text::Buffer`] to shape and layout text
    ///
    /// Negative or 0.0 font sizes will not be laid out.
    #[allow(clippy::too_many_arguments)]
    pub fn update_buffer(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        linebreak_behavior: BreakLineOn,
        bounds: TextBounds,
        scale_factor: f64,
        buffer: &mut CosmicBuffer,
        alignment: JustifyText,
    ) -> Result<(), TextError> {
        let font_system = &mut self.font_system.0;

        // return early if the fonts are not loaded yet
        let mut font_size = 0.;
        for section in sections {
            if section.style.font_size > font_size {
                font_size = section.style.font_size;
            }
            fonts
                .get(section.style.font.id())
                .ok_or(TextError::NoSuchFont)?;
        }
        let line_height = font_size * 1.2;
        let metrics = Metrics::new(font_size, line_height).scale(scale_factor as f32);

        // Load Bevy fonts into cosmic-text's font system.
        // This is done as as separate pre-pass to avoid borrow checker issues
        for section in sections.iter() {
            load_font_to_fontdb(section, font_system, &mut self.map_handle_to_font_id, fonts);
        }

        // Map text sections to cosmic-text spans, and ignore sections with negative or zero fontsizes,
        // since they cannot be rendered by cosmic-text.
        //
        // The section index is stored in the metadata of the spans, and could be used
        // to look up the section the span came from and is not used internally
        // in cosmic-text.
        let spans: Vec<(&str, Attrs)> = sections
            .iter()
            .enumerate()
            .filter(|(_section_index, section)| section.style.font_size > 0.0)
            .map(|(section_index, section)| {
                (
                    &section.value[..],
                    get_attrs(
                        section,
                        section_index,
                        font_system,
                        &self.map_handle_to_font_id,
                        scale_factor,
                    ),
                )
            })
            .collect();

        buffer.set_metrics(font_system, metrics);
        buffer.set_size(font_system, bounds.width, bounds.height);

        buffer.set_wrap(
            font_system,
            match linebreak_behavior {
                BreakLineOn::WordBoundary => Wrap::Word,
                BreakLineOn::AnyCharacter => Wrap::Glyph,
                BreakLineOn::WordOrCharacter => Wrap::WordOrGlyph,
                BreakLineOn::NoWrap => Wrap::None,
            },
        );

        buffer.set_rich_text(font_system, spans, Attrs::new(), Shaping::Advanced);

        // PERF: https://github.com/pop-os/cosmic-text/issues/166:
        // Setting alignment afterwards appears to invalidate some layouting performed by `set_text` which is presumably not free?
        for buffer_line in buffer.lines.iter_mut() {
            buffer_line.set_align(Some(alignment.into()));
        }
        buffer.shape_until_scroll(font_system, false);

        Ok(())
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
        text_alignment: JustifyText,
        linebreak_behavior: BreakLineOn,
        bounds: TextBounds,
        font_atlas_sets: &mut FontAtlasSets,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        y_axis_orientation: YAxisOrientation,
        buffer: &mut CosmicBuffer,
    ) -> Result<TextLayoutInfo, TextError> {
        if sections.is_empty() {
            return Ok(TextLayoutInfo::default());
        }

        self.update_buffer(
            fonts,
            sections,
            linebreak_behavior,
            bounds,
            scale_factor,
            buffer,
            text_alignment,
        )?;

        let box_size = buffer_dimensions(buffer);
        let font_system = &mut self.font_system.0;
        let swash_cache = &mut self.swash_cache.0;

        let glyphs = buffer
            .layout_runs()
            .flat_map(|run| {
                run.glyphs
                    .iter()
                    .map(move |layout_glyph| (layout_glyph, run.line_y))
            })
            .map(|(layout_glyph, line_y)| {
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
                let y = line_y.round() + physical_glyph.y as f32 - top + glyph_size.y as f32 / 2.0;
                let y = match y_axis_orientation {
                    YAxisOrientation::TopToBottom => y,
                    YAxisOrientation::BottomToTop => box_size.y - y,
                };

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
        linebreak_behavior: BreakLineOn,
        buffer: &mut CosmicBuffer,
        text_alignment: JustifyText,
    ) -> Result<TextMeasureInfo, TextError> {
        const MIN_WIDTH_CONTENT_BOUNDS: TextBounds = TextBounds::new_horizontal(0.0);

        self.update_buffer(
            fonts,
            sections,
            linebreak_behavior,
            MIN_WIDTH_CONTENT_BOUNDS,
            scale_factor,
            buffer,
            text_alignment,
        )?;

        let min_width_content_size = buffer_dimensions(buffer);

        let max_width_content_size = {
            let font_system = &mut self.font_system.0;
            buffer.set_size(font_system, None, None);
            buffer_dimensions(buffer)
        };

        Ok(TextMeasureInfo {
            min: min_width_content_size,
            max: max_width_content_size,
            // TODO: This clone feels wasteful, is there another way to structure TextMeasureInfo
            // that it doesn't need to own a buffer? - bytemunch
            buffer: buffer.0.clone(),
        })
    }

    /// Get a mutable reference to the [`cosmic_text::FontSystem`].
    ///
    /// Used internally.
    pub fn font_system_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.font_system.0
    }
}

/// Render information for a corresponding [`Text`](crate::Text) component.
///
/// Contains scaled glyphs and their size. Generated via [`TextPipeline::queue_text`].
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct TextLayoutInfo {
    /// Scaled and positioned glyphs in screenspace
    pub glyphs: Vec<PositionedGlyph>,
    /// The glyphs resulting size
    pub size: Vec2,
}

/// Size information for a corresponding [`Text`](crate::Text) component.
///
/// Generated via [`TextPipeline::create_text_measure`].
pub struct TextMeasureInfo {
    /// Minimum size for a text area in pixels, to be used when laying out widgets with taffy
    pub min: Vec2,
    /// Maximum size for a text area in pixels, to be used when laying out widgets with taffy
    pub max: Vec2,
    buffer: cosmic_text::Buffer,
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
    /// Computes the size of the text area within the provided bounds.
    pub fn compute_size(
        &mut self,
        bounds: TextBounds,
        font_system: &mut cosmic_text::FontSystem,
    ) -> Vec2 {
        self.buffer
            .set_size(font_system, bounds.width, bounds.height);
        buffer_dimensions(&self.buffer)
    }
}

fn load_font_to_fontdb(
    section: &TextSection,
    font_system: &mut cosmic_text::FontSystem,
    map_handle_to_font_id: &mut HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, String)>,
    fonts: &Assets<Font>,
) {
    let font_handle = section.style.font.clone();
    map_handle_to_font_id
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
            let family_name = face.families[0].0.to_owned();

            (face_id, family_name)
        });
}

/// Translates [`TextSection`] to [`Attrs`](cosmic_text::attrs::Attrs),
/// loading fonts into the [`Database`](cosmic_text::fontdb::Database) if required.
fn get_attrs<'a>(
    section: &TextSection,
    section_index: usize,
    font_system: &mut cosmic_text::FontSystem,
    map_handle_to_font_id: &'a HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, String)>,
    scale_factor: f64,
) -> Attrs<'a> {
    let (face_id, family_name) = map_handle_to_font_id
        .get(&section.style.font.id())
        .expect("Already loaded with load_font_to_fontdb");
    let face = font_system.db().face(*face_id).unwrap();

    let attrs = Attrs::new()
        .metadata(section_index)
        .family(Family::Name(family_name))
        .stretch(face.stretch)
        .style(face.style)
        .weight(face.weight)
        .metrics(Metrics::relative(section.style.font_size, 1.2).scale(scale_factor as f32))
        .color(cosmic_text::Color(section.style.color.to_linear().as_u32()));
    attrs
}

/// Calculate the size of the text area for the given buffer.
fn buffer_dimensions(buffer: &Buffer) -> Vec2 {
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .reduce(f32::max)
        .unwrap_or(0.0);
    let line_height = buffer.metrics().line_height.ceil();
    let height = buffer.layout_runs().count() as f32 * line_height;

    Vec2::new(width.ceil(), height).ceil()
}
