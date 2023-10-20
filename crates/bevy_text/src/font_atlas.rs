use bevy_asset::{Assets, Handle};
use bevy_math::{IVec2, Vec2};
use bevy_render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_sprite::{DynamicTextureAtlasBuilder, TextureAtlasLayout};
use bevy_utils::HashMap;

use crate::GlyphAtlasLocation;

/// Rasterized glyphs are cached, stored in, and retrieved from, a `FontAtlas`.
///
/// A [`FontAtlasSet`](crate::FontAtlasSet) contains one or more `FontAtlas`es.
pub struct FontAtlas {
    /// Used to update the [`TextureAtlas`].
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    /// A mapping between subpixel-binned glyphs and their [`GlyphAtlasLocation`].
    pub glyph_to_atlas_index: HashMap<cosmic_text::CacheKey, GlyphAtlasLocation>,
    /// The handle to the [`TextureAtlas`] that holds the rasterized glyphs.
    pub texture_atlas: Handle<TextureAtlasLayout>,
    /// the texture where this font atlas is located
    pub texture: Handle<Image>,
}

impl FontAtlas {
    pub fn new(
        textures: &mut Assets<Image>,
        texture_atlases_layout: &mut Assets<TextureAtlasLayout>,
        size: Vec2,
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
        let texture_atlas = texture_atlases_layout.add(TextureAtlasLayout::new_empty(size));
        Self {
            texture_atlas,
            glyph_to_atlas_index: HashMap::default(),
            dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder::new(size, 0),
            texture,
        }
    }

    pub fn get_glyph_index(&self, cache_key: cosmic_text::CacheKey) -> Option<GlyphAtlasLocation> {
        self.glyph_to_atlas_index.get(&cache_key).copied()
    }

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
    /// Returns `true` if the glyph is successfully added, or `false` otherwise.
    /// In that case, neither the atlas texture nor the atlas layout are
    /// modified.
    pub fn add_glyph(
        &mut self,
        textures: &mut Assets<Image>,crates/bevy_text/src/font_atlas_set.rs
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        cache_key: cosmcrates/bevy_text/src/font_atlas_set.rsic_text::CacheKey,
        texture: &Image,
        offset: IVec2,
    ) -> bool {
        let texture_atlas = texture_atlases.get_mut(&self.texture_atlas).unwrap();

        if let Some(glyph_index) = self.dynamic_texture_atlas_builder.add_texture(
            texture_atlas,
            textures,
            texture,
            &self.texture,
        ) {
            self.glyph_to_atlas_index.insert(
                cache_key,
                GlyphAtlasLocation {
                    glyph_index,
                    offset,
                },
            );
            true
        } else {
            false
        }
    }
}
