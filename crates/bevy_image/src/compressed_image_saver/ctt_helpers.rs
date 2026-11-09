use std::env;

use ctt::{
    encoders::{
        astcenc::{AstcencSettings, AstcencUsage, NormalSwizzle},
        Encoder,
    },
    AlphaMode, ColorSpace, FormatExt, TargetFormat,
};
use ktx2::Format;
use wgpu_types::TextureFormat;

use super::{CompressedImageSaverError, ImageCompressorAlphaMode};
use crate::ctt_format::wgpu_to_ctt_texture_format;

/// Returns `Some((unorm, hdr))` ASTC format pair if the env var is set, `None` otherwise.
pub fn parse_astc_env_var() -> Result<Option<(Format, Format)>, CompressedImageSaverError> {
    let Ok(val) = env::var("BEVY_COMPRESSED_IMAGE_SAVER_ASTC") else {
        return Ok(None);
    };

    let val = val.trim();
    let (unorm, hdr) = match val {
        "" | "1" | "4x4" => (Format::ASTC_4x4_UNORM_BLOCK, Format::ASTC_4x4_SFLOAT_BLOCK),
        "5x4" => (Format::ASTC_5x4_UNORM_BLOCK, Format::ASTC_5x4_SFLOAT_BLOCK),
        "5x5" => (Format::ASTC_5x5_UNORM_BLOCK, Format::ASTC_5x5_SFLOAT_BLOCK),
        "6x5" => (Format::ASTC_6x5_UNORM_BLOCK, Format::ASTC_6x5_SFLOAT_BLOCK),
        "6x6" => (Format::ASTC_6x6_UNORM_BLOCK, Format::ASTC_6x6_SFLOAT_BLOCK),
        "8x5" => (Format::ASTC_8x5_UNORM_BLOCK, Format::ASTC_8x5_SFLOAT_BLOCK),
        "8x6" => (Format::ASTC_8x6_UNORM_BLOCK, Format::ASTC_8x6_SFLOAT_BLOCK),
        "8x8" => (Format::ASTC_8x8_UNORM_BLOCK, Format::ASTC_8x8_SFLOAT_BLOCK),
        "10x5" => (
            Format::ASTC_10x5_UNORM_BLOCK,
            Format::ASTC_10x5_SFLOAT_BLOCK,
        ),
        "10x6" => (
            Format::ASTC_10x6_UNORM_BLOCK,
            Format::ASTC_10x6_SFLOAT_BLOCK,
        ),
        "10x8" => (
            Format::ASTC_10x8_UNORM_BLOCK,
            Format::ASTC_10x8_SFLOAT_BLOCK,
        ),
        "10x10" => (
            Format::ASTC_10x10_UNORM_BLOCK,
            Format::ASTC_10x10_SFLOAT_BLOCK,
        ),
        "12x10" => (
            Format::ASTC_12x10_UNORM_BLOCK,
            Format::ASTC_12x10_SFLOAT_BLOCK,
        ),
        "12x12" => (
            Format::ASTC_12x12_UNORM_BLOCK,
            Format::ASTC_12x12_SFLOAT_BLOCK,
        ),
        other => {
            return Err(CompressedImageSaverError::CompressionFailed(
                format!("Invalid BEVY_COMPRESSED_IMAGE_SAVER_ASTC block size: {other:?}. \
                    Expected one of: 4x4, 5x4, 5x5, 6x5, 6x6, 8x5, 8x6, 8x8, 10x5, 10x6, 10x8, 10x10, 12x10, 12x12")
                    .into(),
            ));
        }
    };

    Ok(Some((unorm, hdr)))
}

pub fn choose_ctt_compressed_format(
    input: TextureFormat,
    color_space: ColorSpace,
    is_normal_map: bool,
) -> Result<TargetFormat, CompressedImageSaverError> {
    let astc_block = parse_astc_env_var()?;

    // Normal maps go to a two-channel format (X, Y) regardless of the input's channel count
    if is_normal_map {
        return Ok(match astc_block {
            Some((astc_unorm, _)) => TargetFormat::Compressed {
                encoder: Encoder::Astcenc(AstcencSettings {
                    usage: AstcencUsage::NormalMap {
                        swizzle: NormalSwizzle::Bc5Compat,
                    },
                    ..Default::default()
                }),
                format: astc_unorm,
            },
            None => TargetFormat::Compressed {
                encoder: Encoder::Auto,
                format: Format::BC5_UNORM_BLOCK,
            },
        });
    }

    let format = match input {
        // 1-channel snorm (ASTC has no snorm variant, pass through uncompressed if ASTC is preferred)
        TextureFormat::R8Snorm => {
            if astc_block.is_some() {
                return Ok(TargetFormat::Uncompressed(ctt_format(input)?));
            }
            Format::BC4_SNORM_BLOCK
        }

        // 1-channel
        TextureFormat::R8Unorm => {
            if let Some((astc_unorm, _)) = astc_block {
                astc_unorm
            } else {
                Format::BC4_UNORM_BLOCK
            }
        }

        // 2-channel snorm (ASTC has no snorm variant, pass through uncompressed if ASTC is preferred)
        TextureFormat::Rg8Snorm => {
            if astc_block.is_some() {
                return Ok(TargetFormat::Uncompressed(ctt_format(input)?));
            }
            Format::BC5_SNORM_BLOCK
        }

        // 2-channel
        TextureFormat::Rg8Unorm => {
            if let Some((astc_unorm, _)) = astc_block {
                astc_unorm
            } else {
                Format::BC5_UNORM_BLOCK
            }
        }

        // HDR / float formats
        TextureFormat::Rgb9e5Ufloat
        | TextureFormat::Rg11b10Ufloat
        | TextureFormat::R16Float
        | TextureFormat::Rg16Float
        | TextureFormat::Rgba16Float => {
            if let Some((_, astc_hdr)) = astc_block {
                astc_hdr
            } else {
                Format::BC6H_UFLOAT_BLOCK
            }
        }

        // 4-channel LDR
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Bgra8Unorm
        | TextureFormat::Bgra8UnormSrgb
        | TextureFormat::Rgb10a2Unorm => {
            if let Some((astc_unorm, _)) = astc_block {
                astc_unorm
            } else {
                Format::BC7_UNORM_BLOCK
            }
        }

        // Already compressed -> re-encode to the same format
        TextureFormat::Bc1RgbaUnorm
        | TextureFormat::Bc1RgbaUnormSrgb
        | TextureFormat::Bc2RgbaUnorm
        | TextureFormat::Bc2RgbaUnormSrgb
        | TextureFormat::Bc3RgbaUnorm
        | TextureFormat::Bc3RgbaUnormSrgb
        | TextureFormat::Bc4RUnorm
        | TextureFormat::Bc4RSnorm
        | TextureFormat::Bc5RgUnorm
        | TextureFormat::Bc5RgSnorm
        | TextureFormat::Bc6hRgbUfloat
        | TextureFormat::Bc6hRgbFloat
        | TextureFormat::Bc7RgbaUnorm
        | TextureFormat::Bc7RgbaUnormSrgb
        | TextureFormat::Etc2Rgb8Unorm
        | TextureFormat::Etc2Rgb8UnormSrgb
        | TextureFormat::Etc2Rgb8A1Unorm
        | TextureFormat::Etc2Rgb8A1UnormSrgb
        | TextureFormat::Etc2Rgba8Unorm
        | TextureFormat::Etc2Rgba8UnormSrgb
        | TextureFormat::EacR11Unorm
        | TextureFormat::EacR11Snorm
        | TextureFormat::EacRg11Unorm
        | TextureFormat::EacRg11Snorm
        | TextureFormat::Astc { .. } => ctt_format(input)?,

        // Integer, high-precision, and float formats -> pass through uncompressed
        TextureFormat::R8Uint
        | TextureFormat::R8Sint
        | TextureFormat::R16Uint
        | TextureFormat::R16Sint
        | TextureFormat::R16Unorm
        | TextureFormat::R16Snorm
        | TextureFormat::R32Uint
        | TextureFormat::R32Sint
        | TextureFormat::R32Float
        | TextureFormat::R64Uint
        | TextureFormat::Rg8Uint
        | TextureFormat::Rg8Sint
        | TextureFormat::Rg16Uint
        | TextureFormat::Rg16Sint
        | TextureFormat::Rg16Unorm
        | TextureFormat::Rg16Snorm
        | TextureFormat::Rg32Uint
        | TextureFormat::Rg32Sint
        | TextureFormat::Rg32Float
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba16Uint
        | TextureFormat::Rgba16Sint
        | TextureFormat::Rgba16Unorm
        | TextureFormat::Rgba16Snorm
        | TextureFormat::Rgba32Uint
        | TextureFormat::Rgba32Sint
        | TextureFormat::Rgba32Float
        | TextureFormat::Rgb10a2Uint => {
            return Ok(TargetFormat::Uncompressed(ctt_format(input)?));
        }

        // Depth/stencil and video formats cannot be compressed
        TextureFormat::Stencil8
        | TextureFormat::Depth16Unorm
        | TextureFormat::Depth24Plus
        | TextureFormat::Depth24PlusStencil8
        | TextureFormat::Depth32Float
        | TextureFormat::Depth32FloatStencil8
        | TextureFormat::NV12
        | TextureFormat::P010 => {
            return Err(CompressedImageSaverError::UnsupportedFormat(input));
        }
    };

    Ok(TargetFormat::Compressed {
        encoder: Encoder::Auto,
        format: format.with_color_space(color_space),
    })
}

pub fn ctt_format(input: TextureFormat) -> Result<Format, CompressedImageSaverError> {
    wgpu_to_ctt_texture_format(input).ok_or(CompressedImageSaverError::UnsupportedFormat(input))
}

pub fn bevy_to_ctt_alpha_mode(alpha_mode: ImageCompressorAlphaMode) -> AlphaMode {
    match alpha_mode {
        ImageCompressorAlphaMode::Straight => AlphaMode::Straight,
        ImageCompressorAlphaMode::Premultiplied => AlphaMode::Premultiplied,
        ImageCompressorAlphaMode::Opaque => AlphaMode::Opaque,
    }
}
