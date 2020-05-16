use crate::render::render_text;
use bevy_render::{texture::Texture, Color};
use font_kit::{error::FontLoadingError, metrics::Metrics};
use skribo::{FontCollection, FontFamily};
use std::sync::Arc;

pub struct Font {
    pub collection: FontCollection,
    pub metrics: Metrics,
}

unsafe impl Send for Font {}
unsafe impl Sync for Font {}

impl Font {
    pub fn try_from_bytes(font_data: Vec<u8>) -> Result<Self, FontLoadingError> {
        let font = font_kit::font::Font::from_bytes(Arc::new(font_data), 0)?;
        let metrics = font.metrics();
        let mut collection = FontCollection::new();
        collection.add_family(FontFamily::new_from_font(font));
        Ok(Font {
            collection,
            metrics,
        })
    }

    pub fn render_text(&self, text: &str, color: Color, width: usize, height: usize) -> Texture {
        render_text(self, text, color, width, height)
    }
}
