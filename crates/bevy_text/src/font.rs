use bevy_asset::Asset;
use bevy_reflect::TypePath;
use parley::fontique::Blob;
use parley::fontique::FamilyId;
use parley::fontique::FontInfo;
use parley::FontContext;

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
    collection: Vec<(FamilyId, Vec<FontInfo>)>,
}

pub struct NoFontsFoundError;

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn try_from_bytes(
        font_cx: &mut FontContext,
        font_data: Vec<u8>,
    ) -> Result<Font, NoFontsFoundError> {
        let collection = font_cx
            .collection
            .register_fonts(Blob::from(font_data), None);
        if collection.is_empty() {
            Ok(Font { collection })
        } else {
            Err(NoFontsFoundError)
        }
    }
}
