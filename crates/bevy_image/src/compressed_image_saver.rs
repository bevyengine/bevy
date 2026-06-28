use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};

use bevy_asset::{
    saver::{AssetSaver, SavedAsset},
    AssetPath,
};
use bevy_reflect::TypePath;
use futures_lite::AsyncWriteExt;
use thiserror::Error;

/// An [`AssetSaver`] that writes compressed basis universal (.ktx2) files.
#[derive(TypePath)]
pub struct CompressedImageSaver;

/// Errors encountered when writing compressed images.
#[non_exhaustive]
#[derive(Debug, Error, TypePath)]
pub enum CompressedImageSaverError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Attempted to save an image with uninitialized data.
    #[error("Cannot compress an uninitialized image")]
    UninitializedImage,
}

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = ();
    type OutputLoader = ImageLoader;
    type Error = CompressedImageSaverError;

    async fn save(
        &self,
        writer: &mut bevy_asset::io::Writer,
        image: SavedAsset<'_, '_, Self::Asset>,
        _settings: &Self::Settings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        let is_srgb = image.texture_descriptor.format.is_srgb();

        let compressed_basis_data = {
            let mut compressor_params = basis_universal::CompressorParams::new();
            compressor_params.set_basis_format(basis_universal::BasisTextureFormat::UASTC4x4);
            compressor_params.set_generate_mipmaps(true);
            let color_space = if is_srgb {
                basis_universal::ColorSpace::Srgb
            } else {
                basis_universal::ColorSpace::Linear
            };
            compressor_params.set_color_space(color_space);
            compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);

            let mut source_image = compressor_params.source_image_mut(0);
            let size = image.size();
            let Some(ref data) = image.data else {
                return Err(CompressedImageSaverError::UninitializedImage);
            };
            source_image.init(data, size.x, size.y, 4);

            let mut compressor = basis_universal::Compressor::new(4);
            #[expect(
                unsafe_code,
                reason = "The basis-universal compressor cannot be interacted with except through unsafe functions"
            )]
            // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
            // library bindings note that invalid params might produce undefined behavior.
            unsafe {
                compressor.init(&compressor_params);
                compressor.process().unwrap();
            }
            compressor.basis_file().to_vec()
        };

        writer.write_all(&compressed_basis_data).await?;
        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Basis),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
            texture_format: None,
            array_layout: None,
        })
    }
}
