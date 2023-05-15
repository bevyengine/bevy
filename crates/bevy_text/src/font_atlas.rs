use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_render::{
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_sprite::{DynamicTextureAtlasBuilder, TextureAtlas};
use bevy_utils::HashMap;

pub struct FontAtlas {
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    pub glyph_to_atlas_index: HashMap<cosmic_text::CacheKey, (usize, i32, i32, u32, u32)>,
    pub texture_atlas: Handle<TextureAtlas>,
}

impl FontAtlas {
    pub fn new(
        textures: &mut Assets<Image>,
        texture_atlases: &mut Assets<TextureAtlas>,
        size: Vec2,
    ) -> FontAtlas {
        let atlas_texture = textures.add(Image::new_fill(
            Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
        ));
        let texture_atlas = TextureAtlas::new_empty(atlas_texture, size);
        Self {
            texture_atlas: texture_atlases.add(texture_atlas),
            glyph_to_atlas_index: HashMap::default(),
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 1),
        }
    }

    pub fn get_glyph_index(
        &self,
        cache_key: cosmic_text::CacheKey,
    ) -> Option<(usize, i32, i32, u32, u32)> {
        self.glyph_to_atlas_index.get(&cache_key).copied()
    }

    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey) -> bool {
        self.glyph_to_atlas_index.contains_key(&cache_key)
    }

    pub fn add_glyph(
        &mut self,
        textures: &mut Assets<Image>,
        texture_atlases: &mut Assets<TextureAtlas>,
        cache_key: cosmic_text::CacheKey,
        texture: &Image,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
    ) -> bool {
        let texture_atlas = texture_atlases.get_mut(&self.texture_atlas).unwrap();
        if let Some(index) =
            self.dynamic_texture_atlas_builder
                .add_texture(texture_atlas, textures, texture)
        {
            self.glyph_to_atlas_index
                .insert(cache_key, (index, left, top, width, height));
            true
        } else {
            false
        }
    }
}
