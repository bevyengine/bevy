use ddsfile::{D3DFormat, Dds, DxgiFormat};
use std::io::Cursor;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use super::{CompressedImageFormats, Image, TextureError};

pub fn dds_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
) -> Result<Image, TextureError> {
    let mut cursor = Cursor::new(buffer);
    let dds = Dds::read(&mut cursor).expect("Failed to parse DDS file");
    let texture_format = dds_format_to_texture_format(&dds, is_srgb)?;
    if !supported_compressed_formats.supports(texture_format) {
        return Err(TextureError::UnsupportedTextureFormat(format!(
            "Format not supported by this GPU: {texture_format:?}",
        )));
    }
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: dds.get_width(),
        height: dds.get_height(),
        depth_or_array_layers: if dds.get_num_array_layers() > 1 {
            dds.get_num_array_layers()
        } else {
            dds.get_depth()
        },
    }
    .physical_size(texture_format);
    image.texture_descriptor.mip_level_count = dds.get_num_mipmap_levels();
    image.texture_descriptor.format = texture_format;
    image.texture_descriptor.dimension = if dds.get_depth() > 1 {
        TextureDimension::D3
    } else if image.is_compressed() || dds.get_height() > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    };
    image.data = dds.data;
    Ok(image)
}

pub fn dds_format_to_texture_format(
    dds: &Dds,
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    Ok(if let Some(d3d_format) = dds.get_d3d_format() {
        match d3d_format {
            D3DFormat::A8B8G8R8 => {
                if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }
            }
            D3DFormat::A8 => TextureFormat::R8Unorm,
            D3DFormat::A8R8G8B8 => {
                if is_srgb {
                    TextureFormat::Bgra8UnormSrgb
                } else {
                    TextureFormat::Bgra8Unorm
                }
            }
            D3DFormat::G16R16 => TextureFormat::Rg16Uint,
            D3DFormat::A2B10G10R10 => TextureFormat::Rgb10a2Unorm,
            D3DFormat::A8L8 => TextureFormat::Rg8Uint,
            D3DFormat::L16 => TextureFormat::R16Uint,
            D3DFormat::L8 => TextureFormat::R8Uint,
            D3DFormat::DXT1 => {
                if is_srgb {
                    TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    TextureFormat::Bc1RgbaUnorm
                }
            }
            D3DFormat::DXT3 | D3DFormat::DXT2 => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            D3DFormat::DXT5 | D3DFormat::DXT4 => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            D3DFormat::A16B16G16R16 => TextureFormat::Rgba16Uint,
            D3DFormat::Q16W16V16U16 => TextureFormat::Rgba16Sint,
            D3DFormat::R16F => TextureFormat::R16Float,
            D3DFormat::G16R16F => TextureFormat::Rg16Float,
            D3DFormat::A16B16G16R16F => TextureFormat::Rgba16Float,
            D3DFormat::R32F => TextureFormat::R32Float,
            D3DFormat::G32R32F => TextureFormat::Rg32Float,
            D3DFormat::A32B32G32R32F => TextureFormat::Rgba32Float,
            D3DFormat::A1R5G5B5
            | D3DFormat::R5G6B5
            // FIXME: Map to argb format and user has to know to ignore the alpha channel?
            | D3DFormat::X8R8G8B8
            // FIXME: Map to argb format and user has to know to ignore the alpha channel?
            | D3DFormat::X8B8G8R8
            | D3DFormat::A2R10G10B10
            | D3DFormat::R8G8B8
            | D3DFormat::X1R5G5B5
            | D3DFormat::A4R4G4B4
            | D3DFormat::X4R4G4B4
            | D3DFormat::A8R3G3B2
            | D3DFormat::A4L4
            | D3DFormat::R8G8_B8G8
            | D3DFormat::G8R8_G8B8
            | D3DFormat::UYVY
            | D3DFormat::YUY2
            | D3DFormat::CXV8U8 => {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{d3d_format:?}",
                )))
            }
        }
    } else if let Some(dxgi_format) = dds.get_dxgi_format() {
        match dxgi_format {
            DxgiFormat::R32G32B32A32_Typeless | DxgiFormat::R32G32B32A32_Float => {
                TextureFormat::Rgba32Float
            }
            DxgiFormat::R32G32B32A32_UInt => TextureFormat::Rgba32Uint,
            DxgiFormat::R32G32B32A32_SInt => TextureFormat::Rgba32Sint,
            DxgiFormat::R16G16B16A16_Typeless | DxgiFormat::R16G16B16A16_Float => {
                TextureFormat::Rgba16Float
            }
            DxgiFormat::R16G16B16A16_UNorm => TextureFormat::Rgba16Unorm,
            DxgiFormat::R16G16B16A16_UInt => TextureFormat::Rgba16Uint,
            DxgiFormat::R16G16B16A16_SNorm => TextureFormat::Rgba16Snorm,
            DxgiFormat::R16G16B16A16_SInt => TextureFormat::Rgba16Sint,
            DxgiFormat::R32G32_Typeless | DxgiFormat::R32G32_Float => TextureFormat::Rg32Float,
            DxgiFormat::R32G32_UInt => TextureFormat::Rg32Uint,
            DxgiFormat::R32G32_SInt => TextureFormat::Rg32Sint,
            DxgiFormat::R10G10B10A2_Typeless | DxgiFormat::R10G10B10A2_UNorm => {
                TextureFormat::Rgb10a2Unorm
            }
            DxgiFormat::R11G11B10_Float => TextureFormat::Rg11b10Float,
            DxgiFormat::R8G8B8A8_Typeless
            | DxgiFormat::R8G8B8A8_UNorm
            | DxgiFormat::R8G8B8A8_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }
            }
            DxgiFormat::R8G8B8A8_UInt => TextureFormat::Rgba8Uint,
            DxgiFormat::R8G8B8A8_SNorm => TextureFormat::Rgba8Snorm,
            DxgiFormat::R8G8B8A8_SInt => TextureFormat::Rgba8Sint,
            DxgiFormat::R16G16_Typeless | DxgiFormat::R16G16_Float => TextureFormat::Rg16Float,
            DxgiFormat::R16G16_UNorm => TextureFormat::Rg16Unorm,
            DxgiFormat::R16G16_UInt => TextureFormat::Rg16Uint,
            DxgiFormat::R16G16_SNorm => TextureFormat::Rg16Snorm,
            DxgiFormat::R16G16_SInt => TextureFormat::Rg16Sint,
            DxgiFormat::R32_Typeless | DxgiFormat::R32_Float => TextureFormat::R32Float,
            DxgiFormat::D32_Float => TextureFormat::Depth32Float,
            DxgiFormat::R32_UInt => TextureFormat::R32Uint,
            DxgiFormat::R32_SInt => TextureFormat::R32Sint,
            DxgiFormat::R24G8_Typeless | DxgiFormat::D24_UNorm_S8_UInt => {
                TextureFormat::Depth24PlusStencil8
            }
            DxgiFormat::R24_UNorm_X8_Typeless => TextureFormat::Depth24Plus,
            DxgiFormat::R8G8_Typeless | DxgiFormat::R8G8_UNorm => TextureFormat::Rg8Unorm,
            DxgiFormat::R8G8_UInt => TextureFormat::Rg8Uint,
            DxgiFormat::R8G8_SNorm => TextureFormat::Rg8Snorm,
            DxgiFormat::R8G8_SInt => TextureFormat::Rg8Sint,
            DxgiFormat::R16_Typeless | DxgiFormat::R16_Float => TextureFormat::R16Float,
            DxgiFormat::R16_UNorm => TextureFormat::R16Unorm,
            DxgiFormat::R16_UInt => TextureFormat::R16Uint,
            DxgiFormat::R16_SNorm => TextureFormat::R16Snorm,
            DxgiFormat::R16_SInt => TextureFormat::R16Sint,
            DxgiFormat::R8_Typeless | DxgiFormat::R8_UNorm => TextureFormat::R8Unorm,
            DxgiFormat::R8_UInt => TextureFormat::R8Uint,
            DxgiFormat::R8_SNorm => TextureFormat::R8Snorm,
            DxgiFormat::R8_SInt => TextureFormat::R8Sint,
            DxgiFormat::R9G9B9E5_SharedExp => TextureFormat::Rgb9e5Ufloat,
            DxgiFormat::BC1_Typeless | DxgiFormat::BC1_UNorm | DxgiFormat::BC1_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    TextureFormat::Bc1RgbaUnorm
                }
            }
            DxgiFormat::BC2_Typeless | DxgiFormat::BC2_UNorm | DxgiFormat::BC2_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            DxgiFormat::BC3_Typeless | DxgiFormat::BC3_UNorm | DxgiFormat::BC3_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            DxgiFormat::BC4_Typeless | DxgiFormat::BC4_UNorm => TextureFormat::Bc4RUnorm,
            DxgiFormat::BC4_SNorm => TextureFormat::Bc4RSnorm,
            DxgiFormat::BC5_Typeless | DxgiFormat::BC5_UNorm => TextureFormat::Bc5RgUnorm,
            DxgiFormat::BC5_SNorm => TextureFormat::Bc5RgSnorm,
            DxgiFormat::B8G8R8A8_UNorm
            | DxgiFormat::B8G8R8A8_Typeless
            | DxgiFormat::B8G8R8A8_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bgra8UnormSrgb
                } else {
                    TextureFormat::Bgra8Unorm
                }
            }

            DxgiFormat::BC6H_Typeless | DxgiFormat::BC6H_UF16 => TextureFormat::Bc6hRgbUfloat,
            DxgiFormat::BC6H_SF16 => TextureFormat::Bc6hRgbFloat,
            DxgiFormat::BC7_Typeless | DxgiFormat::BC7_UNorm | DxgiFormat::BC7_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc7RgbaUnormSrgb
                } else {
                    TextureFormat::Bc7RgbaUnorm
                }
            }
            _ => {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{dxgi_format:?}",
                )))
            }
        }
    } else {
        return Err(TextureError::UnsupportedTextureFormat(
            "unspecified".to_string(),
        ));
    })
}
