use ab_glyph::GlyphId;
use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_render::texture::{Texture, TextureFormat};
use bevy_sprite::{DynamicTextureAtlasBuilder, TextureAtlas};
use bevy_utils::HashMap;

pub struct FontAtlas {
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    pub glyph_to_index: HashMap<char, u32>,
    pub glyph_to_atlas_index: HashMap<GlyphId, u32>,
    pub texture_atlas: Handle<TextureAtlas>,
}

impl FontAtlas {
    pub fn new(
        textures: &mut Assets<Texture>,
        texture_atlases: &mut Assets<TextureAtlas>,
        size: Vec2,
    ) -> FontAtlas {
        let atlas_texture = textures.add(Texture::new_fill(
            size,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
        ));
        let texture_atlas = TextureAtlas::new_empty(atlas_texture, size);
        Self {
            texture_atlas: texture_atlases.add(texture_atlas),
            glyph_to_index: HashMap::default(),
            glyph_to_atlas_index: HashMap::default(),
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 1),
        }
    }

    pub fn get_glyph_index(&self, glyph_id: GlyphId) -> Option<u32> {
        self.glyph_to_atlas_index.get(&glyph_id).copied()
    }

    pub fn has_glyph(&self, glyph_id: GlyphId) -> bool {
        self.glyph_to_atlas_index.contains_key(&glyph_id)
    }

    pub fn add_glyph(
        &mut self,
        textures: &mut Assets<Texture>,
        texture_atlases: &mut Assets<TextureAtlas>,
        glyph_id: GlyphId,
        texture: &Texture,
    ) -> bool {
        let texture_atlas = texture_atlases.get_mut(&self.texture_atlas).unwrap();
        if let Some(index) =
            self.dynamic_texture_atlas_builder
                .add_texture(texture_atlas, textures, texture)
        {
            self.glyph_to_atlas_index.insert(glyph_id, index);
            true
        } else {
            false
        }
    }
}
