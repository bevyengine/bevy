use bevy_asset::Asset;
use bevy_reflect::TypePath;

/// An [`Asset`](bevy_asset::Asset) that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
#[derive(Debug, TypePath, Clone, Asset)]
pub struct Font {
    pub data: std::sync::Arc<Vec<u8>>,
}

impl Font {
    pub fn from_bytes(font_data: Vec<u8>) -> Self {
        // TODO: validate font, restore `try_from_bytes`
        Self {
            data: std::sync::Arc::new(font_data),
        }
    }
}
