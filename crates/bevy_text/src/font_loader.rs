use crate::Font;
use anyhow::Result;
use bevy_asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext};

#[derive(Default)]
pub struct FontLoader;

impl AssetLoader for FontLoader {
    type Asset = Font;
    type Settings = ();
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Font, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let font = Font::try_from_bytes(bytes)?;
            Ok(font)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }
}
