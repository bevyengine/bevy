use crate::texture::{Image, TextureFormatPixelInfo};
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_utils::BoxedFuture;
use image::ImageDecoder;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

/// Loads EXR textures as Texture assets
#[derive(Clone, Default)]
pub struct ExrTextureLoader;

impl AssetLoader for ExrTextureLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let format = TextureFormat::Rgba32Float;
            debug_assert_eq!(
                format.pixel_size(),
                4 * 4,
                "Format should have 32bit x 4 size"
            );

            let decoder = image::codecs::openexr::OpenExrDecoder::with_alpha_preference(
                std::io::Cursor::new(bytes),
                Some(true),
            )?;
            let (width, height) = decoder.dimensions();

            let total_bytes = decoder.total_bytes() as usize;

            let mut buf = vec![0u8; total_bytes];
            decoder.read_image(buf.as_mut_slice())?;

            let texture = Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                buf,
                format,
            );

            load_context.set_default_asset(LoadedAsset::new(texture));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["exr"]
    }
}
