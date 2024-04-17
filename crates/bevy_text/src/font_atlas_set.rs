use crate::{error::TextError, Font, FontAtlas};
use ab_glyph::{GlyphId, OutlinedGlyph, Point};
use bevy_asset::{AssetEvent, AssetId};
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_math::{FloatOrd, UVec2};
use bevy_reflect::Reflect;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlasLayout;
use bevy_utils::HashMap;

type FontSizeKey = FloatOrd;

#[derive(Default, Resource)]
pub struct FontAtlasSets {
    // PERF: in theory this could be optimized with Assets storage ... consider making some fast "simple" AssetMap
    pub(crate) sets: HashMap<AssetId<Font>, FontAtlasSet>,
}

impl FontAtlasSets {
    pub fn get(&self, id: impl Into<AssetId<Font>>) -> Option<&FontAtlasSet> {
        let id: AssetId<Font> = id.into();
        self.sets.get(&id)
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

pub struct FontAtlasSet {
    font_atlases: HashMap<FontSizeKey, Vec<FontAtlas>>,
}

#[derive(Debug, Clone, Reflect)]
pub struct GlyphAtlasInfo {
    pub texture_atlas: Handle<TextureAtlasLayout>,
    pub texture: Handle<Image>,
    pub glyph_index: usize,
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

    pub fn has_glyph(&self, glyph_id: GlyphId, glyph_position: Point, font_size: f32) -> bool {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .map_or(false, |font_atlas| {
                font_atlas
                    .iter()
                    .any(|atlas| atlas.has_glyph(glyph_id, glyph_position.into()))
            })
    }

    pub fn add_glyph_to_atlas(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        outlined_glyph: OutlinedGlyph,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let glyph = outlined_glyph.glyph();
        let glyph_id = glyph.id;
        let glyph_position = glyph.position;
        let font_size = glyph.scale.y;
        let font_atlases = self
            .font_atlases
            .entry(FloatOrd(font_size))
            .or_insert_with(|| vec![FontAtlas::new(textures, texture_atlases, UVec2::splat(512))]);

        let glyph_texture = Font::get_outlined_glyph_texture(outlined_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph(
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
                .max(glyph_texture.width());
            // Pick the higher of 512 or the smallest power of 2 greater than glyph_max_size
            let containing = (1u32 << (32 - glyph_max_size.leading_zeros())).max(512);
            font_atlases.push(FontAtlas::new(
                textures,
                texture_atlases,
                UVec2::splat(containing),
            ));
            if !font_atlases.last_mut().unwrap().add_glyph(
                textures,
                texture_atlases,
                glyph_id,
                glyph_position.into(),
                &glyph_texture,
            ) {
                return Err(TextError::FailedToAddGlyph(glyph_id));
            }
        }

        Ok(self
            .get_glyph_atlas_info(font_size, glyph_id, glyph_position)
            .unwrap())
    }

    pub fn get_glyph_atlas_info(
        &mut self,
        font_size: f32,
        glyph_id: GlyphId,
        position: Point,
    ) -> Option<GlyphAtlasInfo> {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas
                            .get_glyph_index(glyph_id, position.into())
                            .map(|glyph_index| {
                                (
                                    glyph_index,
                                    atlas.texture_atlas.clone_weak(),
                                    atlas.texture.clone_weak(),
                                )
                            })
                    })
                    .map(|(glyph_index, texture_atlas, texture)| GlyphAtlasInfo {
                        texture_atlas,
                        texture,
                        glyph_index,
                    })
            })
    }

    /// Returns the number of font atlases in this set
    pub fn len(&self) -> usize {
        self.font_atlases.len()
    }

    /// Returns `true` if the font atlas set contains no elements
    pub fn is_empty(&self) -> bool {
        self.font_atlases.is_empty()
    }
}
