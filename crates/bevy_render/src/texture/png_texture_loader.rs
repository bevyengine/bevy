use bevy_asset::{AssetPath, AssetLoader};
use super::Texture;
use anyhow::Result;

#[derive(Clone, Default)]
pub struct PngTextureLoader;

impl AssetLoader<Texture> for PngTextureLoader {
    fn from_bytes(&self, _asset_path: &AssetPath, bytes: Vec<u8>) -> Result<Texture> {
        let decoder = png::Decoder::new(bytes.as_slice());
        let (info, mut reader) = decoder.read_info()?;
        let mut data = vec![0; info.buffer_size()];
        reader.next_frame(&mut data)?;
        Ok(Texture {
            data,
            width: info.width as usize,
            height: info.height as usize,
        })
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &[
            "png"
        ];
        EXTENSIONS
    }
}