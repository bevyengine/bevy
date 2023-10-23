use bevy_asset::{Asset, AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_ecs::{
    event::EventReader,
    system::{ResMut, Resource},
};
use bevy_math::{FloatOrd, IVec2, UVec2, Vec2};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_sprite::TextureAtlasLayout;
use bevy_utils::HashMap;

use crate::{error::TextError, Font, FontAtlas, GlyphAtlasInfo};

#[derive(Debug, Default, Resource)]
pub struct FontAtlasSets {
    // PERF: in theory this could be optimized with Assets storage ... consider making some fast "simple" AssetMap
    pub(crate) sets: HashMap<AssetId<Font>, FontAtlasSet>,
}

impl FontAtlasSets {
    pub fn get(&self, id: impl Into<AssetId<Font>>) -> Option<&FontAtlasSet> {
        let id: AssetId<Font> = id.into();
        self.sets.get(&id)
    }
    pub fn get_mut(&mut self, id: impl Into<AssetId<Font>>) -> Option<&mut FontAtlasSet> {
        let id: AssetId<Font> = id.into();
        self.sets.get_mut(&id)
    }
}

pub fn remove_dropped_font_atlas_sets(
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut font_events: EventReader<AssetEvent<Font>>,
) {
    // Clean up font atlas sets for removed fonts
    for event in font_events.read() {
        if let AssetEvent::Removed { id } = event {
            font_atlas_sets.sets.remove(id);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FontSizeKey(pub u32);

impl From<u32> for FontSizeKey {
    fn from(val: u32) -> FontSizeKey {
        Self(val)
    }
}

/// Provides the interface for adding and retrieving rasterized glyphs, and manages the [`FontAtlas`]es.
///
/// A `FontAtlasSet` is an [`Asset`].
///
/// There is one `FontAtlasSet` for each font:
/// - When a [`Font`] is loaded as an asset and then used in [`Text`](crate::Text),
///   a `FontAtlasSet` asset is created from a weak handle to the `Font`.
/// - When a font is loaded as a system font, and then used in [`Text`](crate::Text),
///   a `FontAtlasSet` asset is created and stored with a strong handle to the `FontAtlasSet`.
///
/// A `FontAtlasSet` contains one or more [`FontAtlas`]es for each font size.
///
/// It is used by [`TextPipeline::queue_text`](crate::TextPipeline::queue_text).
#[derive(Debug, TypePath, Asset)]
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

    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey, font_size: &FontSizeKey) -> bool {
        self.font_atlases
            .get(font_size)
            .map_or(false, |font_atlas| {
                font_atlas.iter().any(|atlas| atlas.has_glyph(cache_key))
            })
    }

    pub fn add_glyph_to_atlas(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let physical_glyph = layout_glyph.physical((0., 0.), 1.0);

        let font_atlases = self
            .font_atlases
            .entry(physical_glyph.cache_key.font_size_bits.into())
            .or_insert_with(|| {
                vec![FontAtlas::new(
                    textures,
                    texture_atlases,
                    Vec2::splat(512.0),
                )]
            });

        let (glyph_texture, offset) =
            Self::get_outlined_glyph_texture(font_system, swash_cache, &physical_glyph)?;
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
            ));

            font_atlases.last_mut().unwrap().add_glyph(
                textures,
                texture_atlases,
                physical_glyph.cache_key,
                &glyph_texture,
                offset,
            )?
        }

        Ok(self.get_glyph_atlas_info(physical_glyph.cache_key).unwrap())
    }

    pub fn get_glyph_atlas_info(
        &mut self,
        cache_key: cosmic_text::CacheKey,
    ) -> Option<GlyphAtlasInfo> {
        self.font_atlases
            .get(&FontSizeKey(cache_key.font_size_bits))
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas.get_glyph_index(cache_key).map(|location| {
                            (
                                location,
                                atlas.texture_atlas.clone_weak(),
                                atlas.texture.clone_weak(),
                            )
                        })
                    })
                    .map(|(location, texture_atlas, texture)| GlyphAtlasInfo {
                        texture_atlas,
                        location,
                        texture,
                    })
            })
    }

    /// Returns the number of font atlases in this set
    pub fn len(&self) -> usize {
        self.font_atlases.len()
    }
    /// Returns the number of font atlases in this set
    pub fn is_empty(&self) -> bool {
        self.font_atlases.len() == 0
    }

    /// Get the texture of the glyph as a rendered image, and its offset
    pub fn get_outlined_glyph_texture(
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        physical_glyph: &cosmic_text::PhysicalGlyph,
    ) -> Result<(Image, IVec2), TextError> {
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
