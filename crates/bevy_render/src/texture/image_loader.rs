use bevy_asset::{io::Reader, AssetLoader, LoadContext};
use bevy_ecs::prelude::{FromWorld, World};
use derive_more::derive::{Display, Error, From};

use crate::{
    render_asset::RenderAssetUsages,
    renderer::RenderDevice,
    texture::{Image, ImageFormat, ImageType, TextureError},
};

use super::{CompressedImageFormats, ImageSampler};
use serde::{Deserialize, Serialize};

/// Loader for images that can be read by the `image` crate.
#[derive(Clone)]
pub struct ImageLoader {
    supported_compressed_formats: CompressedImageFormats,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub enum ImageFormatSetting {
    #[default]
    FromExtension,
    Format(ImageFormat),
    Guess,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageLoaderSettings {
    pub format: ImageFormatSetting,
    pub is_srgb: bool,
    pub sampler: ImageSampler,
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

#[non_exhaustive]
#[derive(Debug, Error, Display, From)]
pub enum ImageLoaderError {
    #[display("Could load shader: {_0}")]
    Io(std::io::Error),
    #[display("Could not load texture file: {_0}")]
    FileTexture(FileTextureError),
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
            #[cfg(all(debug_assertions, feature = "dds"))]
            load_context.path().display().to_string(),
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
        ImageFormat::SUPPORTED_FILE_EXTENSIONS
    }
}

impl FromWorld for ImageLoader {
    fn from_world(world: &mut World) -> Self {
        let supported_compressed_formats = match world.get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),

            None => CompressedImageFormats::NONE,
        };
        Self {
            supported_compressed_formats,
        }
    }
}

/// An error that occurs when loading a texture from a file.
#[derive(Error, Display, Debug)]
#[display("Error reading image file {path}: {error}, this is an error in `bevy_render`.")]
pub struct FileTextureError {
    error: TextureError,
    path: String,
}
