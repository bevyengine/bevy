use crate::texture::{Image, TextureFormatPixelInfo};
use bevy_asset::{
    anyhow::Error,
    io::{AsyncReadExt, Reader},
    AssetLoader, LoadContext,
};
use bevy_utils::BoxedFuture;
use image::ImageDecoder;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

/// Loads EXR textures as Texture assets
#[derive(Clone, Default)]
pub struct ExrTextureLoader;

impl AssetLoader for ExrTextureLoader {
    type Asset = Image;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Image, Error>> {
        Box::pin(async move {
            let format = TextureFormat::Rgba32Float;
            debug_assert_eq!(
                format.pixel_size(),
                4 * 4,
                "Format should have 32bit x 4 size"
            );

            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let decoder = image::codecs::openexr::OpenExrDecoder::with_alpha_preference(
                std::io::Cursor::new(bytes),
                Some(true),
            )?;
            let (width, height) = decoder.dimensions();

            let total_bytes = decoder.total_bytes() as usize;

            let mut buf = vec![0u8; total_bytes];
            decoder.read_image(buf.as_mut_slice())?;

            Ok(Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                buf,
                format,
            ))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["exr"]
    }
}
