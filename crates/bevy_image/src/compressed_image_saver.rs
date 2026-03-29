use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};

use bevy_asset::{
    saver::{AssetSaver, SavedAsset},
    AssetPath,
};
use bevy_reflect::TypePath;
use futures_lite::AsyncWriteExt;
use thiserror::Error;

/// An [`AssetSaver`] for [`Image`] that compresses texture files.
///
/// Compressed textures both take up less space on disk, and use less VRAM.
///
/// TODO: Document what platforms are supported, how feature flags work,
/// required native dependencies (https://github.com/cwfitzgerald/ctt?tab=readme-ov-file#prerequisites),
/// what compression types exist, and mipmap generation?
#[derive(TypePath)]
pub struct CompressedImageSaver;

/// Errors encountered when writing compressed images via [`CompressedImageSaver`].
#[non_exhaustive]
#[derive(Debug, Error, TypePath)]
pub enum CompressedImageSaverError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The underlying compression library returned an error.
    #[error(transparent)]
    CompressionFailed(Box<dyn std::error::Error>),
    /// Attempted to save an image with uninitialized data.
    #[error("Cannot compress an uninitialized image")]
    UninitializedImage,
}

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = ();
    type OutputLoader = ImageLoader;
    type Error = CompressedImageSaverError;

    #[cfg(feature = "compressed_image_saver_desktop")]
    async fn save(
        &self,
        writer: &mut bevy_asset::io::Writer,
        image: SavedAsset<'_, '_, Self::Asset>,
        _settings: &Self::Settings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        let Some(ref data) = image.data else {
            return Err(CompressedImageSaverError::UninitializedImage);
        };

        if image.texture_descriptor.mip_level_count != 1 {
            return Err(CompressedImageSaverError::CompressionFailed(
                "Expected texture_descriptor.mip_level_count to be 1".into(),
            ));
        }

        let is_srgb = image.texture_descriptor.format.is_srgb();
        let color_space = if is_srgb {
            ctt::format::ColorSpace::Srgb
        } else {
            ctt::format::ColorSpace::Linear
        };

        let is_cubemap = matches!(
            image.texture_view_descriptor,
            Some(wgpu_types::TextureViewDescriptor {
                dimension: Some(wgpu_types::TextureViewDimension::Cube),
                ..
            })
        );

        let layers = (0..image.texture_descriptor.array_layer_count())
            .into_iter()
            .map(|layer| {
                vec![ctt::image::RawImage {
                    data: todo!(),
                    width: image.width(),
                    height: image.height(),
                    stride: todo!(),
                    pixel_format: ctt::format::PixelFormat {
                        components: todo!(),
                        channel_type: todo!(),
                        color_space,
                    },
                }];
            })
            .collect();
        let layout = ctt::image::ImageLayout { layers, is_cubemap };

        let config = ctt::config::CompressConfig {
            format: todo!(),
            output_format: ctt::config::OutputFormat::Ktx2,
            swizzle: None,
            color_space,
            encode_settings: None,
        };

        let compressed_bytes = ctt::pipeline::run(&config, layout)
            .await
            .map_err(|e| CompressedImageSaver::CompressionFailed(Box::new(e)))?;

        writer.write_all(&compressed_bytes).await?;

        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Ktx2),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
            texture_format: None,
            array_layout: None,
        })
    }

    #[cfg(feature = "compressed_image_saver_web")]
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
                compressor_params.set_no_selector_rdo(true);
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
                compressor
                    .process()
                    .map_err(|e| CompressedImageSaver::CompressionFailed(Box::new(e)))?;
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
