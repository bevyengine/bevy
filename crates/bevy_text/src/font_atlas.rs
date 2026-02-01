use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_image::{prelude::*, ImageSampler, ToExtents};
use bevy_math::{IVec2, UVec2};
use bevy_platform::collections::HashMap;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::{FontSmoothing, GlyphAtlasInfo, GlyphAtlasLocation, TextError};

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
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 2),
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
        let atlas_layout = atlas_layouts
            .get_mut(&self.texture_atlas)
            .ok_or(TextError::MissingAtlasLayout)?;
        let atlas_texture = textures
            .get_mut(&self.texture)
            .ok_or(TextError::MissingAtlasTexture)?;

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

/// Adds the given subpixel-offset glyph to the given font atlases
pub fn add_glyph_to_atlas(
    font_atlases: &mut Vec<FontAtlas>,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    textures: &mut Assets<Image>,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    layout_glyph: &cosmic_text::LayoutGlyph,
    font_smoothing: FontSmoothing,
) -> Result<GlyphAtlasInfo, TextError> {
    let physical_glyph = layout_glyph.physical((0., 0.), 1.0);

    let (glyph_texture, offset) =
        get_outlined_glyph_texture(font_system, swash_cache, &physical_glyph, font_smoothing)?;
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

        let mut new_atlas = FontAtlas::new(
            textures,
            texture_atlases,
            UVec2::splat(containing),
            font_smoothing,
        );

        new_atlas.add_glyph(
            textures,
            texture_atlases,
            physical_glyph.cache_key,
            &glyph_texture,
            offset,
        )?;

        font_atlases.push(new_atlas);
    }

    get_glyph_atlas_info(font_atlases, physical_glyph.cache_key)
        .ok_or(TextError::InconsistentAtlasState)
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

/// Generates the [`GlyphAtlasInfo`] for the given subpixel-offset glyph.
pub fn get_glyph_atlas_info(
    font_atlases: &mut [FontAtlas],
    cache_key: cosmic_text::CacheKey,
) -> Option<GlyphAtlasInfo> {
    font_atlases.iter().find_map(|atlas| {
        atlas
            .get_glyph_index(cache_key)
            .map(|location| GlyphAtlasInfo {
                location,
                texture_atlas: atlas.texture_atlas.id(),
                texture: atlas.texture.id(),
            })
    })
}
