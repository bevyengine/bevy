use cosmic_text::CacheKey;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum TextError {
    #[error("font not found")]
    NoSuchFont,
    #[error("failed to add glyph to newly-created atlas {0:?}")]
    FailedToAddGlyph(u16),
    #[error("font system mutex could not be acquired or is poisoned")]
    FailedToAcquireMutex,
    #[error("failed to get scaled glyph image for cache key: {0:?}")]
    FailedToGetGlyphImage(CacheKey),
}
