use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_utils::BoxedFuture;
use thiserror::Error;

use crate::texture::{Image, ImageError, ImageType};

/// Loader for images that can be read by the `image` crate.
#[derive(Clone, Default)]
pub struct ImageTextureLoader;

/// All supported image file extensions.
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

            let dyn_img = Image::from_buffer(bytes, ImageType::Extension(ext)).map_err(|err| {
                FileImageError {
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

/// An error that occurs when loading a texture from a file.
#[derive(Error, Debug)]
pub struct FileImageError {
    error: ImageError,
    path: String,
}
impl std::fmt::Display for FileImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Error reading image file {}: {}, this is an error in `bevy_render`.",
            self.path, self.error
        )
    }
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
