use crate::{error::TextError, Font, FontAtlas};
use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_reflect::TypePath;
use bevy_reflect::TypeUuid;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::HashMap;
use cosmic_text::Placement;

type FontSizeKey = u32;

#[derive(TypeUuid, TypePath)]
#[uuid = "73ba778b-b6b5-4f45-982d-d21b6b86ace2"]
pub struct FontAtlasSet {
    font_atlases: HashMap<FontSizeKey, Vec<FontAtlas>>,
}

#[derive(Debug, Clone)]
pub struct GlyphAtlasInfo {
    pub texture_atlas: Handle<TextureAtlas>,
    pub glyph_index: usize,
    pub placement: Placement,
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

        let (glyph_texture, placement) =
            Font::get_outlined_glyph_texture(font_system, swash_cache, layout_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph(
                textures,
                texture_atlases,
                layout_glyph.cache_key,
                &glyph_texture,
                placement,
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
                placement,
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
                            .map(|glyph_index| (glyph_index, atlas.texture_atlas.clone_weak()))
                    })
                    .map(|((glyph_index, placement), texture_atlas)| GlyphAtlasInfo {
                        texture_atlas,
                        glyph_index,
                        placement,
                    })
            })
    }

    pub fn num_font_atlases(&self) -> usize {
        self.font_atlases.len()
    }
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub position: Vec2,
    pub size: Vec2,
    pub atlas_info: GlyphAtlasInfo,
    pub section_index: usize,
    /// In order to do text editing, we need access to the size of glyphs and their index in the associated String.
    /// For example, to figure out where to place the cursor in an input box from the mouse's position.
    /// Without this, it's only possible in texts where each glyph is one byte.
    // TODO: re-implement this or equivalent
    pub byte_index: usize,
}
