use bevy_asset::{AssetEvent, Assets};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    event::EventReader,
    prelude::ReflectComponent,
    system::{Query, Res, ResMut},
    world::Ref,
};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{UVec2, Vec2};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use cosmic_text::{Buffer, Metrics};

use crate::{
    buffer_dimensions, load_font_to_fontdb, CosmicFontSystem, Font, FontAtlasSets, FontSmoothing,
    PositionedGlyph, TextBounds, TextError, TextInputAttributes, TextInputTarget, TextLayoutInfo,
    TextPipeline, DEFAULT_FONT_SIZE, DEFAULT_LINE_HEIGHT,
};

/// Placeholder text displayed when the input is empty.
///
/// Text inputs that contain only whitespace (i.e spaces or tabs) are not empty.
///
/// This is an optional component, intended to work with [`TextInputTarget`].
/// The font and other properties are controlled with [`TextInputAttributes`].
#[derive(Default, Component, Clone, Debug, Reflect, Deref, DerefMut)]
#[reflect(Component, Default, Debug)]
#[require(PlaceholderLayout)]
pub struct Placeholder(pub String);

impl Placeholder {
    /// A new [`Placeholder`] text.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self(prompt.into())
    }
}

/// Layout for the [`Placeholder`] text.
///
/// This is laid out in [`update_placeholder_layouts`].
#[derive(Component)]
pub struct PlaceholderLayout {
    /// A [`Placeholder`] text's cosmic-text buffer (not an Editor as it isn't editable).
    buffer: Buffer,
    /// A [`Placeholder`] text's glyph layout. Displayed when the text input is empty.
    /// Doesn't reuse the editor's [`TextLayoutInfo`] as otherwise the placeholder would need a relayout
    /// everytime it was displayed.
    layout: TextLayoutInfo,
}

impl PlaceholderLayout {
    /// Returns the renderable glyph layout for the associated [`Placeholder`] text
    pub fn layout(&self) -> &TextLayoutInfo {
        &self.layout
    }
}

impl Default for PlaceholderLayout {
    fn default() -> Self {
        Self {
            buffer: Buffer::new_empty(Metrics::new(DEFAULT_FONT_SIZE, DEFAULT_LINE_HEIGHT)),
            layout: Default::default(),
        }
    }
}

/// Generates a new [`PlaceholderLayout`] when a [`Placeholder`]'s text or its target's geometry has changed.
pub fn update_placeholder_layouts(
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut swash_cache: ResMut<crate::pipeline::SwashCache>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_query: Query<(
        Ref<Placeholder>,
        Ref<TextInputAttributes>,
        Ref<TextInputTarget>,
        &mut PlaceholderLayout,
    )>,
    mut font_events: EventReader<AssetEvent<Font>>,
) {
    for (placeholder, attributes, target, mut prompt_layout) in text_query.iter_mut() {
        if !(placeholder.is_changed()
            || attributes.is_changed()
            || target.is_changed()
            || font_events.read().any(|event| match event {
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    *id == attributes.font.id()
                }
                _ => false,
            }))
        {
            continue;
        }
        let PlaceholderLayout { buffer, layout } = prompt_layout.as_mut();

        layout.clear();

        if placeholder.0.is_empty() || target.is_empty() {
            continue;
        }

        if !fonts.contains(attributes.font.id()) {
            continue;
        }

        let line_height = attributes.line_height.eval(attributes.font_size);

        let metrics = Metrics::new(attributes.font_size, line_height).scale(target.scale_factor);

        if metrics.font_size <= 0. || metrics.line_height <= 0. {
            continue;
        }

        let bounds: TextBounds = target.size.into();
        let face_info = load_font_to_fontdb(
            attributes.font.clone(),
            font_system.as_mut(),
            &mut text_pipeline.map_handle_to_font_id,
            &fonts,
        );

        buffer.set_size(font_system.as_mut(), bounds.width, bounds.height);

        buffer.set_wrap(&mut font_system, attributes.line_break.into());

        let attrs = cosmic_text::Attrs::new()
            .metadata(0)
            .family(cosmic_text::Family::Name(&face_info.family_name))
            .stretch(face_info.stretch)
            .style(face_info.style)
            .weight(face_info.weight)
            .metrics(metrics);

        buffer.set_text(
            &mut font_system,
            &placeholder.0,
            &attrs,
            cosmic_text::Shaping::Advanced,
        );

        let align = Some(attributes.justify.into());
        for buffer_line in buffer.lines.iter_mut() {
            buffer_line.set_align(align);
        }

        buffer.shape_until_scroll(&mut font_system, false);

        let box_size = buffer_dimensions(buffer);
        let result = buffer.layout_runs().try_for_each(|run| {
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
                                &mut texture_atlases,
                                &mut textures,
                                &mut font_system,
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
                    layout.glyphs.push(pos_glyph);
                    Ok(())
                })
        });

        prompt_layout.layout.size = target.scale_factor.recip() * box_size;

        match result {
            Err(TextError::NoSuchFont) => {
                // There was an error processing the text layout, try again next frame
                prompt_layout.layout.clear();
            }
            Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
                panic!("Fatal error when processing text: {e}.");
            }
            Ok(()) => {}
        }
    }
}
