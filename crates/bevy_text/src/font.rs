use crate::font;
use crate::FontCx;
use crate::FontSource;
use crate::FontStyle;
use crate::FontWeight;
use crate::FontWidth;
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
use smol_str::SmolStr;

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
    /// Font family name used to resolve this asset when referenced by handle.
    /// If the font file is a collection with multiple families, this is the family name from the
    /// first font face in the collection.
    pub family_name: SmolStr,
    /// Font weight used when the font is referenced by handle.
    pub weight: FontWeight,
    /// Font width used when the font is referenced by handle.
    pub width: FontWidth,
    /// Font style used when the font is referenced by handle.
    pub style: FontStyle,
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn from_bytes(font_data: Vec<u8>, family_name: &str) -> Font {
        Self {
            data: Blob::from(font_data),
            family_name: family_name.into(),
            weight: FontWeight::default(),
            width: FontWidth::default(),
            style: FontStyle::default(),
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
    let rebuild = loaded_fonts.iter().any(|id| !fonts.contains(*id));
    loaded_fonts.retain(|id| fonts.contains(*id));

    let new_asset_ids: Vec<_> = if rebuild {
        font_cx.0.collection.clear();

        loaded_fonts.iter().cloned().collect()
    } else {
        fonts.ids().filter(|id| loaded_fonts.insert(*id)).collect()
    };

    if new_asset_ids.is_empty() {
        return;
    }

    let mut new_family_ids = Vec::new();
    for asset_id in new_asset_ids.iter() {
        let font_data = fonts
            .get(*asset_id)
            .expect("AssetId should have a corresponding asset")
            .data
            .clone();

        let new_fonts = font_cx.collection.register_fonts(font_data, None);

        if let Some((family_id, font_info)) = new_fonts
            .iter()
            .flat_map(|(family_id, fonts)| {
                fonts.iter().map(move |font_info| (*family_id, font_info))
            })
            .min_by_key(|(_, font_info)| font_info.index())
            && let Some(family_name) = font_cx.collection.family_name(family_id)
            && let Some(font) = fonts.get_mut_untracked(*asset_id)
        {
            font.family_name = family_name.into();
            font.weight = FontWeight(font_info.weight().value().round().clamp(1.0, 1000.0) as u16);
            font.width = FontWidth(font_info.width().ratio());
            font.style = match font_info.style() {
                parley::FontStyle::Normal => FontStyle::Normal,
                parley::FontStyle::Italic => FontStyle::Italic,
                parley::FontStyle::Oblique(angle) => FontStyle::Oblique(angle),
            };

            new_family_ids.extend(new_fonts.iter().map(|(family_id, _)| *family_id));
        }
    }

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
