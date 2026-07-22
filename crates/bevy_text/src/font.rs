use crate::FontCx;
use crate::FontSource;
use crate::TextFont;
use bevy_asset::Asset;
use bevy_asset::AssetId;
use bevy_asset::Assets;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::system::Local;
use bevy_ecs::system::Query;
use bevy_ecs::system::ResMut;
use bevy_platform::collections::HashSet;
use bevy_reflect::TypePath;
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;
use parley::FontFamilyName;

/// An [`Asset`] that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
///
/// # A note on fonts
///
/// `Font` may differ from the everyday notion of what a "font" is.
/// A font *face* (e.g. Fira Sans Semibold Italic) is part of a font *family* (e.g. Fira Sans),
/// and is distinguished from other font faces in the same family
/// by its style (e.g. italic), its weight (e.g. bold) and its stretch (e.g. condensed).
///
/// Bevy currently loads a single font face as a single `Font` asset.
#[derive(Debug, TypePath, Clone, Asset)]
pub struct Font {
    /// Content of a font file as bytes
    pub data: Blob<u8>,
    /// Alias used to identify the asset in the when referenced by handle.
    pub alias: String,
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn from_bytes(font_data: Vec<u8>) -> Font {
        Self {
            data: Blob::from(font_data),
            alias: String::new(),
        }
    }
}

/// Add new font assets to the internal font collection, and set any associated `TextFont`'s changed.
/// If any fonts are removed, the font collection is completely rebuilt, the generic families are remapped, and all `TextFont`s are set changed.
///
/// Font asset changes are track locally instead of waiting for asset events. Text layout also builds the atlas images, and waiting for asset events would
/// delay the image updates by a frame.
pub fn load_font_assets_into_font_collection(
    mut fonts: ResMut<Assets<Font>>,
    mut loaded_fonts: Local<HashSet<AssetId<Font>>>,
    mut font_cx: ResMut<FontCx>,
    mut text_font_query: Query<&mut TextFont>,
) {
    let font_removed = loaded_fonts.iter().any(|id| !fonts.contains(*id));
    let new_asset_ids: Vec<_> = if font_removed {
        // If any font asset has been removed, clear the font collection and queue the remaining fonts to be reinserted into the collection.
        font_cx.collection.clear();
        loaded_fonts.clear();
        loaded_fonts.extend(fonts.ids());
        loaded_fonts.iter().copied().collect()
    } else {
        fonts.ids().filter(|id| loaded_fonts.insert(*id)).collect()
    };

    if new_asset_ids.is_empty() && !font_removed {
        return;
    }

    let mut new_family_ids = Vec::new();
    for asset_id in &new_asset_ids {
        let font = fonts
            .get_mut_untracked(*asset_id)
            .expect("Each AssetId should have a corresponding asset");

        font.alias = format!("asset_id:{asset_id:?}");

        // Each font is registered twice in Parley's FontContext collection, once under its embedded family name,
        // and once under an alias generated from the asset id.
        // This to allow look ups by the embedded family name while also ensuring that font asset handles
        // accurately resolve to the correct font asset.
        new_family_ids.extend(
            font_cx
                .collection
                .register_fonts(font.data.clone(), None)
                .iter()
                .map(|(family_id, _)| *family_id),
        );

        font_cx.collection.register_fonts(
            font.data.clone(),
            Some(FontInfoOverride {
                family_name: Some(font.alias.as_str()),
                ..Default::default()
            }),
        );
    }

    if font_removed {
        font_cx.restore_generic_families();
    }

    for mut text_font in text_font_query.iter_mut() {
        if font_removed
            || text_font
                .font
                .flatten()
                .into_iter()
                .any(|source| match source {
                    FontSource::Handle(handle) => new_asset_ids.contains(&handle.id()),
                    FontSource::Family(name) => font_cx
                        .collection
                        .family_id(name.as_str())
                        .is_some_and(|id| new_family_ids.contains(&id)),
                    FontSource::Families(source) => FontFamilyName::parse_css_list(source.as_str())
                        .map_while(Result::ok)
                        .any(|family| match family {
                            FontFamilyName::Named(name) => font_cx
                                .collection
                                .family_id(name.as_ref())
                                .is_some_and(|id| new_family_ids.contains(&id)),
                            FontFamilyName::Generic(generic_family) => font_cx
                                .collection
                                .generic_families(generic_family)
                                .any(|id| new_family_ids.contains(&id)),
                        }),
                    &FontSource::Generic(generic_family) => font_cx
                        .collection
                        .generic_families(generic_family.into())
                        .any(|id| new_family_ids.contains(&id)),
                    FontSource::List(_) => {
                        unreachable!("FontSource::flatten should not return lists")
                    }
                })
        {
            text_font.set_changed();
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, Update};
    use bevy_asset::Assets;

    use super::*;

    #[test]
    fn font_asset_registration_and_cleanup() {
        let mut app = App::new();
        app.init_resource::<Assets<Font>>()
            .init_resource::<FontCx>()
            .add_systems(Update, load_font_assets_into_font_collection);

        let font_handle = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!("FiraMono-subset.ttf").to_vec(),
            ));

        app.update();
        let world = app.world_mut();

        let font_alias = world
            .resource::<Assets<Font>>()
            .get(&font_handle)
            .expect("The font asset was just added above.")
            .alias
            .clone();
        assert_eq!(font_alias, format!("asset_id:{:?}", font_handle.id()));
        assert!(world
            .resource_mut::<FontCx>()
            .collection
            .family_id("Fira Mono")
            .is_some());
        assert!(world
            .resource_mut::<FontCx>()
            .collection
            .family_id(&font_alias)
            .is_some());

        world
            .resource_mut::<Assets<Font>>()
            .remove(font_handle.id());

        app.update();
        let world = app.world_mut();

        assert!(world
            .resource_mut::<FontCx>()
            .collection
            .family_id(&font_alias)
            .is_none());
    }
}
