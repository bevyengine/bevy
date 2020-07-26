use super::{Texture, TextureFormat};
use anyhow::Result;
use bevy_asset::AssetLoader;
use bevy_math::Vec2;
use std::path::Path;

#[derive(Clone, Default)]
pub struct PngTextureLoader;

impl AssetLoader<Texture> for PngTextureLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Texture> {
        let decoder = png::Decoder::new(bytes.as_slice());
        let (info, mut reader) = decoder.read_info()?;
        let mut data = vec![0; info.buffer_size()];
        reader.next_frame(&mut data)?;
        Ok(Texture::new(
            Vec2::new(info.width as f32, info.height as f32),
            data,
            TextureFormat::Rgba8UnormSrgb,
        ))
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
