use bevy_reflect::{TypePath, TypeUuid};

#[derive(Debug, TypeUuid, TypePath, Clone)]
#[uuid = "97059ac6-c9ba-4da9-95b6-bed82c3ce198"]

/// An asset that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
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
