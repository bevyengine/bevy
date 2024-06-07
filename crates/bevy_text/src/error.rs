use cosmic_text::CacheKey;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
/// Errors related to the textsystem
pub enum TextError {
    /// Font was not found, this could be that the font has not yet been loaded, or
    /// that the font failed to load for some reason
    #[error("font not found")]
    NoSuchFont,
    /// Failed to add glyph to a newly created atlas for some reason
    #[error("failed to add glyph to newly-created atlas {0:?}")]
    FailedToAddGlyph(u16),
    /// Failed to acquire mutex to cosmic-texts fontsystem
    //TODO: this can be removed since the mutex should be possible to remove as well
    #[error("font system mutex could not be acquired or is poisoned")]
    FailedToAcquireMutex,
    /// Failed to get scaled glyph image for cache key
    #[error("failed to get scaled glyph image for cache key: {0:?}")]
    FailedToGetGlyphImage(CacheKey),
}
