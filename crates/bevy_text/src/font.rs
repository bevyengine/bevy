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
use core::fmt::Write as _;
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;

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
    /// Alias used to identify the asset in the  when referenced by handle.
    pub alias: String,
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn from_bytes(font_data: Vec<u8>, alias: &str) -> Font {
        Self {
            data: Blob::from(font_data),
            alias: alias.to_string(),
        }
    }
}

/// Add new font assets to the internal font collection.
pub fn load_font_assets_into_font_collection(
    mut fonts: ResMut<Assets<Font>>,
    mut loaded_fonts: Local<HashSet<AssetId<Font>>>,
    mut font_cx: ResMut<FontCx>,
    mut text_font_query: Query<&mut TextFont>,
) {
    let new_asset_ids: Vec<_> = if loaded_fonts.iter().any(|id| !fonts.contains(*id)) {
        // If any font asset has been removed, clear the font collection and queue the remaining fonts to be reinserted into the collection.
        font_cx.collection.clear();
        loaded_fonts.clear();
        loaded_fonts.extend(fonts.ids());
        loaded_fonts.iter().copied().collect()
    } else {
        fonts.ids().filter(|id| loaded_fonts.insert(*id)).collect()
    };

    if new_asset_ids.is_empty() {
        return;
    }

    let mut new_family_ids = Vec::new();
    for asset_id in &new_asset_ids {
        let font = fonts
            .get_mut_untracked(*asset_id)
            .expect("Each AssetId should have a corresponding asset");

        write!(&mut font.alias, ":{asset_id:?}").unwrap();

        // Each font is registered twice in Parley's FontContext collection, once under its embedded family name,
        // and once under an alias generated from the asset path and id.
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

    font_cx.restore_generic_families();

    for mut text_font in text_font_query.iter_mut() {
        if match &text_font.font {
            FontSource::Handle(handle) => new_asset_ids.contains(&handle.id()),
            FontSource::Family(name) => font_cx
                .collection
                .family_id(name)
                .is_some_and(|id| new_family_ids.contains(&id)),
            generic_source => {
                let generic_family = match generic_source {
                    FontSource::Handle(_) | FontSource::Family(_) => unreachable!(),
                    FontSource::Serif => parley::GenericFamily::Serif,
                    FontSource::SansSerif => parley::GenericFamily::SansSerif,
                    FontSource::Cursive => parley::GenericFamily::Cursive,
                    FontSource::Fantasy => parley::GenericFamily::Fantasy,
                    FontSource::Monospace => parley::GenericFamily::Monospace,
                    FontSource::SystemUi => parley::GenericFamily::SystemUi,
                    FontSource::UiSerif => parley::GenericFamily::UiSerif,
                    FontSource::UiSansSerif => parley::GenericFamily::UiSansSerif,
                    FontSource::UiMonospace => parley::GenericFamily::UiMonospace,
                    FontSource::UiRounded => parley::GenericFamily::UiRounded,
                    FontSource::Emoji => parley::GenericFamily::Emoji,
                    FontSource::Math => parley::GenericFamily::Math,
                    FontSource::FangSong => parley::GenericFamily::FangSong,
                };
                font_cx
                    .collection
                    .generic_families(generic_family)
                    .any(|id| new_family_ids.contains(&id))
            }
        } {
            text_font.set_changed();
        }
    }
}
