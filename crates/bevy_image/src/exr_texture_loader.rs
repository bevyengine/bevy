use crate::{Image, TextureFormatPixelInfo};
use bevy_asset::{io::Reader, AssetLoader, LoadContext, RenderAssetUsages};
use bevy_reflect::TypePath;
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

/// Loads EXR textures as Texture assets
#[derive(Clone, Default, TypePath)]
#[cfg(feature = "exr")]
pub struct ExrTextureLoader;

/// Settings for [`ExrTextureLoader`].
#[derive(Serialize, Deserialize, Default, Debug)]
#[cfg(feature = "exr")]
pub struct ExrTextureLoaderSettings {
    /// Where the asset will be used - see the docs on [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
}

/// Possible errors that can be produced by [`ExrTextureLoader`]
#[non_exhaustive]
#[derive(Debug, Error, TypePath)]
#[cfg(feature = "exr")]
pub enum ExrTextureLoaderError {
    /// I/O Error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Failed to decode the texture.
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
}

impl AssetLoader for ExrTextureLoader {
    type Asset = Image;
    type Settings = ExrTextureLoaderSettings;
    type Error = ExrTextureLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let format = TextureFormat::Rgba32Float;
        debug_assert_eq!(
            // `Rgba32Float` will always return a valid pixel size
            format.pixel_size().unwrap(),
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
            settings.asset_usage,
        ))
    }

    fn extensions(&self) -> &[&str] {
        &["exr"]
    }
}
