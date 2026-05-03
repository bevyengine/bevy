use bevy_asset::{io::Writer, saver::SavedAsset, AssetPath, AsyncWriteExt};

use super::{CompressedImageSaverError, CompressedImageSaverSettings};
use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoaderSettings};

use basis_universal::{
    BasisTextureFormat, ColorSpace, Compressor, CompressorParams, UASTC_QUALITY_DEFAULT,
};

#[derive(Default)]
pub struct CompressedImageSaverUniversal;

impl CompressedImageSaverUniversal {
    pub async fn save(
        &self,
        writer: &mut Writer,
        image: SavedAsset<'_, '_, Image>,
        _settings: &CompressedImageSaverSettings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, CompressedImageSaverError> {
        let is_srgb = image.texture_descriptor.format.is_srgb();

        let compressed_basis_data = {
            let mut compressor_params = CompressorParams::new();
            compressor_params.set_basis_format(BasisTextureFormat::UASTC4x4);
            compressor_params.set_generate_mipmaps(true);
            let color_space = if is_srgb {
                ColorSpace::Srgb
            } else {
                compressor_params.set_no_selector_rdo(true);
                ColorSpace::Linear
            };
            compressor_params.set_color_space(color_space);
            compressor_params.set_uastc_quality_level(UASTC_QUALITY_DEFAULT);

            let mut source_image = compressor_params.source_image_mut(0);
            let size = image.size();
            let Some(ref data) = image.data else {
                return Err(CompressedImageSaverError::UninitializedImage);
            };
            source_image.init(data, size.x, size.y, 4);

            let mut compressor = Compressor::new(4);
            #[expect(
                unsafe_code,
                reason = "The basis-universal compressor cannot be interacted with except through unsafe functions"
            )]
            // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
            // library bindings note that invalid params might produce undefined behavior.
            unsafe {
                compressor.init(&compressor_params);
                compressor.process().map_err(|e| {
                    CompressedImageSaverError::CompressionFailed(format!("{e:?}").into())
                })?;
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
