use crate::Font;
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_utils::BoxedFuture;

#[derive(Default)]
pub struct FontLoader;

impl AssetLoader for FontLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let font = Font::try_from_bytes(bytes.into())?;
            load_context.set_default_asset(LoadedAsset::new(font));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }
}
