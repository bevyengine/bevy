use crate::ComputedTextBlock;
use crate::FontCx;
use bevy_asset::Asset;
use bevy_asset::AssetEvent;
use bevy_asset::Assets;
use bevy_ecs::message::MessageReader;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_reflect::TypePath;
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;
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
    /// Font family name.
    /// If the font file is a collection with multiple families, the first family name from the last font is used.
    pub family_name: SmolStr,
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn try_from_bytes(font_data: Vec<u8>, family_name: &str) -> Font {
        Self {
            data: Blob::from(font_data),
            family_name: family_name.into(),
        }
    }
}

/// Add new font assets to the internal font collection.
pub fn load_font_assets_into_font_collection(
    fonts: Res<Assets<Font>>,
    mut events: MessageReader<AssetEvent<Font>>,
    mut font_cx: ResMut<FontCx>,
    mut text_block_query: Query<&mut ComputedTextBlock>,
) {
    let mut new_fonts_added = false;

    for event in events.read() {
        if let AssetEvent::Added { id } = event
            && let Some(font) = fonts.get(*id)
        {
            font_cx.0.collection.register_fonts(
                font.data.clone(),
                Some(FontInfoOverride {
                    family_name: Some(font.family_name.as_str()),
                    ..Default::default()
                }),
            );
            new_fonts_added = true;
        }
    }

    // Whenever new fonts are added, update all text blocks so they use the new fonts.
    if new_fonts_added {
        for mut block in text_block_query.iter_mut() {
            block.needs_rerender = true;
        }
    }
}
