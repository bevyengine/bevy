use std::sync::Arc;

use bevy_asset::Asset;
use bevy_reflect::TypePath;

/// An [`Asset`] that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
#[derive(Debug, TypePath, Clone, Asset)]
pub struct Font {
    /// Content of a font file as bytes
    pub data: Arc<Vec<u8>>,
}

impl Font {
    /// Creates a [Font] from bytes, without any validation of the content
    pub fn try_from_bytes(
        font_data: Vec<u8>,
    ) -> Result<Self, cosmic_text::ttf_parser::FaceParsingError> {
        // TODO: validate font, restore `try_from_bytes`
        use cosmic_text::ttf_parser;
        ttf_parser::Face::parse(&font_data, 0)?;
        Ok(Self {
            data: Arc::new(font_data),
        })
    }
}
