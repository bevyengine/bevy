use crate::Font;
use bevy_asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext};
use bevy_utils::{ConditionalSend, ConditionalSendFuture};
use thiserror::Error;

#[derive(Default)]
pub struct FontLoader;

/// Possible errors that can be produced by [`FontLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum FontLoaderError {
    /// An [IO](std::io) Error
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl AssetLoader for FontLoader {
    type Asset = Font;
    type Settings = ();
    type Error = FontLoaderError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Font, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let font = Font::from_bytes(bytes);
        // load_context.set_default_asset(LoadedAsset::new(font));
        Ok(font)
    }

    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }
}
