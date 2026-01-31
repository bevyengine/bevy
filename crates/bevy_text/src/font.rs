use alloc::sync::Arc;

use bevy_asset::Asset;
use bevy_asset::AssetEvent;
use bevy_asset::Assets;
use bevy_ecs::message::MessageReader;
use bevy_ecs::system::Query;
use bevy_ecs::system::ResMut;
use bevy_reflect::TypePath;
use cosmic_text::fontdb::ID;
use cosmic_text::skrifa::raw::ReadError;
use cosmic_text::skrifa::FontRef;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::ComputedTextBlock;
use crate::CosmicFontSystem;

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
    pub data: Arc<Vec<u8>>,
    /// Ids for fonts in font file
    pub ids: SmallVec<[ID; 8]>,
    /// Font family name.
    /// If the font file is a collection with multiple families, the first family name from the last font is used.
    pub family_name: SmolStr,
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn try_from_bytes(font_data: Vec<u8>) -> Result<Self, ReadError> {
        let _ = FontRef::from_index(&font_data, 0)?;
        Ok(Self {
            data: Arc::new(font_data),
            ids: SmallVec::new(),
            family_name: SmolStr::default(),
        })
    }
}

/// Add new font assets to the font system's database.
pub fn load_font_assets_into_fontdb_system(
    mut fonts: ResMut<Assets<Font>>,
    mut events: MessageReader<AssetEvent<Font>>,
    mut cosmic_font_system: ResMut<CosmicFontSystem>,
    mut text_block_query: Query<&mut ComputedTextBlock>,
) {
    let mut new_fonts_added = false;
    let font_system = &mut cosmic_font_system.0;
    for event in events.read() {
        if let AssetEvent::Added { id } = event
            && let Some(mut font) = fonts.get_mut(*id)
        {
            let data = Arc::clone(&font.data);
            font.ids = font_system
                .db_mut()
                .load_font_source(cosmic_text::fontdb::Source::Binary(data))
                .into_iter()
                .collect();
            // TODO: it is assumed this is the right font face
            font.family_name = font_system
                .db()
                .face(*font.ids.last().unwrap())
                .unwrap()
                .families[0]
                .0
                .as_str()
                .into();
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
