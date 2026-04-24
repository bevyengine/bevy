use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_image::{prelude::*, ImageSampler, ToExtents};
use bevy_math::{UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use swash::scale::Scaler;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::{FontSmoothing, GlyphAtlasInfo, GlyphAtlasLocation, TextError};

/// Horizontal subpixel quantisation bucket for
/// [`FontSmoothing::SubpixelAntiAliased`].
///
/// Groups ranges of fractional glyph x-positions into four buckets plus a
/// `NotApplicable` case for non-subpixel smoothing modes. Used inside
/// [`GlyphCacheKey`] so the atlas holds four distinct rasterisations per
/// glyph id at the appropriate horizontal subpixel offsets.
///
/// Four buckets is the quality / atlas-count sweet spot borrowed from the
/// cosmic-text-era stack (which mirrored `cosmic_text::SubpixelBin`); parley
/// does not expose its own bucketing, so this enum is purely internal to
/// `bevy_text`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Hash, Default, PartialEq, Debug, Clone)]
pub enum SubpixelBucket {
    /// Non-subpixel smoothing (`None` / `AntiAliased`). A single atlas cell
    /// per glyph id.
    #[default]
    NotApplicable,
    /// Fractional x in `[0.0, 0.25)` — rasterised at swash offset `0.0`.
    Zero,
    /// Fractional x in `[0.25, 0.5)` — rasterised at swash offset `0.25`.
    Quarter,
    /// Fractional x in `[0.5, 0.75)` — rasterised at swash offset `0.5`.
    Half,
    /// Fractional x in `[0.75, 1.0)` — rasterised at swash offset `0.75`.
    ThreeQuarter,
}

impl SubpixelBucket {
    /// Derive a bucket from a fractional x-position and the current font
    /// smoothing mode.
    ///
    /// Returns [`SubpixelBucket::NotApplicable`] for any smoothing mode other
    /// than [`FontSmoothing::SubpixelAntiAliased`], collapsing grayscale /
    /// `None` glyphs to a single atlas cell per glyph id.
    pub fn from_fract(fract_x: f32, smoothing: FontSmoothing) -> Self {
        if smoothing != FontSmoothing::SubpixelAntiAliased {
            return Self::NotApplicable;
        }
        let frac = fract_x.rem_euclid(1.0);
        if frac < 0.25 {
            Self::Zero
        } else if frac < 0.5 {
            Self::Quarter
        } else if frac < 0.75 {
            Self::Half
        } else {
            Self::ThreeQuarter
        }
    }

    /// The horizontal rasterisation offset for this bucket, intended to be
    /// fed to [`swash::scale::Render::offset`] before `.render(...)`.
    pub fn rasterise_offset_x(self) -> f32 {
        match self {
            Self::NotApplicable | Self::Zero => 0.0,
            Self::Quarter => 0.25,
            Self::Half => 0.5,
            Self::ThreeQuarter => 0.75,
        }
    }
}

/// Key identifying a glyph entry inside a [`FontAtlas`].
///
/// Combines the glyph id with a [`SubpixelBucket`] so that
/// [`FontSmoothing::SubpixelAntiAliased`] can cache four distinct subpixel
/// rasterisations of the same glyph within a single atlas texture. Non-
/// subpixel smoothing modes always use [`SubpixelBucket::NotApplicable`],
/// giving a single atlas cell per glyph id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    /// Id used to look up the glyph.
    pub glyph_id: u16,
    /// Horizontal subpixel bucket the glyph was rasterised for.
    pub subpixel_bucket: SubpixelBucket,
}

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
            TextureFormat::Rgba8UnormSrgb,
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
        is_alpha_mask: bool,
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
                    is_alpha_mask,
                },
            );
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
    subpixel_bucket: SubpixelBucket,
    subpixel_offset: Vec2,
) -> Result<GlyphAtlasInfo, TextError> {
    let (glyph_texture, offset, is_alpha_mask) =
        get_outlined_glyph_texture(scaler, glyph_id, font_smoothing, subpixel_offset)?;
    let cache_key = GlyphCacheKey {
        glyph_id,
        subpixel_bucket,
    };
    let mut add_char_to_font_atlas = |atlas: &mut FontAtlas| -> Result<(), TextError> {
        atlas.add_glyph(textures, cache_key, &glyph_texture, offset, is_alpha_mask)
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

        new_atlas.add_glyph(textures, cache_key, &glyph_texture, offset, is_alpha_mask)?;

        font_atlases.push(new_atlas);
    }

    get_glyph_atlas_info(font_atlases, cache_key).ok_or(TextError::InconsistentAtlasState)
}

/// Get the texture of the glyph as a rendered image, and its offset
#[expect(
    clippy::identity_op,
    reason = "Alignment improves clarity during RGBA operations."
)]
pub fn get_outlined_glyph_texture(
    scaler: &mut Scaler,
    glyph_id: u16,
    font_smoothing: FontSmoothing,
    subpixel_offset: Vec2,
) -> Result<(Image, Vec2, bool), TextError> {
    // `None` and `AntiAliased` both ask swash for a single-channel alpha mask
    // which the match below unpacks into an RGBA texture. `SubpixelAntiAliased`
    // asks for RGB subpixel coverage, which swash returns as a four-byte-per-
    // pixel buffer (`[R_cov, G_cov, B_cov, 0]`) with `Content::SubpixelMask`.
    // The existing `Content::Color | Content::SubpixelMask` arm below passes
    // that buffer through unchanged; the shader side of the subpixel path uses
    // the RGB channels as per-channel coverage in a dual-source blend.
    //
    // `subpixel_offset` is fed to `swash::scale::Render::offset(...)` only on
    // the subpixel path; the quantised horizontal value comes from
    // [`SubpixelBucket::rasterise_offset_x`] at the atlas-lookup call site.
    // Non-subpixel callers pass `Vec2::ZERO` and the offset is ignored.
    let format = match font_smoothing {
        FontSmoothing::None | FontSmoothing::AntiAliased => swash::zeno::Format::Alpha,
        FontSmoothing::SubpixelAntiAliased => swash::zeno::Format::Subpixel,
    };
    let mut render = swash::scale::Render::new(&[
        swash::scale::Source::ColorOutline(0),
        swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
        swash::scale::Source::Outline,
    ]);
    render.format(format);
    if font_smoothing == FontSmoothing::SubpixelAntiAliased {
        render.offset(swash::zeno::Vector::new(
            subpixel_offset.x,
            subpixel_offset.y,
        ));
    }
    let image = render
        .render(scaler, glyph_id)
        .ok_or(TextError::FailedToGetGlyphImage(glyph_id))?;

    let left = image.placement.left;
    let top = image.placement.top;
    let width = image.placement.width;
    let height = image.placement.height;

    let px = (width * height) as usize;
    let rgba = match image.content {
        swash::scale::image::Content::Mask => {
            let mut rgba = vec![0u8; px * 4];
            match font_smoothing {
                // Subpixel renders are requested via `Format::Subpixel` and normally
                // produce `Content::SubpixelMask`. If swash ever falls back to a grayscale
                // mask for a given glyph (for example, a source that only has a plain
                // outline on an emoji-style font), treat it as grayscale AA — still
                // better than returning black pixels.
                FontSmoothing::AntiAliased | FontSmoothing::SubpixelAntiAliased => {
                    for i in 0..px {
                        let a = image.data[i];
                        rgba[i * 4 + 0] = 255; // R
                        rgba[i * 4 + 1] = 255; // G
                        rgba[i * 4 + 2] = 255; // B
                        rgba[i * 4 + 3] = a; // A from swash
                    }
                }
                FontSmoothing::None => {
                    for i in 0..px {
                        let a = image.data[i];
                        rgba[i * 4 + 0] = 255; // R
                        rgba[i * 4 + 1] = 255; // G
                        rgba[i * 4 + 2] = 255; // B
                        rgba[i * 4 + 3] = if 127 < a { 255 } else { 0 }; // A from swash
                    }
                }
            }
            rgba
        }
        swash::scale::image::Content::Color | swash::scale::image::Content::SubpixelMask => {
            image.data
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
            rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        ),
        Vec2::new(left as f32, -top as f32),
        image.content == swash::scale::image::Content::Mask,
    ))
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
                is_alpha_mask: location.is_alpha_mask,
            })
    })
}
