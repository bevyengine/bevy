use ab_glyph::{GlyphId, Point};
use bevy_asset::{Assets, Handle};
use bevy_math::UVec2;
use bevy_render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_sprite::{DynamicTextureAtlasBuilder, TextureAtlasLayout};
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

/// A font glyph placed at a specific sub-pixel offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlacedGlyph {
    /// The font glyph ID.
    pub glyph_id: GlyphId,
    /// The sub-pixel offset of the placed glyph.
    pub subpixel_offset: SubpixelOffset,
}

pub struct FontAtlas {
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    pub glyph_to_atlas_index: HashMap<PlacedGlyph, usize>,
    pub texture_atlas: Handle<TextureAtlasLayout>,
    pub texture: Handle<Image>,
}

impl FontAtlas {
    pub fn new(
        textures: &mut Assets<Image>,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        size: UVec2,
    ) -> FontAtlas {
        let texture = textures.add(Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            // Need to keep this image CPU persistent in order to add additional glyphs later on
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));
        let texture_atlas = TextureAtlasLayout::new_empty(size);
        Self {
            texture_atlas: texture_atlases.add(texture_atlas),
            glyph_to_atlas_index: HashMap::default(),
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 0),
            texture,
        }
    }

    pub fn get_glyph_index(&self, glyph: &PlacedGlyph) -> Option<usize> {
        self.glyph_to_atlas_index.get(glyph).copied()
    }

    pub fn has_glyph(&self, glyph: &PlacedGlyph) -> bool {
        self.glyph_to_atlas_index.contains_key(glyph)
    }

    /// Add a glyph to the atlas, updating both its texture and layout.
    ///
    /// The glyph is represented by `glyph`, and its image content is `glyph_texture`.
    /// This content is copied into the atlas texture, and the atlas layout is updated
    /// to store the location of that glyph into the atlas.
    ///
    /// # Returns
    ///
    /// Returns `true` if the glyph is successfully added, or `false` otherwise.
    /// In that case, neither the atlas texture nor the atlas layout are
    /// modified.
    pub fn add_glyph(
        &mut self,
        textures: &mut Assets<Image>,
        atlas_layouts: &mut Assets<TextureAtlasLayout>,
        glyph: &PlacedGlyph,
        glyph_texture: &Image,
    ) -> bool {
        let Some(atlas_layout) = atlas_layouts.get_mut(&self.texture_atlas) else {
            return false;
        };
        let Some(atlas_texture) = textures.get_mut(&self.texture) else {
            return false;
        };
        if let Some(index) = self.dynamic_texture_atlas_builder.add_texture(
            atlas_layout,
            glyph_texture,
            atlas_texture,
        ) {
            self.glyph_to_atlas_index.insert(*glyph, index);
            true
        } else {
            false
        }
    }
}
