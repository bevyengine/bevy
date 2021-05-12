use ab_glyph::{GlyphId, Point};
use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_render::texture::{Extent3d, Texture, TextureDimension, TextureFormat};
use bevy_sprite::{DynamicTextureAtlasBuilder, TextureAtlas};
use bevy_utils::HashMap;

#[cfg(feature = "subpixel_glyph_atlas")]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SubpixelOffset {
    x: u16,
    y: u16,
}

#[cfg(feature = "subpixel_glyph_atlas")]
impl From<Point> for SubpixelOffset {
    fn from(p: Point) -> Self {
        fn f(v: f32) -> u16 {
            ((v % 1.) * (u16::MAX as f32)) as u16
        }
        Self {
            x: f(p.x),
            y: f(p.y),
        }
    }
}

#[cfg(not(feature = "subpixel_glyph_atlas"))]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SubpixelOffset;

#[cfg(not(feature = "subpixel_glyph_atlas"))]
impl From<Point> for SubpixelOffset {
    fn from(_: Point) -> Self {
        Self
    }
}

pub struct FontAtlas {
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    pub glyph_to_atlas_index: HashMap<(GlyphId, SubpixelOffset), u32>,
    pub texture_atlas: Handle<TextureAtlas>,
}

impl FontAtlas {
    pub fn new(
        textures: &mut Assets<Texture>,
        texture_atlases: &mut Assets<TextureAtlas>,
        size: Vec2,
    ) -> FontAtlas {
        let atlas_texture = textures.add(Texture::new_fill(
            Extent3d::new(size.x as u32, size.y as u32, 1),
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
        glyph_id: GlyphId,
        subpixel_offset: SubpixelOffset,
    ) -> Option<u32> {
        self.glyph_to_atlas_index
            .get(&(glyph_id, subpixel_offset))
            .copied()
    }

    pub fn has_glyph(&self, glyph_id: GlyphId, subpixel_offset: SubpixelOffset) -> bool {
        self.glyph_to_atlas_index
            .contains_key(&(glyph_id, subpixel_offset))
    }

    pub fn add_glyph(
        &mut self,
        textures: &mut Assets<Texture>,
        texture_atlases: &mut Assets<TextureAtlas>,
        glyph_id: GlyphId,
        subpixel_offset: SubpixelOffset,
        texture: &Texture,
    ) -> bool {
        let texture_atlas = texture_atlases.get_mut(&self.texture_atlas).unwrap();
        if let Some(index) =
            self.dynamic_texture_atlas_builder
                .add_texture(texture_atlas, textures, texture)
        {
            self.glyph_to_atlas_index
                .insert((glyph_id, subpixel_offset), index);
            true
        } else {
            false
        }
    }
}
