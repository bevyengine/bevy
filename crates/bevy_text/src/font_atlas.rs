use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_image::{prelude::*, ImageSampler, ToExtents};
use bevy_math::{UVec2, Vec2};
use bevy_platform::collections::HashMap;
use swash::{scale::Scaler, zeno::Format};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::{FontSmoothing, GlyphAtlasInfo, GlyphAtlasLocation, TextError};

/// Key identifying a glyph
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    /// Id used to look up the glyph
    pub glyph_id: u16,
}

#[doc(hidden)]
pub const TEXT_EFFECT_PADDING: u32 = 16;

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
/// A [`GlyphCacheKey`] encodes all of the information of a subpixel-offset glyph and is used to
/// find that glyphs raster in a [`TextureAtlas`] through its corresponding [`GlyphAtlasLocation`].
pub struct FontAtlas {
    /// Used to update the [`TextureAtlasLayout`].
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    /// A mapping between subpixel-offset glyphs and their [`GlyphAtlasLocation`].
    pub glyph_to_atlas_index: HashMap<GlyphCacheKey, GlyphAtlasLocation>,
    /// The layout for the font atlas.
    pub texture_atlas: TextureAtlasLayout,
    /// The texture where this font atlas is located
    pub texture: Handle<Image>,
}

impl FontAtlas {
    /// Create a new [`FontAtlas`] with the given size, adding it to the appropriate asset collections.
    pub fn new(
        textures: &mut Assets<Image>,
        size: UVec2,
        font_smoothing: FontSmoothing,
    ) -> FontAtlas {
        let mut image = Image::new_fill(
            size.to_extents(),
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8Unorm,
            // Need to keep this image CPU persistent in order to add additional glyphs later on
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        if font_smoothing == FontSmoothing::None {
            image.sampler = ImageSampler::nearest();
        }
        let texture = textures.add(image);
        Self {
            texture_atlas: TextureAtlasLayout::new_empty(size),
            glyph_to_atlas_index: HashMap::default(),
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 2),
            texture,
        }
    }

    /// Get the [`GlyphAtlasLocation`] for a subpixel-offset glyph.
    pub fn get_glyph_index(&self, cache_key: GlyphCacheKey) -> Option<GlyphAtlasLocation> {
        self.glyph_to_atlas_index.get(&cache_key).copied()
    }

    /// Checks if the given subpixel-offset glyph is contained in this [`FontAtlas`].
    pub fn has_glyph(&self, cache_key: GlyphCacheKey) -> bool {
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
        key: GlyphCacheKey,
        texture: &Image,
        offset: Vec2,
        text_effect_padding: bool,
    ) -> Result<(), TextError> {
        let mut atlas_texture = textures
            .get_mut(&self.texture)
            .ok_or(TextError::MissingAtlasTexture)?;

        if let Ok(glyph_index) = self.dynamic_texture_atlas_builder.add_texture(
            &mut self.texture_atlas,
            texture,
            &mut atlas_texture,
        ) {
            self.glyph_to_atlas_index.insert(
                key,
                GlyphAtlasLocation {
                    glyph_index,
                    offset,
                },
            );

            if text_effect_padding {
                let glyph_rect = &mut self.texture_atlas.textures[glyph_index];
                glyph_rect.min += UVec2::splat(TEXT_EFFECT_PADDING);
                glyph_rect.max -= UVec2::splat(TEXT_EFFECT_PADDING);
            }

            Ok(())
        } else {
            Err(TextError::FailedToAddGlyph(key.glyph_id))
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
    textures: &mut Assets<Image>,
    scaler: &mut Scaler,
    font_smoothing: FontSmoothing,
    glyph_id: u16,
    text_effect_padding: bool,
    outline_width: Option<f32>,
) -> Result<GlyphAtlasInfo, TextError> {
    let (glyph_texture, offset) = get_glyph_texture(
        scaler,
        glyph_id,
        font_smoothing,
        text_effect_padding,
        outline_width,
    )?;
    let mut add_char_to_font_atlas = |atlas: &mut FontAtlas| -> Result<(), TextError> {
        atlas.add_glyph(
            textures,
            GlyphCacheKey { glyph_id },
            &glyph_texture,
            offset,
            text_effect_padding,
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

        let mut new_atlas = FontAtlas::new(textures, UVec2::splat(containing), font_smoothing);

        new_atlas.add_glyph(
            textures,
            GlyphCacheKey { glyph_id },
            &glyph_texture,
            offset,
            text_effect_padding,
        )?;

        font_atlases.push(new_atlas);
    }

    get_glyph_atlas_info(font_atlases, GlyphCacheKey { glyph_id })
        .ok_or(TextError::InconsistentAtlasState)
}

/// Get the texture of the glyph as a rendered image, and its offset
pub fn get_glyph_texture(
    scaler: &mut Scaler,
    glyph_id: u16,
    font_smoothing: FontSmoothing,
    text_effect_padding: bool,
    outline_width: Option<f32>,
) -> Result<(Image, Vec2), TextError> {
    let image = swash::scale::Render::new(&[
        swash::scale::Source::ColorOutline(0),
        swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
        swash::scale::Source::Outline,
    ])
    .format(Format::Alpha)
    .render(scaler, glyph_id)
    .ok_or(TextError::FailedToGetGlyphImage(glyph_id))?;

    let left = image.placement.left;
    let top = image.placement.top;
    let mut width = image.placement.width;
    let mut height = image.placement.height;

    let mut fill_alpha = apply_font_smoothing(&image.data, font_smoothing);
    if text_effect_padding {
        fill_alpha = pad_mask(&fill_alpha, width, height, TEXT_EFFECT_PADDING);
        width += TEXT_EFFECT_PADDING * 2;
        height += TEXT_EFFECT_PADDING * 2;
    }

    let outline_alpha = outline_width
        .filter(|outline_width| 0.0 < *outline_width)
        .map(|outline_width| build_outline_mask(&fill_alpha, width, height, outline_width));
    let data = pack_text_glyph_texture(&fill_alpha, outline_alpha.as_deref());

    Ok((
        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD,
        ),
        Vec2::new(left as f32, -top as f32),
    ))
}

fn apply_font_smoothing(alpha: &[u8], font_smoothing: FontSmoothing) -> Vec<u8> {
    match font_smoothing {
        FontSmoothing::AntiAliased => alpha.to_vec(),
        FontSmoothing::None => alpha
            .iter()
            .map(|alpha| if 127 < *alpha { 255 } else { 0 })
            .collect(),
    }
}

fn pad_mask(mask: &[u8], width: u32, height: u32, padding: u32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let padding = padding as usize;
    let padded_width = width + padding * 2;
    let padded_height = height + padding * 2;
    let mut padded = vec![0; padded_width * padded_height];

    for y in 0..height {
        let src_start = y * width;
        let dst_start = (y + padding) * padded_width + padding;
        padded[dst_start..dst_start + width].copy_from_slice(&mask[src_start..src_start + width]);
    }

    padded
}

fn build_outline_mask(fill_alpha: &[u8], width: u32, height: u32, outline_width: f32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let max_radius = outline_width.ceil().max(1.0) as i32;
    let max_distance_squared = (outline_width + 0.5) * (outline_width + 0.5);
    let mut offsets = Vec::new();

    for y in -max_radius..=max_radius {
        for x in -max_radius..=max_radius {
            let distance_squared = (x * x + y * y) as f32;
            if distance_squared <= max_distance_squared {
                offsets.push((x, y));
            }
        }
    }

    let mut outline_alpha = vec![0; fill_alpha.len()];
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            let mut dilated = 0;

            for &(offset_x, offset_y) in &offsets {
                let sample_x = x as i32 + offset_x;
                let sample_y = y as i32 + offset_y;
                if !(0..width as i32).contains(&sample_x) || !(0..height as i32).contains(&sample_y)
                {
                    continue;
                }

                let sample_index = sample_y as usize * width + sample_x as usize;
                dilated = dilated.max(fill_alpha[sample_index]);
            }

            outline_alpha[index] = dilated.saturating_sub(fill_alpha[index]);
        }
    }

    outline_alpha
}

fn pack_text_glyph_texture(fill_alpha: &[u8], outline_alpha: Option<&[u8]>) -> Vec<u8> {
    let mut rgba = vec![0; fill_alpha.len() * 4];

    for (i, fill_alpha) in fill_alpha.iter().enumerate() {
        rgba[i * 4] = outline_alpha.map_or(0, |outline_alpha| outline_alpha[i]);
        rgba[i * 4 + 3] = *fill_alpha;
    }

    rgba
}

/// Generates the [`GlyphAtlasInfo`] for the given subpixel-offset glyph.
pub fn get_glyph_atlas_info(
    font_atlases: &mut [FontAtlas],
    cache_key: GlyphCacheKey,
) -> Option<GlyphAtlasInfo> {
    font_atlases.iter().find_map(|atlas| {
        atlas
            .get_glyph_index(cache_key)
            .map(|location| GlyphAtlasInfo {
                offset: location.offset,
                rect: atlas.texture_atlas.textures[location.glyph_index].as_rect(),
                texture: atlas.texture.id(),
            })
    })
}
