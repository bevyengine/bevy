use crate::{error::TextError, Font, FontAtlas};
use ab_glyph::{GlyphId, OutlinedGlyph, Point};
use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_reflect::TypePath;
use bevy_reflect::TypeUuid;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::FloatOrd;
use bevy_utils::HashMap;

type FontSizeKeyOld = FloatOrd;
type FontSizeKey = u32;

// TODO: FontAtlasSet is an asset tied to a Handle<Font> cast weakly
// This won't work for "font queries" (non-vendored fonts)
#[derive(TypeUuid, TypePath)]
#[uuid = "73ba778b-b6b5-4f45-982d-d21b6b86ace2"]
pub struct FontAtlasSet {
    font_atlases_old: HashMap<FontSizeKeyOld, Vec<FontAtlas>>,
    font_atlases: HashMap<FontSizeKey, Vec<FontAtlas>>,
}

#[derive(Debug, Clone)]
pub struct GlyphAtlasInfoNew {
    pub texture_atlas: Handle<TextureAtlas>,
    pub glyph_index: usize,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct GlyphAtlasInfo {
    pub texture_atlas: Handle<TextureAtlas>,
    pub glyph_index: usize,
}

impl Default for FontAtlasSet {
    fn default() -> Self {
        FontAtlasSet {
            font_atlases_old: HashMap::with_capacity_and_hasher(1, Default::default()),
            font_atlases: HashMap::with_capacity_and_hasher(1, Default::default()),
        }
    }
}

impl FontAtlasSet {
    pub fn iter(&self) -> impl Iterator<Item = (&FontSizeKey, &Vec<FontAtlas>)> {
        self.font_atlases.iter()
    }

    pub fn has_glyph(&self, glyph_id: GlyphId, glyph_position: Point, font_size: f32) -> bool {
        self.font_atlases
            .get(&font_size.to_bits())
            .map_or(false, |font_atlas| {
                font_atlas
                    .iter()
                    .any(|atlas| atlas.has_glyph(glyph_id, glyph_position.into()))
            })
    }

    pub fn add_glyph_to_atlas_new(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
    ) -> Result<GlyphAtlasInfoNew, TextError> {
        // let glyph = layout_glyph.glyph();
        // let glyph_id = glyph.id;
        // let glyph_position = glyph.position;
        // let font_size = glyph.scale.y;
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

        let (glyph_texture, left, top, w, h) =
            Font::get_outlined_glyph_texture_new(font_system, swash_cache, layout_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph_new(
                textures,
                texture_atlases,
                layout_glyph.cache_key,
                &glyph_texture,
                left,
                top,
                w,
                h,
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
            if !font_atlases.last_mut().unwrap().add_glyph_new(
                textures,
                texture_atlases,
                layout_glyph.cache_key,
                &glyph_texture,
                left,
                top,
                w,
                h,
            ) {
                return Err(TextError::FailedToAddGlyph(layout_glyph.cache_key.glyph_id));
            }
        }

        Ok(self
            .get_glyph_atlas_info_new(layout_glyph.cache_key)
            .unwrap())
    }

    pub fn add_glyph_to_atlas_old(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        outlined_glyph: OutlinedGlyph,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let glyph = outlined_glyph.glyph();
        let glyph_id = glyph.id;
        let glyph_position = glyph.position;
        let font_size = glyph.scale.y;
        let font_atlases = self
            .font_atlases_old
            .entry(FloatOrd(font_size))
            .or_insert_with(|| {
                vec![FontAtlas::new(
                    textures,
                    texture_atlases,
                    Vec2::splat(512.0),
                )]
            });

        let glyph_texture = Font::get_outlined_glyph_texture(outlined_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph_old(
                textures,
                texture_atlases,
                glyph_id,
                glyph_position.into(),
                &glyph_texture,
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
            if !font_atlases.last_mut().unwrap().add_glyph_old(
                textures,
                texture_atlases,
                glyph_id,
                glyph_position.into(),
                &glyph_texture,
            ) {
                return Err(TextError::FailedToAddGlyphOld(glyph_id));
            }
        }

        Ok(self
            .get_glyph_atlas_info_old(font_size, glyph_id, glyph_position)
            .unwrap())
    }

    pub fn get_glyph_atlas_info_new(
        &mut self,
        cache_key: cosmic_text::CacheKey,
    ) -> Option<GlyphAtlasInfoNew> {
        self.font_atlases
            .get(&cache_key.font_size_bits)
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas
                            .get_glyph_index_new(cache_key)
                            .map(|glyph_index| (glyph_index, atlas.texture_atlas.clone_weak()))
                    })
                    .map(
                        |((glyph_index, left, top, w, h), texture_atlas)| GlyphAtlasInfoNew {
                            texture_atlas,
                            glyph_index,
                            left,
                            top,
                            width: w,
                            height: h,
                        },
                    )
            })
    }

    pub fn get_glyph_atlas_info_old(
        &mut self,
        font_size: f32,
        glyph_id: GlyphId,
        position: Point,
    ) -> Option<GlyphAtlasInfo> {
        self.font_atlases_old
            .get(&FloatOrd(font_size))
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas
                            .get_glyph_index(glyph_id, position.into())
                            .map(|glyph_index| (glyph_index, atlas.texture_atlas.clone_weak()))
                    })
                    .map(|(glyph_index, texture_atlas)| GlyphAtlasInfo {
                        texture_atlas,
                        glyph_index,
                    })
            })
    }

    pub fn num_font_atlases(&self) -> usize {
        self.font_atlases_old.len()
    }
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub position: Vec2,
    pub size: Vec2,
    pub atlas_info: GlyphAtlasInfoNew,
    pub section_index: usize,
    /// In order to do text editing, we need access to the size of glyphs and their index in the associated String.
    /// For example, to figure out where to place the cursor in an input box from the mouse's position.
    /// Without this, it's only possible in texts where each glyph is one byte.
    // TODO: re-implement this or equivalent
    pub byte_index: usize,
}
