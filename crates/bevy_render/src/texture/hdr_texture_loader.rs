use crate::texture::{Image, TextureSizeInfo};
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_utils::BoxedFuture;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

/// Loads HDR textures as Texture assets
#[derive(Clone, Default)]
pub struct HdrTextureLoader;

impl AssetLoader for HdrTextureLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let format = TextureFormat::Rgba32Float;

            let decoder = image::codecs::hdr::HdrDecoder::new(bytes)?;
            let info = decoder.metadata();
            let rgb_data = decoder.read_image_hdr()?;

            let size_in_pixels = Extent3d {
                width: info.width,
                height: info.height,
                depth_or_array_layers: 1,
            };
            let mut rgba_data = Vec::with_capacity(format.texture_size(size_in_pixels).in_bytes());

            for rgb in rgb_data {
                let alpha = 1.0f32;

                rgba_data.extend_from_slice(&rgb.0[0].to_ne_bytes());
                rgba_data.extend_from_slice(&rgb.0[1].to_ne_bytes());
                rgba_data.extend_from_slice(&rgb.0[2].to_ne_bytes());
                rgba_data.extend_from_slice(&alpha.to_ne_bytes());
            }

            let texture = Image::new(size_in_pixels, TextureDimension::D2, rgba_data, format);

            load_context.set_default_asset(LoadedAsset::new(texture));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["hdr"]
    }
}
