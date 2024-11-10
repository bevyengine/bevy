use cosmic_text::CacheKey;
use derive_more::derive::{Display, Error};

#[derive(Debug, PartialEq, Eq, Error, Display)]
/// Errors related to the textsystem
pub enum TextError {
    /// Font was not found, this could be that the font has not yet been loaded, or
    /// that the font failed to load for some other reason
    #[display("font not found")]
    NoSuchFont,
    /// Failed to add glyph to a newly created atlas for some reason
    #[display("failed to add glyph to newly-created atlas {_0:?}")]
    #[error(ignore)]
    FailedToAddGlyph(u16),
    /// Failed to get scaled glyph image for cache key
    #[display("failed to get scaled glyph image for cache key: {_0:?}")]
    #[error(ignore)]
    FailedToGetGlyphImage(CacheKey),
}
