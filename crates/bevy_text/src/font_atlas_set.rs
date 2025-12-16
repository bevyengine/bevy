use crate::{Font, FontAtlas, FontSmoothing, TextFont};
use bevy_asset::{AssetEvent, AssetId};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{message::MessageReader, resource::Resource, system::ResMut};
use bevy_platform::collections::HashMap;
use cosmic_text::fontdb::ID;

/// Identifies the font atlases for a particular font in [`FontAtlasSet`]
///
/// Allows an `f32` font size to be used as a key in a `HashMap`, by its binary representation.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FontAtlasKey(pub ID, pub u32, pub FontSmoothing);

// impl From<&TextFont> for FontAtlasKey {
//     fn from(font: &TextFont) -> Self {
//         FontAtlasKey(
//             font.font.id(),
//             font.font_size.to_bits(),
//             font.font_smoothing,
//         )
//     }
// }

/// Set of rasterized fonts stored in [`FontAtlas`]es.
#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct FontAtlasSet(HashMap<FontAtlasKey, Vec<FontAtlas>>);

impl FontAtlasSet {
    /// Checks whether the given subpixel-offset glyph is contained in any of the [`FontAtlas`]es for the font identified by the given [`FontAtlasKey`].
    pub fn has_glyph(&self, cache_key: cosmic_text::CacheKey, font_key: &FontAtlasKey) -> bool {
        self.get(font_key)
            .is_some_and(|font_atlas| font_atlas.iter().any(|atlas| atlas.has_glyph(cache_key)))
    }
}

// /// A system that automatically frees unused texture atlases when a font asset is removed.
// pub fn free_unused_font_atlases_system(
//     mut font_atlas_sets: ResMut<FontAtlasSet>,
//     mut font_events: MessageReader<AssetEvent<Font>>,
// ) {
//     for event in font_events.read() {
//         if let AssetEvent::Removed { id } = event {
//             font_atlas_sets.retain(|key, _| key.0 != *id);
//         }
//     }
// }
