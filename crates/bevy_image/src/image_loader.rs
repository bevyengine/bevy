use crate::{
    image::{Image, ImageFormat, ImageType, TextureError},
    TextureReinterpretationError,
};
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

/// How to interpret the image as an array of textures.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ImageArrayLayout {
    /// Interpret the image as a vertical stack of *n* images.
    RowCount { rows: u32 },
    /// Interpret the image as a vertical stack of images, each *n* pixels tall.
    RowHeight { pixels: u32 },
}

/// Settings for loading an [`Image`] using an [`ImageLoader`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageLoaderSettings {
    /// How to determine the image's container format.
    pub format: ImageFormatSetting,
    /// Forcibly use a specific [`wgpu_types::TextureFormat`].
    /// Useful to control how data is handled when used
    /// in a shader.
    /// Ex: data that would be `R16Uint` that needs to
    /// be sampled as a float using `R16Snorm`.
    #[serde(skip)]
    pub texture_format: Option<wgpu_types::TextureFormat>,
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
    /// Interpret the image as an array of images. This is
    /// primarily for use with the `texture2DArray` shader
    /// uniform type.
    #[serde(default)]
    pub array_layout: Option<ImageArrayLayout>,
}

impl Default for ImageLoaderSettings {
    fn default() -> Self {
        Self {
            format: ImageFormatSetting::default(),
            texture_format: None,
            is_srgb: true,
            sampler: ImageSampler::Default,
            asset_usage: RenderAssetUsages::default(),
            array_layout: None,
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
    /// An error occurred while trying to interpret the image bytes as an array texture.
    #[error("Invalid array layout: {0}")]
    ArrayLayout(#[from] TextureReinterpretationError),
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
                let ext = load_context
                    .path()
                    .path()
                    .extension()
                    .unwrap()
                    .to_str()
                    .unwrap();
                ImageType::Extension(ext)
            }
            ImageFormatSetting::Format(format) => ImageType::Format(format),
            ImageFormatSetting::Guess => {
                let format = image::guess_format(&bytes).map_err(|err| FileTextureError {
                    error: err.into(),
                    path: format!("{}", load_context.path().path().display()),
                })?;
                ImageType::Format(ImageFormat::from_image_crate_format(format).ok_or_else(
                    || FileTextureError {
                        error: TextureError::UnsupportedTextureFormat(format!("{format:?}")),
                        path: format!("{}", load_context.path().path().display()),
                    },
                )?)
            }
        };

        let mut image = Image::from_buffer(
            &bytes,
            image_type,
            self.supported_compressed_formats,
            settings.is_srgb,
            settings.sampler.clone(),
            settings.asset_usage,
        )
        .map_err(|err| FileTextureError {
            error: err,
            path: format!("{}", load_context.path().path().display()),
        })?;

        if let Some(format) = settings.texture_format {
            image.texture_descriptor.format = format;
        }

        if let Some(array_layout) = settings.array_layout {
            let layers = match array_layout {
                ImageArrayLayout::RowCount { rows } => rows,
                ImageArrayLayout::RowHeight { pixels } => image.height() / pixels,
            };

            image.reinterpret_stacked_2d_as_array(layers)?;
        }

        Ok(image)
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
