use std::sync::{Arc, Mutex};

use bevy_asset::{Assets, Handle, HandleId};
use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::{
    tracing::{error, info, warn},
    HashMap,
};

use cosmic_text::{Attrs, AttrsList, Buffer, BufferLine, Family, Metrics, Wrap};

use crate::{
    error::TextError, BreakLineOn, Font, FontAtlasSet, FontAtlasWarning, FontRef, PositionedGlyph,
    TextAlignment, TextSection, TextSettings, YAxisOrientation,
};

// TODO: cache buffers / store buffers on the entity
// TODO: reconstruct byte indices
// TODO: rescale font sizes in all examples
// TODO: fix any broken examples
// TODO: solve spans with different font sizes, see https://github.com/pop-os/cosmic-text/issues/64
// TODO: (future work) split text entities into section entities
// TODO: (future work) text editing
// TODO: font validation

// TODO: the only reason we need a mutex is due to TextMeasure
// - is there a way to do this without it?
pub struct FontSystem(Arc<Mutex<cosmic_text::FontSystem>>);

impl Default for FontSystem {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        let db = cosmic_text::fontdb::Database::new();
        // TODO: consider using `cosmic_text::FontSystem::new()` (load system fonts by default)
        Self(Arc::new(Mutex::new(
            cosmic_text::FontSystem::new_with_locale_and_db(locale, db),
        )))
    }
}

impl FontSystem {
    fn load_system_fonts(&mut self) {
        match self.0.try_lock() {
            Ok(mut font_system) => {
                font_system.db_mut().load_system_fonts();
            }
            Err(err) => {
                error!("Failed to acquire mutex: {:?}", err);
            }
        };
    }
}

pub struct SwashCache(cosmic_text::SwashCache);

impl Default for SwashCache {
    fn default() -> Self {
        Self(cosmic_text::SwashCache::new())
    }
}

#[derive(Default, Resource)]
pub struct TextPipeline {
    /// Identifies a font ID by its font asset handle
    map_handle_to_font_id: HashMap<HandleId, cosmic_text::fontdb::ID>,
    /// Identifies a font atlas set handle by its font iD
    map_font_id_to_handle: HashMap<cosmic_text::fontdb::ID, Handle<FontAtlasSet>>,
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextPipeline {
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
        // TODO: Support line height as an option. Unitless `1.2` is the default used in browsers (1.2x font size).
        let line_height = font_size * 1.2;
        let (font_size, line_height) = (font_size as f32, line_height as f32);
        let metrics = Metrics::new(font_size, line_height);

        let font_system = &mut self
            .font_system
            .0
            .try_lock()
            .map_err(|_| TextError::FailedToAcquireMutex)?;

        // TODO: cache buffers (see Iced / glyphon)
        let mut buffer = Buffer::new(font_system, metrics);

        buffer.lines.clear();
        let mut attrs_list = AttrsList::new(Attrs::new());
        let mut line_string = String::new();
        // all sections need to be combined and broken up into lines
        // e.g.
        // style0"Lorem ipsum\ndolor sit amet,"
        // style1" consectetur adipiscing\nelit,"
        // style2" sed do eiusmod tempor\nincididunt"
        // style3" ut labore et dolore\nmagna aliqua."
        // becomes:
        // line0: style0"Lorem ipsum"
        // line1: style0"dolor sit amet,"
        //        style1" consectetur adipiscing,"
        // line2: style1"elit,"
        //        style2" sed do eiusmod tempor"
        // line3: style2"incididunt"
        //        style3"ut labore et dolore"
        // line4: style3"magna aliqua."

        // combine all sections into a string
        // as well as metadata that links those sections to that string
        let mut end = 0;
        let (string, sections_data): (String, Vec<_>) = sections
            .iter()
            .enumerate()
            .map(|(section_index, section)| {
                let start = end;
                end += section.value.len();
                (section.value.as_str(), (section, section_index, start..end))
            })
            .unzip();

        let mut sections_iter = sections_data.into_iter();
        let mut maybe_section = sections_iter.next();

        // split the string into lines, as ranges
        let string_start = string.as_ptr() as usize;
        let mut lines_iter = BidiParagraphs::new(&string).map(|line: &str| {
            let start = line.as_ptr() as usize - string_start;
            let end = start + line.len();
            start..end
        });
        let mut maybe_line = lines_iter.next();

        loop {
            let (Some(line_range), Some((section, section_index, section_range))) = (&maybe_line, &maybe_section) else {
                // this is reached only if this text is empty
                break;
            };

            // start..end is the intersection of this line and this section
            let start = line_range.start.max(section_range.start);
            let end = line_range.end.min(section_range.end);
            if start < end {
                let text = &string[start..end];
                add_span(
                    &mut line_string,
                    &mut attrs_list,
                    section,
                    *section_index,
                    text,
                    font_system,
                    &mut self.map_handle_to_font_id,
                    fonts,
                );
            }

            // we know that at the end of a line,
            // section text's end index is always >= line text's end index
            // so if this section ends before this line ends,
            // there is another section in this line.
            // otherwise, we move on to the next line.
            if section_range.end < line_range.end {
                maybe_section = sections_iter.next();
            } else {
                maybe_line = lines_iter.next();
                if maybe_line.is_some() {
                    // finalize this line and start a new line
                    let prev_attrs_list =
                        std::mem::replace(&mut attrs_list, AttrsList::new(Attrs::new()));
                    let prev_line_string = std::mem::take(&mut line_string);
                    buffer
                        .lines
                        .push(BufferLine::new(prev_line_string, prev_attrs_list));
                } else {
                    // finalize the final line
                    buffer.lines.push(BufferLine::new(line_string, attrs_list));
                    break;
                }
            }
        }

        // node size (bounds) is already scaled by the systems that call queue_text
        // TODO: cosmic text does not shape/layout text outside the buffer height
        // consider a better way to do this
        // let buffer_height = bounds.y;
        let buffer_height = f32::INFINITY;
        buffer.set_size(font_system, bounds.x.ceil(), buffer_height);

        buffer.set_wrap(
            font_system,
            match linebreak_behavior {
                BreakLineOn::WordBoundary => Wrap::Word,
                BreakLineOn::AnyCharacter => Wrap::Glyph,
            },
        );

        // TODO: other shaping methods?
        buffer.shape_until_scroll(font_system);

        if buffer.visible_lines() == 0 {
            // Presumably the font(s) are not available yet
            return Err(TextError::NoSuchFont);
        }

        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn queue_text(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        // TODO: Implement text alignment properly
        text_alignment: TextAlignment,
        linebreak_behavior: BreakLineOn,
        bounds: Vec2,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        text_settings: &TextSettings,
        font_atlas_warning: &mut FontAtlasWarning,
        y_axis_orientation: YAxisOrientation,
    ) -> Result<TextLayoutInfo, TextError> {
        if sections.is_empty() {
            return Ok(TextLayoutInfo::default());
        }

        let buffer =
            self.create_buffer(fonts, sections, linebreak_behavior, bounds, scale_factor)?;

        let font_system = &mut self
            .font_system
            .0
            .try_lock()
            .map_err(|_| TextError::FailedToAcquireMutex)?;
        let swash_cache = &mut self.swash_cache.0;

        let box_size = buffer_dimensions(&buffer);

        let glyphs = buffer.layout_runs().flat_map(|run| {
            run.glyphs
                .iter()
                .map(move |layout_glyph| (layout_glyph, run.line_w, run.line_y))
        })
            .map(|(layout_glyph, line_w, line_y)| {
                let section_index = layout_glyph.metadata;

                let font_atlas_set = match sections[section_index].style.font {
                    FontRef::Asset(ref font_handle) => {
                        let handle: Handle<FontAtlasSet> = font_handle.cast_weak();
                        font_atlas_set_storage
                            .get_or_insert_with(handle, FontAtlasSet::default)
                    }
                    FontRef::Query(ref query) => {
                        // get the id from the database
                        // TODO: error handling
                        // TODO: font may not yet be available, but may be available in future
                        let font_id = font_system.get_font_matches(cosmic_text::Attrs {
                            color_opt: None,
                            family: query.family.as_family(),
                            stretch: query.stretch,
                            style: query.style,
                            weight: query.weight,
                            metadata: 0,
                        })[0];
                        let handle = self
                            .map_font_id_to_handle
                            .entry(font_id)
                            .or_insert_with(|| font_atlas_set_storage.add(FontAtlasSet::default()));
                        font_atlas_set_storage.get_mut(handle).unwrap()
                    }
                };

                let atlas_info = font_atlas_set
                    .get_glyph_atlas_info(layout_glyph.cache_key)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        font_atlas_set.add_glyph_to_atlas(texture_atlases, textures, font_system, swash_cache, layout_glyph)
                    })?;

                if !text_settings.allow_dynamic_font_size
                    && !font_atlas_warning.warned
                    && font_atlas_set.num_font_atlases() > text_settings.max_font_atlases.get()
                {
                    warn!("warning[B0005]: Number of font atlases has exceeded the maximum of {}. Performance and memory usage may suffer.", text_settings.max_font_atlases.get());
                    font_atlas_warning.warned = true;
                }

                let texture_atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();
                let glyph_rect = texture_atlas.textures[atlas_info.glyph_index];
                let left = atlas_info.left as f32;
                let top = atlas_info.top as f32;
                let glyph_size = Vec2::new(glyph_rect.width(), glyph_rect.height());
                assert_eq!(atlas_info.width as f32, glyph_size.x);
                assert_eq!(atlas_info.height as f32, glyph_size.y);

                // offset by half the size because the origin is center
                let x = glyph_size.x / 2.0 + left + layout_glyph.x_int as f32;
                let y = line_y + layout_glyph.y_int as f32 - top + glyph_size.y / 2.0;
                // TODO: use cosmic text's implementation (per-BufferLine alignment) as it will be editor aware
                // see https://github.com/pop-os/cosmic-text/issues/130 (currently bugged)
                let x = x + match text_alignment {
                    TextAlignment::Left => 0.0,
                    TextAlignment::Center => (box_size.x - line_w) / 2.0,
                    TextAlignment::Right => box_size.x - line_w,
                };
                let y = match y_axis_orientation {
                    YAxisOrientation::TopToBottom => y,
                    YAxisOrientation::BottomToTop => box_size.y - y,
                };

                // TODO: confirm whether we need to offset by glyph baseline
                // (this should be testable with a single line of text with
                // fonts of different sizes and/or baselines)

                let position = Vec2::new(x, y);

                let pos_glyph = PositionedGlyph {
                    position,
                    size: glyph_size,
                    atlas_info,
                    section_index,
                    // TODO: recreate the byte index, relevant for #1319
                    // alternatively, reimplement cosmic-text's own hit tests for text
                    byte_index: 0,
                };
                Ok(pos_glyph)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TextLayoutInfo {
            glyphs,
            size: box_size,
        })
    }

    pub fn create_text_measure(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        // TODO: not currently required
        _text_alignment: TextAlignment,
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
            let font_system = &mut self
                .font_system
                .0
                .try_lock()
                .map_err(|_| TextError::FailedToAcquireMutex)?;

            buffer.set_size(
                font_system,
                MAX_WIDTH_CONTENT_BOUNDS.x,
                MAX_WIDTH_CONTENT_BOUNDS.y,
            );

            buffer_dimensions(&buffer)
        };

        Ok(TextMeasureInfo {
            min_width_content_size,
            max_width_content_size,
            font_system: Arc::clone(&self.font_system.0),
            buffer: Mutex::new(buffer),
        })
    }

    /// Attempts to load system fonts.
    ///
    /// Supports Windows, Linux and macOS.
    ///
    /// System fonts loading is a surprisingly complicated task,
    /// mostly unsolvable without interacting with system libraries.
    /// And since `fontdb` tries to be small and portable, this method
    /// will simply scan some predefined directories.
    /// Which means that fonts that are not in those directories must
    /// be added manually.
    ///
    /// This allows access to any installed system fonts
    ///
    /// # Timing
    ///
    /// This function takes some time to run. On the release build, it can take up to a second,
    /// while debug builds can take up to ten times longer. For this reason, it should only be
    /// called once, and the resulting [`FontSystem`] should be shared.
    ///
    /// This should ideally run in a background thread.
    // TODO: This should run in a background thread.
    pub fn load_system_fonts(&mut self) {
        info!("Loading system fonts");
        self.font_system.load_system_fonts();
        info!("Loaded system fonts");
    }
}

/// Render information for a corresponding [`Text`](crate::Text) component.
///
/// Contains scaled glyphs and their size. Generated via [`TextPipeline::queue_text`].
#[derive(Component, Clone, Default, Debug)]
pub struct TextLayoutInfo {
    pub glyphs: Vec<PositionedGlyph>,
    pub size: Vec2,
}

// TODO: is there a way to do this without mutexes?
pub struct TextMeasureInfo {
    pub min_width_content_size: Vec2,
    pub max_width_content_size: Vec2,
    buffer: Mutex<cosmic_text::Buffer>,
    font_system: Arc<Mutex<cosmic_text::FontSystem>>,
}

impl std::fmt::Debug for TextMeasureInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextMeasureInfo")
            .field("min_width_content_size", &self.min_width_content_size)
            .field("max_width_content_size", &self.max_width_content_size)
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

/// Adds a span to the attributes list,
/// loading fonts into the DB if required.
#[allow(clippy::too_many_arguments)]
fn add_span(
    line_string: &mut String,
    attrs_list: &mut AttrsList,
    section: &TextSection,
    section_index: usize,
    text: &str,
    font_system: &mut cosmic_text::FontSystem,
    map_handle_to_font_id: &mut HashMap<HandleId, cosmic_text::fontdb::ID>,
    fonts: &Assets<Font>,
) {
    let start = line_string.len();
    line_string.push_str(text);
    let end = line_string.len();

    let attrs = match section.style.font {
        FontRef::Asset(ref font_handle) => {
            let font_handle_id = font_handle.id();
            let face_id = map_handle_to_font_id
                .entry(font_handle_id)
                .or_insert_with(|| {
                    let font = fonts.get(font_handle).unwrap();
                    let data = Arc::clone(&font.data);
                    font_system
                        .db_mut()
                        .load_font_source(cosmic_text::fontdb::Source::Binary(data));
                    // TODO: it is assumed this is the right font face
                    // see https://github.com/pop-os/cosmic-text/issues/125
                    // fontdb 0.14 returns the font ids from `load_font_source`
                    let face_id = font_system.db().faces().last().unwrap().id;
                    // TODO: below may be required if we need to offset by the baseline (TBC)
                    // see https://github.com/pop-os/cosmic-text/issues/123
                    // let font = font_system.get_font(face_id).unwrap();
                    // map_font_id_to_metrics
                    //     .entry(face_id)
                    //     .or_insert_with(|| font.as_swash().metrics(&[]));
                    face_id
                });
            let face = font_system.db().face(*face_id).unwrap();

            // TODO: validate this is the correct string to extract
            let family_name = &face.families[0].0;
            Attrs::new()
                // TODO: validate that we can use metadata
                .metadata(section_index)
                .family(Family::Name(family_name))
                .stretch(face.stretch)
                .style(face.style)
                .weight(face.weight)
                .color(cosmic_text::Color(section.style.color.as_linear_rgba_u32()))
        }
        FontRef::Query(ref query) => {
            Attrs::new()
                // TODO: validate that we can use metadata
                .metadata(section_index)
                .family(query.family.as_family())
                .stretch(query.stretch)
                .style(query.style)
                .weight(query.weight)
                .color(cosmic_text::Color(section.style.color.as_linear_rgba_u32()))
        }
    };

    attrs_list.add_span(start..end, attrs);
}

fn buffer_dimensions(buffer: &Buffer) -> Vec2 {
    // TODO: see https://github.com/pop-os/cosmic-text/issues/70 Let a Buffer figure out its height during set_size
    // TODO: see https://github.com/pop-os/cosmic-text/issues/42 Request: Allow buffer dimensions to be undefined
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .reduce(|max_w, w| max_w.max(w))
        .unwrap();
    // TODO: support multiple line heights / font sizes (once supported by cosmic text), see https://github.com/pop-os/cosmic-text/issues/64
    let line_height = buffer.metrics().line_height.ceil();
    let height = buffer.layout_runs().count() as f32 * line_height;

    // `width.ceil() + 0.001` gets around a rare text layout bug in the tonemapping example.
    // See https://github.com/pop-os/cosmic-text/issues/134
    Vec2::new(width.ceil() + 0.001, height).ceil()
}

/// An iterator over the paragraphs in the input text.
/// It is equivalent to [`core::str::Lines`] but follows `unicode-bidi` behavior.
// TODO: upstream to cosmic_text, see https://github.com/pop-os/cosmic-text/pull/124
// TODO: create separate iterator that keeps the ranges, or simply use memory address introspection (as_ptr())
// TODO: this breaks for lines ending in newlines, e.g. "foo\n" should split into ["foo", ""] but we actually get ["foo"]
pub struct BidiParagraphs<'text> {
    text: &'text str,
    info: std::vec::IntoIter<unicode_bidi::ParagraphInfo>,
}

impl<'text> BidiParagraphs<'text> {
    /// Create an iterator to split the input text into paragraphs
    /// in accordance with `unicode-bidi` behavior.
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
