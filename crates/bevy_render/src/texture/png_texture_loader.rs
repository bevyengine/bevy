use super::Texture;
use anyhow::Result;
use bevy_asset::AssetLoader;
use std::path::Path;
use glam::Vec2;

#[derive(Clone, Default)]
pub struct PngTextureLoader;

impl AssetLoader<Texture> for PngTextureLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Texture> {
        let decoder = png::Decoder::new(bytes.as_slice());
        let (info, mut reader) = decoder.read_info()?;
        let mut data = vec![0; info.buffer_size()];
        reader.next_frame(&mut data)?;
        Ok(Texture {
            data,
            size: Vec2::new(info.width as f32, info.height as f32),
        })
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
