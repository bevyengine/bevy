use bevy_asset::Assets;
use bevy_math::{IVec2, Vec2};
use bevy_reflect::{TypePath, TypeUuid};
use bevy_render::{
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_sprite::TextureAtlas;
use bevy_utils::HashMap;

use crate::{error::TextError, FontAtlas, GlyphAtlasInfo};

type FontSizeKey = u32;

/// Provides the interface for adding and retrieving rasterized glyphs, and manages the [`FontAtlas`]es.
///
/// A `FontAtlasSet` is an asset.
///
/// There is one `FontAtlasSet` for each font:
/// - When a [`Font`](crate::Font) is loaded as an asset and then used in [`Text`](crate::Text),
///   a `FontAtlasSet` asset is created from a weak handle to the `Font`.
/// - When a font is loaded as a system font, and then used in [`Text`](crate::Text),
///   a `FontAtlasSet` asset is created and stored with a strong handle to the `FontAtlasSet`.
///
/// A `FontAtlasSet` contains one or more [`FontAtlas`]es for each font size.
///
/// It is used by [`TextPipeline::queue_text`](crate::TextPipeline::queue_text).
#[derive(TypeUuid, TypePath)]
#[uuid = "73ba778b-b6b5-4f45-982d-d21b6b86ace2"]
pub struct FontAtlasSet {
    font_atlases: HashMap<FontSizeKey, Vec<FontAtlas>>,
}

impl Default for FontAtlasSet {
    fn default() -> Self {
        FontAtlasSet {
            font_atlases: HashMap::with_capacity_and_hasher(1, Default::default()),
        }
    }
}

impl FontAtlasSet {
    pub fn iter(&self) -> impl Iterator<Item = (&FontSizeKey, &Vec<FontAtlas>)> {
        self.font_atlases.iter()
    }

    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey, font_size: f32) -> bool {
        self.font_atlases
            .get(&font_size.to_bits())
            .map_or(false, |font_atlas| {
                font_atlas.iter().any(|atlas| atlas.has_glyph(cache_key))
            })
    }

    pub fn add_glyph_to_atlas(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let font_atlases = self
            .font_atlases
            .entry(layout_glyph.cache_key.font_size_bits)
            .or_insert_with(|| {
                vec![FontAtlas::new(
                    textures,
                    texture_atlases,
                    Vec2::splat(512.0),
                )]
            });

        let (glyph_texture, offset) =
            Self::get_outlined_glyph_texture(font_system, swash_cache, layout_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph(
                textures,
                texture_atlases,
                layout_glyph.cache_key,
                &glyph_texture,
                offset,
            )
        };
        if !font_atlases.iter_mut().any(add_char_to_font_atlas) {
            // Find the largest dimension of the glyph, either its width or its height
            let glyph_max_size: u32 = glyph_texture
                .texture_descriptor
                .size
                .height
                .max(glyph_texture.texture_descriptor.size.width);
            // Pick the higher of 512 or the smallest power of 2 greater than glyph_max_size
            let containing = (1u32 << (32 - glyph_max_size.leading_zeros())).max(512) as f32;
            font_atlases.push(FontAtlas::new(
                textures,
                texture_atlases,
                Vec2::new(containing, containing),
            ));
            if !font_atlases.last_mut().unwrap().add_glyph(
                textures,
                texture_atlases,
                layout_glyph.cache_key,
                &glyph_texture,
                offset,
            ) {
                return Err(TextError::FailedToAddGlyph(layout_glyph.cache_key.glyph_id));
            }
        }

        Ok(self.get_glyph_atlas_info(layout_glyph.cache_key).unwrap())
    }

    pub fn get_glyph_atlas_info(
        &mut self,
        cache_key: cosmic_text::CacheKey,
    ) -> Option<GlyphAtlasInfo> {
        self.font_atlases
            .get(&cache_key.font_size_bits)
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas
                            .get_glyph_index(cache_key)
                            .map(|location| (location, atlas.texture_atlas.clone_weak()))
                    })
                    .map(|(location, texture_atlas)| GlyphAtlasInfo {
                        texture_atlas,
                        location,
                    })
            })
    }

    pub fn num_font_atlases(&self) -> usize {
        self.font_atlases.len()
    }

    /// Get the texture of the glyph as a rendered image, and its offset
    pub fn get_outlined_glyph_texture(
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
    ) -> (Image, IVec2) {
        let image = swash_cache
            .get_image_uncached(font_system, layout_glyph.cache_key)
            // TODO: don't unwrap
            .unwrap();

        let cosmic_text::Placement {
            left,
            top,
            width,
            height,
        } = image.placement;

        let data = match image.content {
            cosmic_text::SwashContent::Mask => image
                .data
                .iter()
                .flat_map(|a| [255, 255, 255, *a])
                .collect(),
            cosmic_text::SwashContent::Color => image.data,
            cosmic_text::SwashContent::SubpixelMask => {
                // TODO
                todo!()
            }
        };

        (
            Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
            ),
            IVec2::new(left, top),
        )
    }
}
