use cosmic_text::CacheKey;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
/// Errors related to the textsystem
pub enum TextError {
    /// Font was not found, this could be that the font has not yet been loaded, or
    /// that the font failed to load for some other reason
    #[error("font not found")]
    NoSuchFont,
    /// Failed to add glyph to a newly created atlas for some reason
    #[error("failed to add glyph to newly-created atlas {0:?}")]
    FailedToAddGlyph(u16),
    /// Failed to get scaled glyph image for cache key
    #[error("failed to get scaled glyph image for cache key: {0:?}")]
    FailedToGetGlyphImage(CacheKey),
    /// Missing texture atlas layout for the font
    #[error("missing texture atlas layout for the font")]
    MissingAtlasLayout,
    /// Missing texture for the font atlas
    #[error("missing texture for the font atlas")]
    MissingAtlasTexture,
    /// Failed to find glyph in atlas after it was added
    #[error("failed to find glyph in atlas after it was added")]
    InconsistentAtlasState,
    #[error("scale factor <= 0")]
    /// Text cannot be rendered for a scale factor <= zero.
    DegenerateScaleFactor,
}
