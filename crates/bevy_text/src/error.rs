#[derive(Debug, PartialEq, Eq)]
pub enum TextError {
    NoSuchFont,
    FailedToOutlineGlyph,
}
