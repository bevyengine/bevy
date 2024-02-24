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
    pub fn from_bytes(font_data: Vec<u8>) -> Self {
        // TODO: validate font, restore `try_from_bytes`
        Self {
            data: Arc::new(font_data),
        }
    }
}
