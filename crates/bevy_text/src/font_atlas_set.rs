use bevy_asset::{AssetEvent, AssetId, Assets, RenderAssetUsages};
use bevy_ecs::{event::EventReader, resource::Resource, system::ResMut};
use bevy_image::prelude::*;
use bevy_math::{IVec2, UVec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::{error::TextError, Font, FontAtlas, FontSmoothing, GlyphAtlasInfo};

/// A map of font faces to their corresponding [`FontAtlasSet`]s.
#[derive(Debug, Default, Resource)]
pub struct FontAtlasSets {
    // PERF: in theory this could be optimized with Assets storage ... consider making some fast "simple" AssetMap
    pub(crate) sets: HashMap<AssetId<Font>, FontAtlasSet>,
}

impl FontAtlasSets {
    /// Get a reference to the [`FontAtlasSet`] with the given font asset id.
    pub fn get(&self, id: impl Into<AssetId<Font>>) -> Option<&FontAtlasSet> {
        let id: AssetId<Font> = id.into();
        self.sets.get(&id)
    }
    /// Get a mutable reference to the [`FontAtlasSet`] with the given font asset id.
    pub fn get_mut(&mut self, id: impl Into<AssetId<Font>>) -> Option<&mut FontAtlasSet> {
        let id: AssetId<Font> = id.into();
        self.sets.get_mut(&id)
    }
}

/// A system that cleans up [`FontAtlasSet`]s for removed [`Font`]s
pub fn remove_dropped_font_atlas_sets(
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut font_events: EventReader<AssetEvent<Font>>,
) {
    for event in font_events.read() {
        if let AssetEvent::Removed { id } = event {
            font_atlas_sets.sets.remove(id);
        }
    }
}

/// Identifies a font size and smoothing method in a [`FontAtlasSet`].
///
/// Allows an `f32` font size to be used as a key in a `HashMap`, by its binary representation.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FontAtlasKey(pub u32, pub FontSmoothing);

/// A map of font sizes to their corresponding [`FontAtlas`]es, for a given font face.
///
/// Provides the interface for adding and retrieving rasterized glyphs, and manages the [`FontAtlas`]es.
///
/// There is at most one `FontAtlasSet` for each font, stored in the `FontAtlasSets` resource.
/// `FontAtlasSet`s are added and updated by the [`queue_text`](crate::pipeline::TextPipeline::queue_text) function.
///
/// A `FontAtlasSet` contains one or more [`FontAtlas`]es for each font size.
#[derive(Debug, TypePath)]
pub struct FontAtlasSet {
    font_atlases: HashMap<FontAtlasKey, Vec<FontAtlas>>,
}

impl Default for FontAtlasSet {
    fn default() -> Self {
        FontAtlasSet {
            font_atlases: HashMap::with_capacity_and_hasher(1, Default::default()),
        }
    }
}

impl FontAtlasSet {
    /// Returns an iterator over the [`FontAtlas`]es in this set
    pub fn iter(&self) -> impl Iterator<Item = (&FontAtlasKey, &Vec<FontAtlas>)> {
        self.font_atlases.iter()
    }

    /// Checks if the given subpixel-offset glyph is contained in any of the [`FontAtlas`]es in this set
    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey, font_size: &FontAtlasKey) -> bool {
        self.font_atlases
            .get(font_size)
            .is_some_and(|font_atlas| font_atlas.iter().any(|atlas| atlas.has_glyph(cache_key)))
    }

    /// Adds the given subpixel-offset glyph to the [`FontAtlas`]es in this set
    pub fn add_glyph_to_atlas(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
        font_smoothing: FontSmoothing,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let physical_glyph = layout_glyph.physical((0., 0.), 1.0);

        let font_atlases = self
            .font_atlases
            .entry(FontAtlasKey(
                physical_glyph.cache_key.font_size_bits,
                font_smoothing,
            ))
            .or_insert_with(|| {
                vec![FontAtlas::new(
                    textures,
                    texture_atlases,
                    UVec2::splat(512),
                    font_smoothing,
                )]
            });

        let (glyph_texture, offset) = Self::get_outlined_glyph_texture(
            font_system,
            swash_cache,
            &physical_glyph,
            font_smoothing,
        )?;
        let mut add_char_to_font_atlas = |atlas: &mut FontAtlas| -> Result<(), TextError> {
            atlas.add_glyph(
                textures,
                texture_atlases,
                physical_glyph.cache_key,
                &glyph_texture,
                offset,
            )
        };
        if !font_atlases
            .iter_mut()
            .any(|atlas| add_char_to_font_atlas(atlas).is_ok())
        {
            // Find the largest dimension of the glyph, either its width or its height
            let glyph_max_size: u32 = glyph_texture
                .texture_descriptor
                .size
                .height
                .max(glyph_texture.width());
            // Pick the higher of 512 or the smallest power of 2 greater than glyph_max_size
            let containing = (1u32 << (32 - glyph_max_size.leading_zeros())).max(512);
            font_atlases.push(FontAtlas::new(
                textures,
                texture_atlases,
                UVec2::splat(containing),
                font_smoothing,
            ));

            font_atlases.last_mut().unwrap().add_glyph(
                textures,
                texture_atlases,
                physical_glyph.cache_key,
                &glyph_texture,
                offset,
            )?;
        }

        Ok(self
            .get_glyph_atlas_info(physical_glyph.cache_key, font_smoothing)
            .unwrap())
    }

    /// Generates the [`GlyphAtlasInfo`] for the given subpixel-offset glyph.
    pub fn get_glyph_atlas_info(
        &mut self,
        cache_key: cosmic_text::CacheKey,
        font_smoothing: FontSmoothing,
    ) -> Option<GlyphAtlasInfo> {
        self.font_atlases
            .get(&FontAtlasKey(cache_key.font_size_bits, font_smoothing))
            .and_then(|font_atlases| {
                font_atlases.iter().find_map(|atlas| {
                    atlas
                        .get_glyph_index(cache_key)
                        .map(|location| GlyphAtlasInfo {
                            location,
                            texture_atlas: atlas.texture_atlas.id(),
                            texture: atlas.texture.id(),
                        })
                })
            })
    }

    /// Returns the number of font atlases in this set.
    pub fn len(&self) -> usize {
        self.font_atlases.len()
    }

    /// Returns `true` if the set has no font atlases.
    pub fn is_empty(&self) -> bool {
        self.font_atlases.len() == 0
    }

    /// Get the texture of the glyph as a rendered image, and its offset
    pub fn get_outlined_glyph_texture(
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        physical_glyph: &cosmic_text::PhysicalGlyph,
        font_smoothing: FontSmoothing,
    ) -> Result<(Image, IVec2), TextError> {
        // NOTE: Ideally, we'd ask COSMIC Text to honor the font smoothing setting directly.
        // However, since it currently doesn't support that, we render the glyph with antialiasing
        // and apply a threshold to the alpha channel to simulate the effect.
        //
        // This has the side effect of making regular vector fonts look quite ugly when font smoothing
        // is turned off, but for fonts that are specifically designed for pixel art, it works well.
        //
        // See: https://github.com/pop-os/cosmic-text/issues/279
        let image = swash_cache
            .get_image_uncached(font_system, physical_glyph.cache_key)
            .ok_or(TextError::FailedToGetGlyphImage(physical_glyph.cache_key))?;

        let cosmic_text::Placement {
            left,
            top,
            width,
            height,
        } = image.placement;

        let data = match image.content {
            cosmic_text::SwashContent::Mask => {
                if font_smoothing == FontSmoothing::None {
                    image
                        .data
                        .iter()
                        // Apply a 50% threshold to the alpha channel
                        .flat_map(|a| [255, 255, 255, if *a > 127 { 255 } else { 0 }])
                        .collect()
                } else {
                    image
                        .data
                        .iter()
                        .flat_map(|a| [255, 255, 255, *a])
                        .collect()
                }
            }
            cosmic_text::SwashContent::Color => image.data,
            cosmic_text::SwashContent::SubpixelMask => {
                // TODO: implement
                todo!()
            }
        };

        Ok((
            Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD,
            ),
            IVec2::new(left, top),
        ))
    }
}
