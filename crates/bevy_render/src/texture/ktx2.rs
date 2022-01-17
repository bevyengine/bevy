use wgpu::{Extent3d, TextureDimension, TextureFormat};

use super::{Image, TextureError};

pub fn ktx2_buffer_to_image(buffer: &[u8], is_srgb: bool) -> Result<Image, TextureError> {
    let ktx2 = ktx2::Reader::new(buffer).expect("Can't create reader");
    let ktx2_header = ktx2.header();
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: ktx2_header.pixel_width,
        height: ktx2_header.pixel_height,
        depth_or_array_layers: if ktx2_header.layer_count > 1 {
            ktx2_header.layer_count
        } else {
            ktx2_header.pixel_depth
        },
    };
    image.texture_descriptor.mip_level_count = ktx2_header.level_count;
    image.texture_descriptor.dimension = if ktx2_header.pixel_depth > 1 {
        TextureDimension::D3
    } else if image.is_compressed() || ktx2_header.pixel_height > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    };
    if let Some(format) = ktx2_header.format {
        image.texture_descriptor.format = ktx2_format_to_texture_format(format, is_srgb)?;
        image.data = ktx2.levels().flatten().copied().collect();
    } else if let Some(supercompression_scheme) = ktx2_header.supercompression_scheme {
        return Err(TextureError::SuperCompressionNotSupported(format!(
            "{:?}",
            supercompression_scheme
        )));
    } else {
        return Err(TextureError::UnsupportedTextureFormat(
            "unspecified".to_string(),
        ));
    }
    Ok(image)
}

pub fn ktx2_format_to_texture_format(
    ktx2_format: ktx2::Format,
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    Ok(match ktx2_format {
        ktx2::Format::R4G4_UNORM_PACK8 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R4G4B4A4_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B4G4R4A4_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R5G6B5_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B5G6R5_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R5G5B5A1_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B5G5R5A1_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A1R5G5B5_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8_UNORM => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::R8Unorm
            }
        }
        ktx2::Format::R8_SNORM => TextureFormat::R8Snorm,
        ktx2::Format::R8_UINT => TextureFormat::R8Uint,
        ktx2::Format::R8_SINT => TextureFormat::R8Sint,
        ktx2::Format::R8_SRGB => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::R8Unorm
            }
        }
        ktx2::Format::R8G8_UNORM => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::Rg8Unorm
            }
        }
        ktx2::Format::R8G8_SNORM => TextureFormat::Rg8Snorm,
        ktx2::Format::R8G8_UINT => TextureFormat::Rg8Uint,
        ktx2::Format::R8G8_SINT => TextureFormat::Rg8Sint,
        ktx2::Format::R8G8_SRGB => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::Rg8Unorm
            }
        }
        ktx2::Format::R8G8B8_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_SRGB => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_SRGB => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8A8_UNORM => {
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            }
        }
        ktx2::Format::R8G8B8A8_SNORM => TextureFormat::Rgba8Snorm,
        ktx2::Format::R8G8B8A8_UINT => TextureFormat::Rgba8Uint,
        ktx2::Format::R8G8B8A8_SINT => TextureFormat::Rgba8Sint,
        ktx2::Format::R8G8B8A8_SRGB => {
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            }
        }
        ktx2::Format::B8G8R8A8_UNORM => {
            if is_srgb {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        }
        ktx2::Format::B8G8R8A8_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8A8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8A8_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8A8_SRGB => {
            if is_srgb {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        }
        // FIXME: Is this correct?
        ktx2::Format::A2R10G10B10_UNORM_PACK32 => TextureFormat::Rgb10a2Unorm,
        ktx2::Format::A2R10G10B10_SNORM_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2R10G10B10_UINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2R10G10B10_SINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_UNORM_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_SNORM_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_UINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_SINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16_UNORM => TextureFormat::R16Unorm,
        ktx2::Format::R16_SNORM => TextureFormat::R16Snorm,
        ktx2::Format::R16_UINT => TextureFormat::R16Uint,
        ktx2::Format::R16_SINT => TextureFormat::R16Sint,
        ktx2::Format::R16_SFLOAT => TextureFormat::R16Float,
        ktx2::Format::R16G16_UNORM => TextureFormat::Rg16Unorm,
        ktx2::Format::R16G16_SNORM => TextureFormat::Rg16Snorm,
        ktx2::Format::R16G16_UINT => TextureFormat::Rg16Uint,
        ktx2::Format::R16G16_SINT => TextureFormat::Rg16Sint,
        ktx2::Format::R16G16_SFLOAT => TextureFormat::Rg16Float,
        ktx2::Format::R16G16B16_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16A16_UNORM => TextureFormat::Rgba16Unorm,
        ktx2::Format::R16G16B16A16_SNORM => TextureFormat::Rgba16Snorm,
        ktx2::Format::R16G16B16A16_UINT => TextureFormat::Rgba16Uint,
        ktx2::Format::R16G16B16A16_SINT => TextureFormat::Rgba16Sint,
        ktx2::Format::R16G16B16A16_SFLOAT => TextureFormat::Rgba16Float,
        ktx2::Format::R32_UINT => TextureFormat::R32Uint,
        ktx2::Format::R32_SINT => TextureFormat::R32Sint,
        ktx2::Format::R32_SFLOAT => TextureFormat::R32Float,
        ktx2::Format::R32G32_UINT => TextureFormat::Rg32Uint,
        ktx2::Format::R32G32_SINT => TextureFormat::Rg32Sint,
        ktx2::Format::R32G32_SFLOAT => TextureFormat::Rg32Float,
        ktx2::Format::R32G32B32_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R32G32B32_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R32G32B32_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R32G32B32A32_UINT => TextureFormat::Rgba32Uint,
        ktx2::Format::R32G32B32A32_SINT => TextureFormat::Rgba32Sint,
        ktx2::Format::R32G32B32A32_SFLOAT => TextureFormat::Rgba32Float,
        ktx2::Format::R64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64A64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64A64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64A64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        // FIXME: Is this correct?
        ktx2::Format::B10G11R11_UFLOAT_PACK32 => TextureFormat::Rg11b10Float,
        // FIXME: Is this correct?
        ktx2::Format::E5B9G9R9_UFLOAT_PACK32 => TextureFormat::Rgb9e5Ufloat,
        ktx2::Format::D16_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::X8_D24_UNORM_PACK32 => TextureFormat::Depth24Plus,
        ktx2::Format::D32_SFLOAT => TextureFormat::Depth32Float,
        ktx2::Format::S8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::D16_UNORM_S8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::D24_UNORM_S8_UINT => TextureFormat::Depth24PlusStencil8,
        ktx2::Format::D32_SFLOAT_S8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::BC1_RGB_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC1_RGB_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC1_RGBA_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC1_RGBA_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC2_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        ktx2::Format::BC2_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        ktx2::Format::BC3_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc3RgbaUnormSrgb
            } else {
                TextureFormat::Bc3RgbaUnorm
            }
        }
        ktx2::Format::BC3_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc3RgbaUnormSrgb
            } else {
                TextureFormat::Bc3RgbaUnorm
            }
        }
        ktx2::Format::BC4_UNORM_BLOCK => TextureFormat::Bc4RUnorm,
        ktx2::Format::BC4_SNORM_BLOCK => TextureFormat::Bc4RSnorm,
        ktx2::Format::BC5_UNORM_BLOCK => TextureFormat::Bc5RgUnorm,
        ktx2::Format::BC5_SNORM_BLOCK => TextureFormat::Bc5RgSnorm,
        ktx2::Format::BC6H_UFLOAT_BLOCK => TextureFormat::Bc6hRgbUfloat,
        ktx2::Format::BC6H_SFLOAT_BLOCK => TextureFormat::Bc6hRgbSfloat,
        ktx2::Format::BC7_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        ktx2::Format::BC7_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        ktx2::Format::ETC2_R8G8B8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A1_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8A1UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8A1Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A1_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8A1UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8A1Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            }
        }
        ktx2::Format::EAC_R11_UNORM_BLOCK => TextureFormat::EacR11Unorm,
        ktx2::Format::EAC_R11_SNORM_BLOCK => TextureFormat::EacR11Snorm,
        ktx2::Format::EAC_R11G11_UNORM_BLOCK => TextureFormat::EacRg11Unorm,
        ktx2::Format::EAC_R11G11_SNORM_BLOCK => TextureFormat::EacRg11Snorm,
        ktx2::Format::ASTC_4x4_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc4x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc4x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_4x4_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc4x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc4x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x4_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x4_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x6_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x6_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x6_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x6_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x6_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x6_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x10_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x10_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x10_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x10_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x12_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x12RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x12RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x12_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x12RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x12RgbaUnorm
            }
        }
        _ => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
    })
}
