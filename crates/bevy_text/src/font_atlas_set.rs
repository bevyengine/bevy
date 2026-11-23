use crate::{FontAtlas, FontHinting, FontSmoothing, GlyphCacheKey, TextBrush};
use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::resource::Resource;
use bevy_image::Image;
use bevy_platform::{collections::HashMap, hash::FixedHasher};
use core::hash::BuildHasher;
use parley::GlyphRun;

/// Identifies the font atlases for a particular font in [`FontAtlasSet`]
///
/// Allows an `f32` font size to be used as a key in a `HashMap`, by its binary representation.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FontAtlasKey {
    /// Font data id
    pub id: u32,
    /// Font data index
    pub index: u32,
    /// Font size via `f32::to_bits`
    pub font_size_bits: u32,
    /// Hash of normalized variation coords for this run.
    pub variations_hash: u64,
    /// Synthetic italic skew in degrees via `f32::to_bits`, `0.0` when unskewed
    pub synthetic_skew_bits: u32,
    /// Hinting
    pub hinting: FontHinting,
    /// Antialiasing method
    pub font_smoothing: FontSmoothing,
}

impl FontAtlasKey {
    /// Create a new `FontAtlasKey`.
    pub fn new(glyph_run: &GlyphRun<'_, TextBrush>, hinting: FontHinting) -> Self {
        let run = glyph_run.run();
        let font = run.font();
        Self {
            id: font.data.id() as u32,
            index: font.index,
            font_size_bits: run.font_size().to_bits(),
            variations_hash: FixedHasher.hash_one(run.normalized_coords()),
            synthetic_skew_bits: run.synthesis().skew().unwrap_or_default().to_bits(),
            hinting,
            font_smoothing: glyph_run.style().brush.font_smoothing,
        }
    }
}

/// Set of rasterized fonts stored in [`FontAtlas`]es.
#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct FontAtlasSet(HashMap<FontAtlasKey, Vec<FontAtlas>>);

impl FontAtlasSet {
    /// Checks whether the given subpixel-offset glyph is contained in any of the [`FontAtlas`]es for the font identified by the given [`FontAtlasKey`].
    pub fn has_glyph(&self, cache_key: GlyphCacheKey, font_key: &FontAtlasKey) -> bool {
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

    /// Clear all font atlases
    pub fn clear(&mut self) {
        self.0.clear();
    }
}
