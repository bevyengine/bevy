use crate::Font;
use bevy_asset::{io::Reader, AssetLoader, LoadContext};
use bevy_reflect::TypePath;
use thiserror::Error;

#[derive(Default, TypePath)]
/// An [`AssetLoader`] for [`Font`]s, for use by the [`AssetServer`](bevy_asset::AssetServer)
pub struct FontLoader;

/// Possible errors that can be produced by [`FontLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum FontLoaderError {
    /// The contents that could not be parsed
    #[error("Failed to parse font.")]
    Content,
    /// An [IO](std::io) Error
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl AssetLoader for FontLoader {
    type Asset = Font;
    type Settings = ();
    type Error = FontLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Font, Self::Error> {
        let path = load_context.path();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let font = Font::try_from_bytes(bytes, &path.to_string());
        Ok(font)
    }

    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }
}
