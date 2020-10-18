use super::{Texture, TextureFormat};
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_math::Vec2;

/// Loads HDR textures as Texture assets
#[derive(Clone, Default)]
pub struct HdrTextureLoader;

impl AssetLoader for HdrTextureLoader {
    fn load(&self, bytes: &[u8], load_context: &mut LoadContext) -> Result<()> {
        let format = TextureFormat::Rgba32Float;
        debug_assert_eq!(
            format.pixel_size(),
            4 * 4,
            "Format should have 32bit x 4 size"
        );

        let decoder = image::hdr::HdrDecoder::new(bytes)?;
        let info = decoder.metadata();
        let rgb_data = decoder.read_image_hdr()?;
        let mut rgba_data = Vec::with_capacity(rgb_data.len() * format.pixel_size());

        for rgb in rgb_data {
            let alpha = 1.0f32;

            rgba_data.extend_from_slice(&rgb.0[0].to_ne_bytes());
            rgba_data.extend_from_slice(&rgb.0[1].to_ne_bytes());
            rgba_data.extend_from_slice(&rgb.0[2].to_ne_bytes());
            rgba_data.extend_from_slice(&alpha.to_ne_bytes());
        }

        let texture = Texture::new(
            Vec2::new(info.width as f32, info.height as f32),
            rgba_data,
            format,
        );

        load_context.set_default_asset(LoadedAsset::new(texture));
        Ok(())
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["hdr"];
        EXTENSIONS
    }
}
