use std::env;

use ctt::{AlphaMode, TargetFormat};
use ktx2::Format;
use wgpu_types::{AstcBlock, AstcChannel, TextureFormat};

use super::{CompressedImageSaverError, ImageCompressorAlphaMode};

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
) -> Result<TargetFormat, CompressedImageSaverError> {
    let astc_block = parse_astc_env_var()?;

    let format = match input {
        // 1-channel snorm (ASTC has no snorm variant, pass through uncompressed if ASTC is preferred)
        TextureFormat::R8Snorm => {
            if astc_block.is_some() {
                return Ok(TargetFormat::Uncompressed(wgpu_to_ctt_texture_format(
                    input,
                )?));
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
                return Ok(TargetFormat::Uncompressed(wgpu_to_ctt_texture_format(
                    input,
                )?));
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

        // Already compressed -> pass through as compressed
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
        | TextureFormat::Astc { .. } => wgpu_to_ctt_texture_format(input)?,

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
            return Ok(TargetFormat::Uncompressed(wgpu_to_ctt_texture_format(
                input,
            )?));
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
        encoder_name: None,
        format,
    })
}

pub fn wgpu_to_ctt_texture_format(
    input: TextureFormat,
) -> Result<Format, CompressedImageSaverError> {
    Ok(match input {
        TextureFormat::R8Unorm => Format::R8_UNORM,
        TextureFormat::R8Snorm => Format::R8_SNORM,
        TextureFormat::R8Uint => Format::R8_UINT,
        TextureFormat::R8Sint => Format::R8_SINT,
        TextureFormat::R16Uint => Format::R16_UINT,
        TextureFormat::R16Sint => Format::R16_SINT,
        TextureFormat::R16Unorm => Format::R16_UNORM,
        TextureFormat::R16Snorm => Format::R16_SNORM,
        TextureFormat::R16Float => Format::R16_SFLOAT,
        TextureFormat::Rg8Unorm => Format::R8G8_UNORM,
        TextureFormat::Rg8Snorm => Format::R8G8_SNORM,
        TextureFormat::Rg8Uint => Format::R8G8_UINT,
        TextureFormat::Rg8Sint => Format::R8G8_SINT,
        TextureFormat::R32Uint => Format::R32_UINT,
        TextureFormat::R32Sint => Format::R32_SINT,
        TextureFormat::R32Float => Format::R32_SFLOAT,
        TextureFormat::Rg16Uint => Format::R16G16_UINT,
        TextureFormat::Rg16Sint => Format::R16G16_SINT,
        TextureFormat::Rg16Unorm => Format::R16G16_UNORM,
        TextureFormat::Rg16Snorm => Format::R16G16_SNORM,
        TextureFormat::Rg16Float => Format::R16G16_SFLOAT,
        TextureFormat::Rgba8Unorm => Format::R8G8B8A8_UNORM,
        TextureFormat::Rgba8UnormSrgb => Format::R8G8B8A8_SRGB,
        TextureFormat::Rgba8Snorm => Format::R8G8B8A8_SNORM,
        TextureFormat::Rgba8Uint => Format::R8G8B8A8_UINT,
        TextureFormat::Rgba8Sint => Format::R8G8B8A8_SINT,
        TextureFormat::Bgra8Unorm => Format::B8G8R8A8_UNORM,
        TextureFormat::Bgra8UnormSrgb => Format::B8G8R8A8_SRGB,
        TextureFormat::Rgb9e5Ufloat => Format::E5B9G9R9_UFLOAT_PACK32,
        TextureFormat::Rgb10a2Uint => Format::A2B10G10R10_UINT_PACK32,
        TextureFormat::Rgb10a2Unorm => Format::A2B10G10R10_UNORM_PACK32,
        TextureFormat::Rg11b10Ufloat => Format::B10G11R11_UFLOAT_PACK32,
        TextureFormat::R64Uint => Format::R64_UINT,
        TextureFormat::Rg32Uint => Format::R32G32_UINT,
        TextureFormat::Rg32Sint => Format::R32G32_SINT,
        TextureFormat::Rg32Float => Format::R32G32_SFLOAT,
        TextureFormat::Rgba16Uint => Format::R16G16B16A16_UINT,
        TextureFormat::Rgba16Sint => Format::R16G16B16A16_SINT,
        TextureFormat::Rgba16Unorm => Format::R16G16B16A16_UNORM,
        TextureFormat::Rgba16Snorm => Format::R16G16B16A16_SNORM,
        TextureFormat::Rgba16Float => Format::R16G16B16A16_SFLOAT,
        TextureFormat::Rgba32Uint => Format::R32G32B32A32_UINT,
        TextureFormat::Rgba32Sint => Format::R32G32B32A32_SINT,
        TextureFormat::Rgba32Float => Format::R32G32B32A32_SFLOAT,
        TextureFormat::Stencil8 => Format::S8_UINT,
        TextureFormat::Depth16Unorm => Format::D16_UNORM,
        TextureFormat::Depth24Plus => Format::X8_D24_UNORM_PACK32,
        TextureFormat::Depth24PlusStencil8 => Format::D24_UNORM_S8_UINT,
        TextureFormat::Depth32Float => Format::D32_SFLOAT,
        TextureFormat::Depth32FloatStencil8 => Format::D32_SFLOAT_S8_UINT,
        TextureFormat::NV12 | TextureFormat::P010 => {
            return Err(CompressedImageSaverError::UnsupportedFormat(input));
        }
        TextureFormat::Bc1RgbaUnorm => Format::BC1_RGBA_UNORM_BLOCK,
        TextureFormat::Bc1RgbaUnormSrgb => Format::BC1_RGBA_SRGB_BLOCK,
        TextureFormat::Bc2RgbaUnorm => Format::BC2_UNORM_BLOCK,
        TextureFormat::Bc2RgbaUnormSrgb => Format::BC2_SRGB_BLOCK,
        TextureFormat::Bc3RgbaUnorm => Format::BC3_UNORM_BLOCK,
        TextureFormat::Bc3RgbaUnormSrgb => Format::BC3_SRGB_BLOCK,
        TextureFormat::Bc4RUnorm => Format::BC4_UNORM_BLOCK,
        TextureFormat::Bc4RSnorm => Format::BC4_SNORM_BLOCK,
        TextureFormat::Bc5RgUnorm => Format::BC5_UNORM_BLOCK,
        TextureFormat::Bc5RgSnorm => Format::BC5_SNORM_BLOCK,
        TextureFormat::Bc6hRgbUfloat => Format::BC6H_UFLOAT_BLOCK,
        TextureFormat::Bc6hRgbFloat => Format::BC6H_SFLOAT_BLOCK,
        TextureFormat::Bc7RgbaUnorm => Format::BC7_UNORM_BLOCK,
        TextureFormat::Bc7RgbaUnormSrgb => Format::BC7_SRGB_BLOCK,
        TextureFormat::Etc2Rgb8Unorm => Format::ETC2_R8G8B8_UNORM_BLOCK,
        TextureFormat::Etc2Rgb8UnormSrgb => Format::ETC2_R8G8B8_SRGB_BLOCK,
        TextureFormat::Etc2Rgb8A1Unorm => Format::ETC2_R8G8B8A1_UNORM_BLOCK,
        TextureFormat::Etc2Rgb8A1UnormSrgb => Format::ETC2_R8G8B8A1_SRGB_BLOCK,
        TextureFormat::Etc2Rgba8Unorm => Format::ETC2_R8G8B8A8_UNORM_BLOCK,
        TextureFormat::Etc2Rgba8UnormSrgb => Format::ETC2_R8G8B8A8_SRGB_BLOCK,
        TextureFormat::EacR11Unorm => Format::EAC_R11_UNORM_BLOCK,
        TextureFormat::EacR11Snorm => Format::EAC_R11_SNORM_BLOCK,
        TextureFormat::EacRg11Unorm => Format::EAC_R11G11_UNORM_BLOCK,
        TextureFormat::EacRg11Snorm => Format::EAC_R11G11_SNORM_BLOCK,
        TextureFormat::Astc { block, channel } => match (block, channel) {
            (AstcBlock::B4x4, AstcChannel::Unorm) => Format::ASTC_4x4_UNORM_BLOCK,
            (AstcBlock::B4x4, AstcChannel::UnormSrgb) => Format::ASTC_4x4_SRGB_BLOCK,
            (AstcBlock::B4x4, AstcChannel::Hdr) => Format::ASTC_4x4_SFLOAT_BLOCK,
            (AstcBlock::B5x4, AstcChannel::Unorm) => Format::ASTC_5x4_UNORM_BLOCK,
            (AstcBlock::B5x4, AstcChannel::UnormSrgb) => Format::ASTC_5x4_SRGB_BLOCK,
            (AstcBlock::B5x4, AstcChannel::Hdr) => Format::ASTC_5x4_SFLOAT_BLOCK,
            (AstcBlock::B5x5, AstcChannel::Unorm) => Format::ASTC_5x5_UNORM_BLOCK,
            (AstcBlock::B5x5, AstcChannel::UnormSrgb) => Format::ASTC_5x5_SRGB_BLOCK,
            (AstcBlock::B5x5, AstcChannel::Hdr) => Format::ASTC_5x5_SFLOAT_BLOCK,
            (AstcBlock::B6x5, AstcChannel::Unorm) => Format::ASTC_6x5_UNORM_BLOCK,
            (AstcBlock::B6x5, AstcChannel::UnormSrgb) => Format::ASTC_6x5_SRGB_BLOCK,
            (AstcBlock::B6x5, AstcChannel::Hdr) => Format::ASTC_6x5_SFLOAT_BLOCK,
            (AstcBlock::B6x6, AstcChannel::Unorm) => Format::ASTC_6x6_UNORM_BLOCK,
            (AstcBlock::B6x6, AstcChannel::UnormSrgb) => Format::ASTC_6x6_SRGB_BLOCK,
            (AstcBlock::B6x6, AstcChannel::Hdr) => Format::ASTC_6x6_SFLOAT_BLOCK,
            (AstcBlock::B8x5, AstcChannel::Unorm) => Format::ASTC_8x5_UNORM_BLOCK,
            (AstcBlock::B8x5, AstcChannel::UnormSrgb) => Format::ASTC_8x5_SRGB_BLOCK,
            (AstcBlock::B8x5, AstcChannel::Hdr) => Format::ASTC_8x5_SFLOAT_BLOCK,
            (AstcBlock::B8x6, AstcChannel::Unorm) => Format::ASTC_8x6_UNORM_BLOCK,
            (AstcBlock::B8x6, AstcChannel::UnormSrgb) => Format::ASTC_8x6_SRGB_BLOCK,
            (AstcBlock::B8x6, AstcChannel::Hdr) => Format::ASTC_8x6_SFLOAT_BLOCK,
            (AstcBlock::B8x8, AstcChannel::Unorm) => Format::ASTC_8x8_UNORM_BLOCK,
            (AstcBlock::B8x8, AstcChannel::UnormSrgb) => Format::ASTC_8x8_SRGB_BLOCK,
            (AstcBlock::B8x8, AstcChannel::Hdr) => Format::ASTC_8x8_SFLOAT_BLOCK,
            (AstcBlock::B10x5, AstcChannel::Unorm) => Format::ASTC_10x5_UNORM_BLOCK,
            (AstcBlock::B10x5, AstcChannel::UnormSrgb) => Format::ASTC_10x5_SRGB_BLOCK,
            (AstcBlock::B10x5, AstcChannel::Hdr) => Format::ASTC_10x5_SFLOAT_BLOCK,
            (AstcBlock::B10x6, AstcChannel::Unorm) => Format::ASTC_10x6_UNORM_BLOCK,
            (AstcBlock::B10x6, AstcChannel::UnormSrgb) => Format::ASTC_10x6_SRGB_BLOCK,
            (AstcBlock::B10x6, AstcChannel::Hdr) => Format::ASTC_10x6_SFLOAT_BLOCK,
            (AstcBlock::B10x8, AstcChannel::Unorm) => Format::ASTC_10x8_UNORM_BLOCK,
            (AstcBlock::B10x8, AstcChannel::UnormSrgb) => Format::ASTC_10x8_SRGB_BLOCK,
            (AstcBlock::B10x8, AstcChannel::Hdr) => Format::ASTC_10x8_SFLOAT_BLOCK,
            (AstcBlock::B10x10, AstcChannel::Unorm) => Format::ASTC_10x10_UNORM_BLOCK,
            (AstcBlock::B10x10, AstcChannel::UnormSrgb) => Format::ASTC_10x10_SRGB_BLOCK,
            (AstcBlock::B10x10, AstcChannel::Hdr) => Format::ASTC_10x10_SFLOAT_BLOCK,
            (AstcBlock::B12x10, AstcChannel::Unorm) => Format::ASTC_12x10_UNORM_BLOCK,
            (AstcBlock::B12x10, AstcChannel::UnormSrgb) => Format::ASTC_12x10_SRGB_BLOCK,
            (AstcBlock::B12x10, AstcChannel::Hdr) => Format::ASTC_12x10_SFLOAT_BLOCK,
            (AstcBlock::B12x12, AstcChannel::Unorm) => Format::ASTC_12x12_UNORM_BLOCK,
            (AstcBlock::B12x12, AstcChannel::UnormSrgb) => Format::ASTC_12x12_SRGB_BLOCK,
            (AstcBlock::B12x12, AstcChannel::Hdr) => Format::ASTC_12x12_SFLOAT_BLOCK,
        },
    })
}

pub fn bevy_to_ctt_alpha_mode(alpha_mode: ImageCompressorAlphaMode) -> AlphaMode {
    match alpha_mode {
        ImageCompressorAlphaMode::Straight => AlphaMode::Straight,
        ImageCompressorAlphaMode::Premultiplied => AlphaMode::Premultiplied,
        ImageCompressorAlphaMode::Opaque => AlphaMode::Opaque,
    }
}
