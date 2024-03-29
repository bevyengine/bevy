use crate::{
    render_asset::RenderAssetUsages,
    texture::{Image, TextureFormatPixelInfo},
};
use bevy_asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

/// Loads HDR textures as Texture assets
#[derive(Clone, Default)]
pub struct HdrTextureLoader;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HdrTextureLoaderSettings {
    pub asset_usage: RenderAssetUsages,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum HdrTextureLoaderError {
    #[error("Could load texture: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not extract image: {0}")]
    Image(#[from] image::ImageError),
}

impl AssetLoader for HdrTextureLoader {
    type Asset = Image;
    type Settings = HdrTextureLoaderSettings;
    type Error = HdrTextureLoaderError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let format = TextureFormat::Rgba32Float;
        debug_assert_eq!(
            format.pixel_size(),
            4 * 4,
            "Format should have 32bit x 4 size"
        );

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let decoder = image::codecs::hdr::HdrDecoder::new(bytes.as_slice())?;
        let info = decoder.metadata();
        let dynamic_image = DynamicImage::from_decoder(decoder)?;
        let image_buffer = dynamic_image
            .as_rgb32f()
            .expect("HDR Image format should be Rgb32F");
        let mut rgba_data = Vec::with_capacity(image_buffer.pixels().len() * format.pixel_size());

        for rgb in image_buffer.pixels() {
            let alpha = 1.0f32;

            rgba_data.extend_from_slice(&rgb.0[0].to_ne_bytes());
            rgba_data.extend_from_slice(&rgb.0[1].to_ne_bytes());
            rgba_data.extend_from_slice(&rgb.0[2].to_ne_bytes());
            rgba_data.extend_from_slice(&alpha.to_ne_bytes());
        }

        Ok(Image::new(
            Extent3d {
                width: info.width,
                height: info.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba_data,
            format,
            settings.asset_usage,
        ))
    }

    fn extensions(&self) -> &[&str] {
        &["hdr"]
    }
}
