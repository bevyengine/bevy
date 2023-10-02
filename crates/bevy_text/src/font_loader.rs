use crate::Font;
use bevy_asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext};
use thiserror::Error;

#[derive(Default)]
pub struct FontLoader;

/// Possible errors that can be produced by [`FontLoader`]
#[derive(Debug, Error)]
pub enum FontLoaderError {
    /// An [IO](std::io) Error
    #[error(transparent)]
    IO(#[from] std::io::Error),
    /// An [InvalidFont](ab_glyph::InvalidFont) Error
    #[error(transparent)]
    FontInvalid(#[from] ab_glyph::InvalidFont),
}

impl AssetLoader for FontLoader {
    type Asset = Font;
    type Settings = ();
    type Error = FontLoaderError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Font, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            Ok(Font::try_from_bytes(bytes)?)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }
}
