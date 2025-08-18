use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_image::{prelude::*, ImageSampler, ToExtents};
use bevy_math::{IVec2, UVec2};
use bevy_platform::collections::HashMap;
use wgpu_types::{TextureDimension, TextureFormat};

use crate::{FontSmoothing, GlyphAtlasLocation, TextError};

/// Rasterized glyphs are cached, stored in, and retrieved from, a `FontAtlas`.
///
/// A `FontAtlas` contains one or more textures, each of which contains one or more glyphs packed into them.
///
/// A [`FontAtlasSet`](crate::FontAtlasSet) contains a `FontAtlas` for each font size in the same font face.
///
/// For the same font face and font size, a glyph will be rasterized differently for different subpixel offsets.
/// In practice, ranges of subpixel offsets are grouped into subpixel bins to limit the number of rasterized glyphs,
/// providing a trade-off between visual quality and performance.
///
/// A [`CacheKey`](cosmic_text::CacheKey) encodes all of the information of a subpixel-offset glyph and is used to
/// find that glyphs raster in a [`TextureAtlas`] through its corresponding [`GlyphAtlasLocation`].
pub struct FontAtlas {
    /// Used to update the [`TextureAtlasLayout`].
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    /// A mapping between subpixel-offset glyphs and their [`GlyphAtlasLocation`].
    pub glyph_to_atlas_index: HashMap<cosmic_text::CacheKey, GlyphAtlasLocation>,
    /// The handle to the [`TextureAtlasLayout`] that holds the rasterized glyphs.
    pub texture_atlas: Handle<TextureAtlasLayout>,
    /// The texture where this font atlas is located
    pub texture: Handle<Image>,
}

impl FontAtlas {
    /// Create a new [`FontAtlas`] with the given size, adding it to the appropriate asset collections.
    pub fn new(
        textures: &mut Assets<Image>,
        texture_atlases_layout: &mut Assets<TextureAtlasLayout>,
        size: UVec2,
        font_smoothing: FontSmoothing,
    ) -> FontAtlas {
        let mut image = Image::new_fill(
            size.to_extents(),
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            // Need to keep this image CPU persistent in order to add additional glyphs later on
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        if font_smoothing == FontSmoothing::None {
            image.sampler = ImageSampler::nearest();
        }
        let texture = textures.add(image);
        let texture_atlas = texture_atlases_layout.add(TextureAtlasLayout::new_empty(size));
        Self {
            texture_atlas,
            glyph_to_atlas_index: HashMap::default(),
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 1),
            texture,
        }
    }

    /// Get the [`GlyphAtlasLocation`] for a subpixel-offset glyph.
    pub fn get_glyph_index(&self, cache_key: cosmic_text::CacheKey) -> Option<GlyphAtlasLocation> {
        self.glyph_to_atlas_index.get(&cache_key).copied()
    }

    /// Checks if the given subpixel-offset glyph is contained in this [`FontAtlas`].
    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey) -> bool {
        self.glyph_to_atlas_index.contains_key(&cache_key)
    }

    /// Add a glyph to the atlas, updating both its texture and layout.
    ///
    /// The glyph is represented by `glyph`, and its image content is `glyph_texture`.
    /// This content is copied into the atlas texture, and the atlas layout is updated
    /// to store the location of that glyph into the atlas.
    ///
    /// # Returns
    ///
    /// Returns `()` if the glyph is successfully added, or [`TextError::FailedToAddGlyph`] otherwise.
    /// In that case, neither the atlas texture nor the atlas layout are
    /// modified.
    pub fn add_glyph(
        &mut self,
        textures: &mut Assets<Image>,
        atlas_layouts: &mut Assets<TextureAtlasLayout>,
        cache_key: cosmic_text::CacheKey,
        texture: &Image,
        offset: IVec2,
    ) -> Result<(), TextError> {
        let atlas_layout = atlas_layouts.get_mut(&self.texture_atlas).unwrap();
        let atlas_texture = textures.get_mut(&self.texture).unwrap();

        if let Ok(glyph_index) =
            self.dynamic_texture_atlas_builder
                .add_texture(atlas_layout, texture, atlas_texture)
        {
            self.glyph_to_atlas_index.insert(
                cache_key,
                GlyphAtlasLocation {
                    glyph_index,
                    offset,
                },
            );
            Ok(())
        } else {
            Err(TextError::FailedToAddGlyph(cache_key.glyph_id))
        }
    }
}

impl core::fmt::Debug for FontAtlas {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FontAtlas")
            .field("glyph_to_atlas_index", &self.glyph_to_atlas_index)
            .field("texture_atlas", &self.texture_atlas)
            .field("texture", &self.texture)
            .field("dynamic_texture_atlas_builder", &"[...]")
            .finish()
    }
}
