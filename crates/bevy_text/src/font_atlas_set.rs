use crate::{FontAtlas, FontSmoothing};
use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::resource::Resource;
use bevy_image::Image;
use bevy_platform::collections::HashMap;
use cosmic_text::fontdb::ID;

/// Identifies the font atlases for a particular font in [`FontAtlasSet`]
///
/// Allows an `f32` font size to be used as a key in a `HashMap`, by its binary representation.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FontAtlasKey {
    /// Font asset id
    pub id: ID,
    /// Font size via `f32::to_bits`
    pub font_size_bits: u32,
    /// Antialiasing method
    pub font_smoothing: FontSmoothing,
}

/// Set of rasterized fonts stored in [`FontAtlas`]es.
#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct FontAtlasSet(HashMap<FontAtlasKey, Vec<FontAtlas>>);

impl FontAtlasSet {
    /// Checks whether the given subpixel-offset glyph is contained in any of the [`FontAtlas`]es for the font identified by the given [`FontAtlasKey`].
    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey, font_key: &FontAtlasKey) -> bool {
        self.get(font_key)
            .is_some_and(|font_atlas| font_atlas.iter().any(|atlas| atlas.has_glyph(cache_key)))
    }

    /// Returns the total size in bytes of the image data for all fonts.
    pub fn total_bytes(&self, images: &Assets<Image>) -> u64 {
        self.values()
            .flat_map(|font_atlases| font_atlases.iter())
            .map(|font_atlas| {
                images
                    .get(&font_atlas.texture)
                    .and_then(|image| image.data.as_ref())
                    .map_or(0, |data| data.len() as u64)
            })
            .sum()
    }
}
