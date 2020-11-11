use ab_glyph::GlyphId;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum TextError {
    #[error("Font not found")]
    NoSuchFont,
    #[error("Failed to add glyph to newly-created atlas {0:?}")]
    FailedToAddGlyph(GlyphId),
}
