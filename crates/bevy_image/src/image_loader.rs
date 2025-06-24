use crate::image::{Image, ImageFormat, ImageType, TextureError};
use bevy_asset::{io::Reader, AssetLoader, LoadContext, RenderAssetUsages};
use thiserror::Error;

use super::{CompressedImageFormats, ImageSampler};
use serde::{Deserialize, Serialize};

/// Loader for images that can be read by the `image` crate.
#[derive(Clone)]
pub struct ImageLoader {
    supported_compressed_formats: CompressedImageFormats,
}

impl ImageLoader {
    /// Full list of supported formats.
    pub const SUPPORTED_FORMATS: &'static [ImageFormat] = &[
        #[cfg(feature = "basis-universal")]
        ImageFormat::Basis,
        #[cfg(feature = "bmp")]
        ImageFormat::Bmp,
        #[cfg(feature = "dds")]
        ImageFormat::Dds,
        #[cfg(feature = "ff")]
        ImageFormat::Farbfeld,
        #[cfg(feature = "gif")]
        ImageFormat::Gif,
        #[cfg(feature = "ico")]
        ImageFormat::Ico,
        #[cfg(feature = "jpeg")]
        ImageFormat::Jpeg,
        #[cfg(feature = "ktx2")]
        ImageFormat::Ktx2,
        #[cfg(feature = "png")]
        ImageFormat::Png,
        #[cfg(feature = "pnm")]
        ImageFormat::Pnm,
        #[cfg(feature = "qoi")]
        ImageFormat::Qoi,
        #[cfg(feature = "tga")]
        ImageFormat::Tga,
        #[cfg(feature = "tiff")]
        ImageFormat::Tiff,
        #[cfg(feature = "webp")]
        ImageFormat::WebP,
    ];

    /// Total count of file extensions, for computing supported file extensions list.
    const COUNT_FILE_EXTENSIONS: usize = {
        let mut count = 0;
        let mut idx = 0;
        while idx < Self::SUPPORTED_FORMATS.len() {
            count += Self::SUPPORTED_FORMATS[idx].to_file_extensions().len();
            idx += 1;
        }
        count
    };

    /// Gets the list of file extensions for all formats.
    pub const SUPPORTED_FILE_EXTENSIONS: &'static [&'static str] = &{
        let mut exts = [""; Self::COUNT_FILE_EXTENSIONS];
        let mut ext_idx = 0;
        let mut fmt_idx = 0;
        while fmt_idx < Self::SUPPORTED_FORMATS.len() {
            let mut off = 0;
            let fmt_exts = Self::SUPPORTED_FORMATS[fmt_idx].to_file_extensions();
            while off < fmt_exts.len() {
                exts[ext_idx] = fmt_exts[off];
                off += 1;
                ext_idx += 1;
            }
            fmt_idx += 1;
        }
        exts
    };

    /// Creates a new image loader that supports the provided formats.
    pub fn new(supported_compressed_formats: CompressedImageFormats) -> Self {
        Self {
            supported_compressed_formats,
        }
    }
}

/// How to determine an image's format when loading.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum ImageFormatSetting {
    /// Determine the image format from its file extension.
    ///
    /// This is the default.
    #[default]
    FromExtension,
    /// Declare the image format explicitly.
    Format(ImageFormat),
    /// Guess the image format by looking for magic bytes at the
    /// beginning of its data.
    Guess,
}

/// Settings for loading an [`Image`] using an [`ImageLoader`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageLoaderSettings {
    /// How to determine the image's format.
    pub format: ImageFormatSetting,
    /// Specifies whether image data is linear
    /// or in sRGB space when this is not determined by
    /// the image format.
    pub is_srgb: bool,
    /// [`ImageSampler`] to use when rendering - this does
    /// not affect the loading of the image data.
    pub sampler: ImageSampler,
    /// Where the asset will be used - see the docs on
    /// [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
}

impl Default for ImageLoaderSettings {
    fn default() -> Self {
        Self {
            format: ImageFormatSetting::default(),
            is_srgb: true,
            sampler: ImageSampler::Default,
            asset_usage: RenderAssetUsages::default(),
        }
    }
}

/// An error when loading an image using [`ImageLoader`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ImageLoaderError {
    /// An error occurred while trying to load the image bytes.
    #[error("Failed to load image bytes: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred while trying to decode the image bytes.
    #[error("Could not load texture file: {0}")]
    FileTexture(#[from] FileTextureError),
}

impl AssetLoader for ImageLoader {
    type Asset = Image;
    type Settings = ImageLoaderSettings;
    type Error = ImageLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &ImageLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let image_type = match settings.format {
            ImageFormatSetting::FromExtension => {
                // use the file extension for the image type
                let ext = load_context.path().extension().unwrap().to_str().unwrap();
                ImageType::Extension(ext)
            }
            ImageFormatSetting::Format(format) => ImageType::Format(format),
            ImageFormatSetting::Guess => {
                let format = image::guess_format(&bytes).map_err(|err| FileTextureError {
                    error: err.into(),
                    path: format!("{}", load_context.path().display()),
                })?;
                ImageType::Format(ImageFormat::from_image_crate_format(format).ok_or_else(
                    || FileTextureError {
                        error: TextureError::UnsupportedTextureFormat(format!("{format:?}")),
                        path: format!("{}", load_context.path().display()),
                    },
                )?)
            }
        };
        Ok(Image::from_buffer(
            &bytes,
            image_type,
            self.supported_compressed_formats,
            settings.is_srgb,
            settings.sampler.clone(),
            settings.asset_usage,
        )
        .map_err(|err| FileTextureError {
            error: err,
            path: format!("{}", load_context.path().display()),
        })?)
    }

    fn extensions(&self) -> &[&str] {
        Self::SUPPORTED_FILE_EXTENSIONS
    }
}

/// An error that occurs when loading a texture from a file.
#[derive(Error, Debug)]
#[error("Error reading image file {path}: {error}.")]
pub struct FileTextureError {
    error: TextureError,
    path: String,
}
