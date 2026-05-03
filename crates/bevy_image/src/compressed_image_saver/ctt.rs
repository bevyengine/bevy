use bevy_asset::{io::Writer, saver::SavedAsset, AssetPath, AsyncWriteExt};

use super::{
    ctt_helpers::{
        bevy_to_ctt_alpha_mode, choose_ctt_compressed_format, wgpu_to_ctt_texture_format,
    },
    CompressedImageSaverError, CompressedImageSaverSettings,
};
use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoaderSettings};

#[derive(Default)]
pub struct CompressedImageSaverCtt;

impl CompressedImageSaverCtt {
    pub async fn save(
        &self,
        writer: &mut Writer,
        image: SavedAsset<'_, '_, Image>,
        settings: &CompressedImageSaverSettings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, CompressedImageSaverError> {
        let Some(ref data) = image.data else {
            return Err(CompressedImageSaverError::UninitializedImage);
        };

        if image.texture_descriptor.mip_level_count != 1 {
            return Err(CompressedImageSaverError::CompressionFailed(
                "Expected texture_descriptor.mip_level_count to be 1".into(),
            ));
        }

        let input_format = wgpu_to_ctt_texture_format(image.texture_descriptor.format)?;
        let output_format = choose_ctt_compressed_format(image.texture_descriptor.format)?;

        let is_srgb = image.texture_descriptor.format.is_srgb();
        let color_space = if is_srgb {
            ctt::ColorSpace::Srgb
        } else {
            ctt::ColorSpace::Linear
        };

        let is_cubemap = matches!(
            image.texture_view_descriptor,
            Some(wgpu_types::TextureViewDescriptor {
                dimension: Some(wgpu_types::TextureViewDimension::Cube),
                ..
            })
        );

        let bytes_per_pixel =
            crate::TextureFormatPixelInfo::pixel_size(&image.texture_descriptor.format).map_err(
                |_| CompressedImageSaverError::UnsupportedFormat(image.texture_descriptor.format),
            )? as u32;

        let surfaces = data
            .chunks_exact((image.width() * image.height() * bytes_per_pixel) as usize)
            .map(|layer_data| {
                vec![ctt::Surface {
                    data: layer_data.to_vec(),
                    width: image.width(),
                    height: image.height(),
                    stride: image.width() * bytes_per_pixel,
                    format: input_format,
                    color_space,
                    alpha: bevy_to_ctt_alpha_mode(settings.input_alpha_mode),
                }]
            })
            .collect();
        let ctt_image = ctt::Image {
            surfaces,
            is_cubemap,
        };

        let settings = ctt::ConvertSettings {
            format: Some(output_format),
            container: ctt::Container::ktx2_zstd(0),
            quality: ctt::Quality::default(),
            output_color_space: None,
            output_alpha: Some(bevy_to_ctt_alpha_mode(settings.output_alpha_mode)),
            swizzle: None,
            mipmap: true,
            mipmap_count: None,
            mipmap_filter: ctt::MipmapFilter::default(),
            encoder_settings: None,
            registry: None,
        };

        let output = ctt::convert(ctt_image, settings)
            .map_err(|e| CompressedImageSaverError::CompressionFailed(Box::new(e)))?;
        let ctt::PipelineOutput::Encoded(compressed_bytes) = &output else {
            return Err(CompressedImageSaverError::CompressionFailed(
                "Expected encoded output from ctt".into(),
            ));
        };

        writer.write_all(compressed_bytes).await?;

        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Ktx2),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
            texture_format: None,
            array_layout: None,
        })
    }
}
