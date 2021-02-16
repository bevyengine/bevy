use super::{image_texture_conversion::image_to_texture, Texture};
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_utils::BoxedFuture;
use thiserror::Error;

/// Loader for images that can be read by the `image` crate.
#[derive(Clone, Default)]
pub struct ImageTextureLoader;

const FILE_EXTENSIONS: &[&str] = &["png", "dds", "tga", "jpg", "jpeg", "bmp"];

impl AssetLoader for ImageTextureLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            // use the file extension for the image type
            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let dyn_img = buffer_to_texture(bytes, ImageType::Extension(ext)).map_err(|err| {
                FileTextureError {
                    error: err,
                    path: format!("{}", load_context.path().display()),
                }
            })?;

            load_context.set_default_asset(LoadedAsset::new(dyn_img));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        FILE_EXTENSIONS
    }
}

/// An error that occurs when loading a texture from a file
#[derive(Error, Debug)]
pub struct FileTextureError {
    error: TextureError,
    path: String,
}
impl std::fmt::Display for FileTextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Error reading image file {}: {}, this is an error in `bevy_render`.",
            self.path, self.error
        )
    }
}

/// An error that occurs when loading a texture
#[derive(Error, Debug)]
pub enum TextureError {
    #[error("invalid image mime type")]
    InvalidImageMimeType(String),
    #[error("invalid image extension")]
    InvalidImageExtension(String),
    #[error("failed to load an image")]
    ImageError(#[from] image::ImageError),
}

/// Type of a raw image buffer
pub enum ImageType<'a> {
    /// Mime type of an image, for example `"image/png"`
    MimeType(&'a str),
    /// Extension of an image file, for example `"png"`
    Extension(&'a str),
}

/// Load a bytes buffer in a [`Texture`], according to type `image_type`, using the `image` crate`
pub fn buffer_to_texture(buffer: &[u8], image_type: ImageType) -> Result<Texture, TextureError> {
    let format = match image_type {
        ImageType::MimeType(mime_type) => match mime_type {
            "image/png" => Ok(image::ImageFormat::Png),
            "image/vnd-ms.dds" => Ok(image::ImageFormat::Dds),
            "image/x-targa" => Ok(image::ImageFormat::Tga),
            "image/x-tga" => Ok(image::ImageFormat::Tga),
            "image/jpeg" => Ok(image::ImageFormat::Jpeg),
            "image/bmp" => Ok(image::ImageFormat::Bmp),
            "image/x-bmp" => Ok(image::ImageFormat::Bmp),
            _ => Err(TextureError::InvalidImageMimeType(mime_type.to_string())),
        },
        ImageType::Extension(extension) => image::ImageFormat::from_extension(extension)
            .ok_or_else(|| TextureError::InvalidImageMimeType(extension.to_string())),
    }?;

    // Load the image in the expected format.
    // Some formats like PNG allow for R or RG textures too, so the texture
    // format needs to be determined. For RGB textures an alpha channel
    // needs to be added, so the image data needs to be converted in those
    // cases.

    let dyn_img = image::load_from_memory_with_format(buffer, format)?;
    Ok(image_to_texture(dyn_img))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_file_extensions() {
        for ext in FILE_EXTENSIONS {
            assert!(image::ImageFormat::from_extension(ext).is_some())
        }
    }
}
