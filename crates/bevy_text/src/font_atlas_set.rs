use crate::{Font, FontAtlas, FontSmoothing, TextFont};
use bevy_asset::{AssetEvent, AssetId};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component, lifecycle::HookContext, message::MessageReader, resource::Resource,
    system::ResMut, world::DeferredWorld,
};
use bevy_platform::collections::HashMap;

/// Identifies the font atlases for a particular font in [`FontAtlasSet`]
///
/// Allows an `f32` font size to be used as a key in a `HashMap`, by its binary representation.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
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
pub struct MaxUnusedFontAtlasSets(pub usize);

impl Default for MaxUnusedFontAtlasSets {
    fn default() -> Self {
        Self(20)
    }
}

#[derive(Component, PartialEq, Default)]
#[component(
    on_insert = on_insert_computed_text_font,
    on_replace = on_remove_computed_text_font,
    on_remove = on_remove_computed_text_font,
)]
/// Computed font derived from `TextFont` and the scale factor of the render target.
pub struct ComputedTextFont(FontAtlasKey);

impl ComputedTextFont {
    /// new ComputedTextFont
    pub fn new(font: &TextFont, scale_factor: f32) -> Self {
        Self(FontAtlasKey(
            font.font.id(),
            (scale_factor * font.font_size).to_bits(),
            font.font_smoothing,
        ))
    }
}

#[derive(Resource, Default)]
/// Counts entities using font atlases
pub struct FontAtlasesManager {
    counts: HashMap<FontAtlasKey, usize>,
    lru: Vec<FontAtlasKey>,
}

fn on_insert_computed_text_font(mut world: DeferredWorld, hook_context: HookContext) {
    let key = world
        .get::<ComputedTextFont>(hook_context.entity)
        .unwrap()
        .0;
    *world
        .resource_mut::<FontAtlasesManager>()
        .counts
        .entry(key)
        .or_default() += 1;
}

fn on_remove_computed_text_font(mut world: DeferredWorld, hook_context: HookContext) {
    if let Some(&ComputedTextFont(key)) = world.get::<ComputedTextFont>(hook_context.entity) {
        let mut f = world.resource_mut::<FontAtlasesManager>();
        let c = f.counts.entry(key).or_default();
        *c -= 1;
        if *c == 0 {
            f.lru.push(key);
        }
    }
}

/// Automatically frees unused fonts when the total number of fonts
/// is greater than the [`MaxFonts`] value. Doesn't free in use fonts
/// even if the number of in use fonts is greater than [`MaxFonts`].
pub fn free_unused_font_atlases(
    mut font_atlases_manager: ResMut<FontAtlasesManager>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    max_fonts: ResMut<MaxUnusedFontAtlasSets>,
) {
    // If the total number of fonts is greater than max_fonts, free fonts from the least rcently used list
    // until the total is lower than max_fonts or the least recently used list is empty.
    let FontAtlasesManager { counts, lru } = &mut *font_atlases_manager;
    let number_of_fonts_to_free = font_atlas_set
        .len()
        .saturating_sub(max_fonts.0)
        .min(lru.len());
    for font_atlas_key in lru.drain(..number_of_fonts_to_free) {
        if counts.get(&font_atlas_key) == Some(&0) {
            font_atlas_set.remove(&font_atlas_key);
            counts.remove(&font_atlas_key);
        }
    }
}
