use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum TextError {
    #[error("Font not found")]
    NoSuchFont,
}
