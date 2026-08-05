use bevy_asset::{io::Writer, saver::SavedAsset, AssetPath, AsyncWriteExt};

use super::{CompressedImageSaverError, CompressedImageSaverSettings};
use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoaderSettings};
use wgpu_types::TextureFormat;

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
        settings: &CompressedImageSaverSettings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, CompressedImageSaverError> {
        let is_srgb = image.texture_descriptor.format.is_srgb();

        let compressed_basis_data = {
            let mut compressor_params = CompressorParams::new();
            compressor_params.set_basis_format(BasisTextureFormat::UASTC4x4);
            compressor_params.set_generate_mipmaps(settings.generate_mipmaps);
            let color_space = if is_srgb {
                ColorSpace::Srgb
            } else {
                compressor_params.set_no_selector_rdo(true);
                ColorSpace::Linear
            };
            compressor_params.set_color_space(color_space);
            compressor_params.set_uastc_quality_level(UASTC_QUALITY_DEFAULT);
            if settings.is_normal_map {
                compressor_params.tune_for_normal_maps();
            }

            let format = image.texture_descriptor.format;
            let size = image.size();
            let data = image
                .data
                .as_ref()
                .ok_or(CompressedImageSaverError::UninitializedImage)?;
            // Get the per-pixel channel count from the texture format.
            let channel_count = pixel_channels(format)
                .ok_or(CompressedImageSaverError::UnsupportedFormat(format))?;

            let mut source_image = compressor_params.source_image_mut(0);
            source_image.init(data, size.x, size.y, channel_count);

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

/// Returns the channel_count of `format` passed to basis-universal's `CompressorImageRef::init`,
/// which internally calls the FFI `image_init`, or `None` if `format` can't be fed to `CompressorImageRef` at all.
///
/// The FFI `image_init` blindly reads `width * height * comps` bytes as `u8` channels.
/// Only 8-bit-per-channel, RGBA-ordered, unpacked formats (`R8*`, `Rg8*`, `Rgba8*`) satisfy "one byte = one channel"
/// -- everything else (wider/float/packed formats, `Bgra8*`'s reversed order, block-compressed formats,
/// depth/stencil formats) would make `image_init` overread the buffer or read the wrong bytes as channels,
/// and these cases return `None`.
fn pixel_channels(format: TextureFormat) -> Option<u8> {
    match format {
        TextureFormat::R8Unorm
        | TextureFormat::R8Snorm
        | TextureFormat::R8Uint
        | TextureFormat::R8Sint => Some(1),
        TextureFormat::Rg8Unorm
        | TextureFormat::Rg8Snorm
        | TextureFormat::Rg8Uint
        | TextureFormat::Rg8Sint => Some(2),
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint => Some(4),
        // Every other TextureFormat (16/32-bit, float, packed, BGRA-order,
        // block-compressed, depth/stencil, ...) is not safely feedable to
        // basis-universal as raw bytes.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_asset::saver::SavedAsset;
    use bevy_asset::RenderAssetUsages;
    use futures_lite::future::block_on;
    use futures_lite::io::AssertAsync;
    use wgpu_types::{Extent3d, TextureDimension};

    /// Helper function: run the saver on a 1x1 image of `format` with `data`,
    /// returning the produced basis bytes (or the saver error).
    /// `AssertAsync<Vec<u8>>` adapts `Vec<u8>`'s `std::io::Write` into
    /// the `AsyncWrite` the saver expects and lets us inspect the output.
    fn compress_1x1(
        format: TextureFormat,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, CompressedImageSaverError> {
        block_on(async move {
            let image = Image::new(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                format,
                RenderAssetUsages::default(),
            );
            let mut writer = AssertAsync::new(Vec::<u8>::new());
            let saved = SavedAsset::from_asset(&image);
            let settings = CompressedImageSaverSettings::default();
            CompressedImageSaverUniversal::default()
                .save(&mut writer, saved, &settings, "x.basis".into())
                .await?;
            Ok(writer.into_inner())
        })
    }

    /// Every format `pixel_channels` accepts must let the saver run to completion and
    /// produce a compressed result. Each case below is one representative of the formats
    /// mapped to a given `comps` value (1, 2, and 4 channels respectively).
    #[test]
    fn saver_compresses_supported_formats() {
        let out = compress_1x1(TextureFormat::R8Unorm, vec![0]);
        assert!(out.is_ok(), "R8Unorm should compress: {out:?}");

        let out = compress_1x1(TextureFormat::Rg8Unorm, vec![0, 0]);
        assert!(out.is_ok(), "Rg8Unorm should compress: {out:?}");

        let out = compress_1x1(TextureFormat::Rgba8Unorm, vec![0, 0, 0, 255]);
        assert!(out.is_ok(), "Rgba8Unorm should compress: {out:?}");
    }

    /// Formats `pixel_channels` cannot map to a byte-per-channel layout must be rejected
    /// with `UnsupportedFormat` rather than fed to `source_image.init`. Each case below
    /// covers one of the rejection reasons listed in `pixel_channels`'s doc comment, so
    /// removing one silently drops coverage for that entire reason, not just one format.
    #[test]
    fn saver_rejects_unsupported_formats() {
        // Wider-than-8-bit / float: bytes-per-channel != 1, so reading one byte per channel
        // would split/misread the data.
        let err = compress_1x1(TextureFormat::Rgba16Float, vec![0; 8]);
        assert!(
            matches!(
                err,
                Err(CompressedImageSaverError::UnsupportedFormat(
                    TextureFormat::Rgba16Float
                ))
            ),
            "Rgba16Float should be rejected: {err:?}"
        );

        // Reversed channel order: same byte size as Rgba8Unorm, but B-G-R-A instead of
        // R-G-B-A, so `image_init` (which always reads R-G-B-A) would swap red and blue.
        let err = compress_1x1(TextureFormat::Bgra8Unorm, vec![0, 0, 0, 0]);
        assert!(
            matches!(err, Err(CompressedImageSaverError::UnsupportedFormat(_))),
            "Bgra8Unorm should be rejected: {err:?}"
        );

        // Packed: channels aren't byte-aligned, so there is no valid comps at all.
        let err = compress_1x1(TextureFormat::Rgb10a2Unorm, vec![0; 4]);
        assert!(
            matches!(err, Err(CompressedImageSaverError::UnsupportedFormat(_))),
            "Rgb10a2Unorm should be rejected: {err:?}"
        );

        // Block-compressed: bytes are a compressed block, not raw per-pixel channels.
        let err = compress_1x1(TextureFormat::Bc7RgbaUnorm, vec![0; 16]);
        assert!(
            matches!(err, Err(CompressedImageSaverError::UnsupportedFormat(_))),
            "Bc7RgbaUnorm should be rejected: {err:?}"
        );

        // Depth/stencil: bytes encode a depth value, not per-pixel color channels.
        let err = compress_1x1(TextureFormat::Depth32Float, vec![0; 4]);
        assert!(
            matches!(err, Err(CompressedImageSaverError::UnsupportedFormat(_))),
            "Depth32Float should be rejected: {err:?}"
        );
    }
}
