use crate::Font;
use anyhow::Result;
use bevy_asset::{AssetLoader, AssetPath};

#[derive(Clone)]
pub struct FontLoader;

impl AssetLoader<Font> for FontLoader {
    fn from_bytes(&self, _asset_path: &AssetPath, bytes: Vec<u8>) -> Result<Font> {
        Ok(Font::try_from_bytes(bytes)?)
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["ttf"];
        EXTENSIONS
    }
}
