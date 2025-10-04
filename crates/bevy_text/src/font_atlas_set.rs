use crate::{Font, FontAtlas, FontSmoothing};
use bevy_asset::{AssetEvent, AssetId};
use bevy_ecs::{message::MessageReader, resource::Resource, system::ResMut};
use bevy_platform::collections::HashMap;

/// Identifies the font atlases for a particular font in [`FontAtlasSet`]
///
/// The `u32` value is bitcast from the font's `f32` size value.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FontAtlasKey(pub AssetId<Font>, pub u32, pub FontSmoothing);

/// Set of rasterized font's stored in [`FontAtlas`]es.
#[derive(Debug, Default, Resource)]
pub struct FontAtlasSet {
    // PERF: in theory this could be optimized with Assets storage ... consider making some fast "simple" AssetMap
    pub(crate) sets: HashMap<FontAtlasKey, Vec<FontAtlas>>,
}

impl FontAtlasSet {
    /// Get a reference to the [`FontAtlas`]es with the given font asset id.
    pub fn get(&self, id: FontAtlasKey) -> Option<&[FontAtlas]> {
        self.sets.get(&id).map(Vec::as_slice)
    }

    /// Get a mutable reference to the [`FontAtlas`]es with the given font asset id.
    pub fn get_mut(&mut self, id: FontAtlasKey) -> Option<&mut Vec<FontAtlas>> {
        self.sets.get_mut(&id)
    }

    /// Returns the total number of rasterized font faces stored in this `FontAtlasSet`.
    pub fn font_count(&self) -> usize {
        self.sets.len()
    }

    /// Returns an iterator over each font and its corresponding list of [`FontAtlas`]es.
    pub fn iter(&self) -> impl Iterator<Item = (&FontAtlasKey, &Vec<FontAtlas>)> {
        self.sets.iter()
    }

    /// Checks if the given subpixel-offset glyph is contained in any of the [`FontAtlas`]es for this font.
    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey, font_key: &FontAtlasKey) -> bool {
        self.sets
            .get(font_key)
            .is_some_and(|font_atlas| font_atlas.iter().any(|atlas| atlas.has_glyph(cache_key)))
    }
}

/// System that automatically frees the unused texture atlases after a font asset is removed.
pub fn free_unused_font_atlases_system(
    mut font_atlas_sets: ResMut<FontAtlasSet>,
    mut font_events: MessageReader<AssetEvent<Font>>,
) {
    for event in font_events.read() {
        if let AssetEvent::Removed { id } = event {
            font_atlas_sets.sets.retain(|key, _| key.0 != *id);
        }
    }
}
