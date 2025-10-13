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

#[derive(Component, PartialEq, Default)]
#[component(
    on_insert = on_insert_computed_text_font,
    on_replace = on_replace_computed_text_font,
)]
/// Computed font derived from `TextFont` and the scale factor of the render target.
pub struct ComputedTextFont(pub(crate) Option<FontAtlasKey>);

impl ComputedTextFont {
    /// Create a new `ComputedTextFont` from the given `TextFont` and scale factor.
    pub fn new(font: &TextFont, scale_factor: f32) -> Self {
        Self(Some(FontAtlasKey(
            font.font.id(),
            (scale_factor * font.font_size).to_bits(),
            font.font_smoothing,
        )))
    }
}

fn on_insert_computed_text_font(mut world: DeferredWorld, hook_context: HookContext) {
    if let Some(key) = world
        .get::<ComputedTextFont>(hook_context.entity)
        .unwrap()
        .0
    {
        *world
            .resource_mut::<FontAtlasManager>()
            .reference_counts
            .entry(key)
            .or_default() += 1;
    }
}

fn on_replace_computed_text_font(mut world: DeferredWorld, hook_context: HookContext) {
    if let Some(&ComputedTextFont(Some(key))) = world.get::<ComputedTextFont>(hook_context.entity) {
        let mut f = world.resource_mut::<FontAtlasManager>();
        let c = f.reference_counts.entry(key).or_default();
        *c -= 1;
        if *c == 0 {
            f.least_recently_used_buffer.push(key);
        }
    }
}

#[derive(Resource, Default)]
/// Used to keep a count of the number of text entities using each font, and decide
/// when font atlases should be freed.
pub struct FontAtlasManager {
    reference_counts: HashMap<FontAtlasKey, usize>,
    least_recently_used_buffer: Vec<FontAtlasKey>,
    /// Maximum number of fonts before unused font atlases are freed.
    pub max_fonts: usize,
}

impl FontAtlasManager {
    /// New font atlas manager with the given max fonts limits.
    pub fn new(max_fonts: usize) -> Self {
        Self {
            max_fonts,
            ..Default::default()
        }
    }

    /// Returns the number of text entities using the font with the given key.
    pub fn get_count(&self, key: &FontAtlasKey) -> usize {
        self.reference_counts.get(key).copied().unwrap_or(0)
    }
}

/// Automatically frees unused fonts when the total number of fonts
/// is greater than [`FontAtlasesManager::max_fonts`]. Doesn't free in use fonts
/// even if the number of in use fonts is greater than  [`FontAtlasesManager::max_fonts`].
pub fn free_unused_font_atlases_computed_system(
    mut font_atlases_manager: ResMut<FontAtlasManager>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
) {
    // If the total number of fonts is greater than max_fonts, free fonts from the least rcently used list
    // until the total is lower than max_fonts or the least recently used list is empty.
    let FontAtlasManager {
        reference_counts,
        least_recently_used_buffer,
        max_fonts,
    } = &mut *font_atlases_manager;
    let mut target = font_atlas_set
        .len()
        .saturating_sub(*max_fonts)
        .min(least_recently_used_buffer.len());
    if 0 < target {
        least_recently_used_buffer.retain(|key| {
            let free = 0 < target && reference_counts.get(key).is_some_and(|&c| c == 0);
            if free {
                font_atlas_set.remove(key);
                reference_counts.remove(key);
                target -= 1;
            }
            free
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::free_unused_font_atlases_computed_system;
    use crate::ComputedTextFont;
    use crate::FontAtlasKey;
    use crate::FontAtlasManager;
    use crate::FontAtlasSet;
    use bevy_app::App;
    use bevy_app::Update;
    use bevy_asset::AssetId;

    #[test]
    fn text_free_unused_font_atlases_computed_system() {
        let mut app = App::new();

        app.init_resource::<FontAtlasManager>();
        app.init_resource::<FontAtlasSet>();

        app.add_systems(Update, free_unused_font_atlases_computed_system);

        let world = app.world_mut();

        let mut font_atlases = world.resource_mut::<FontAtlasSet>();

        let font_atlas_key_1 =
            FontAtlasKey(AssetId::default(), 10, crate::FontSmoothing::AntiAliased);
        let font_atlas_key_2 = FontAtlasKey(AssetId::default(), 10, crate::FontSmoothing::None);

        font_atlases.insert(font_atlas_key_1, vec![]);
        font_atlases.insert(font_atlas_key_2, vec![]);

        let e = world.spawn(ComputedTextFont(Some(font_atlas_key_1))).id();
        let f = world.spawn(ComputedTextFont(Some(font_atlas_key_2))).id();

        app.update();

        let world = app.world_mut();
        let font_atlases = world.resource_mut::<FontAtlasSet>();
        assert_eq!(font_atlases.len(), 2);

        world.despawn(f);

        app.update();

        let world = app.world_mut();
        let font_atlases = world.resource_mut::<FontAtlasSet>();
        assert_eq!(font_atlases.len(), 2);

        world.resource_mut::<FontAtlasManager>().max_fonts = 1;

        app.update();

        let world = app.world_mut();
        let font_atlases = world.resource_mut::<FontAtlasSet>();
        assert_eq!(font_atlases.len(), 1);
        assert!(font_atlases.contains_key(&font_atlas_key_1));
        assert!(!font_atlases.contains_key(&font_atlas_key_2));

        world.despawn(e);
        world.resource_mut::<FontAtlasManager>().max_fonts = 0;

        app.update();

        let world = app.world_mut();
        let font_atlases = world.resource_mut::<FontAtlasSet>();
        assert_eq!(font_atlases.len(), 0);
    }
}
