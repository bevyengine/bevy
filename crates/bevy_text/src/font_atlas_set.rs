use crate::{Font, FontAtlas, FontSmoothing, TextFont};
use bevy_asset::{AssetEvent, AssetId};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    message::MessageReader,
    resource::Resource,
    system::{Local, Query, ResMut},
};
use bevy_platform::collections::{HashMap, HashSet};

/// Identifies the font atlases for a particular font in [`FontAtlasSet`]
///
/// Allows an `f32` font size to be used as a key in a `HashMap`, by its binary representation.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FontAtlasKey(pub AssetId<Font>, pub u32, pub FontSmoothing);

impl From<&TextFont> for FontAtlasKey {
    fn from(font: &TextFont) -> Self {
        FontAtlasKey(
            font.font.id(),
            font.font_size.to_bits(),
            font.font_smoothing,
        )
    }
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
}

/// A system that automatically frees unused texture atlases when a font asset is removed.
pub fn free_unused_font_atlases_system(
    mut font_atlas_sets: ResMut<FontAtlasSet>,
    mut font_events: MessageReader<AssetEvent<Font>>,
) {
    for event in font_events.read() {
        if let AssetEvent::Removed { id } = event {
            font_atlas_sets.retain(|key, _| key.0 != *id);
        }
    }
}

#[derive(Resource)]
/// Maximum number of font atlas sets.
pub struct MaxFontAtlasSets(pub usize);

impl Default for MaxFontAtlasSets {
    fn default() -> Self {
        Self(20)
    }
}

#[derive(Component, PartialEq, Default)]
/// Computed font derived from `TextFont` and the scale factor of the render target.
pub struct ComputedFont(pub Option<FontAtlasKey>);

/// Automatically frees unused fonts when the total number of fonts
/// is greater than the [`MaxFonts`] value. Doesn't free in use fonts
/// even if the number of in use fonts is greater than [`MaxFonts`].
pub fn free_unused_font_atlases(
    // list of unused fonts in order from least to most recently used
    mut least_recently_used: Local<Vec<FontAtlasKey>>,
    // fonts that were in use the previous frame
    mut previous_active_fonts: Local<HashSet<FontAtlasKey>>,
    mut active_fonts: Local<HashSet<FontAtlasKey>>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    max_fonts: ResMut<MaxFontAtlasSets>,
    active_font_query: Query<&ComputedFont>,
) {
    // collect keys for all fonts currently in use by a text entity
    active_fonts.extend(
        active_font_query
            .iter()
            .filter_map(|computed_font| computed_font.0),
    );

    // remove any keys for fonts in use from the least recently used list
    least_recently_used.retain(|font| !active_fonts.contains(font));

    // push keys for any fonts no longer in use onto the least recently used list
    least_recently_used.extend(
        previous_active_fonts
            .difference(&active_fonts)
            .into_iter()
            .cloned(),
    );

    // If the total number of fonts is greater than max_fonts, free fonts from the least rcently used list
    // until the total is lower than max_fonts or the least recently used list is empty.
    let number_of_fonts_to_free = font_atlas_set
        .len()
        .saturating_sub(max_fonts.0)
        .min(least_recently_used.len());
    for font_atlas_key in least_recently_used.drain(..number_of_fonts_to_free) {
        font_atlas_set.remove(&font_atlas_key);
    }

    previous_active_fonts.clear();
    core::mem::swap(&mut *previous_active_fonts, &mut *active_fonts);
}
