#[cfg(debug_assertions)]
use bevy_utils::warn_once;
use ddsfile::{Caps2, D3DFormat, Dds, DxgiFormat};
use std::io::Cursor;
use wgpu::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use super::{CompressedImageFormats, Image, TextureError};

pub fn dds_buffer_to_image(
    #[cfg(debug_assertions)] name: String,
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
    let is_cubemap = dds.header.caps2.contains(Caps2::CUBEMAP);
    let depth_or_array_layers = if dds.get_num_array_layers() > 1 {
        dds.get_num_array_layers()
    } else {
        dds.get_depth()
    };
    if is_cubemap
        && !dds.header.caps2.contains(
            Caps2::CUBEMAP_NEGATIVEX
                | Caps2::CUBEMAP_NEGATIVEY
                | Caps2::CUBEMAP_NEGATIVEZ
                | Caps2::CUBEMAP_POSITIVEX
                | Caps2::CUBEMAP_POSITIVEY
                | Caps2::CUBEMAP_POSITIVEZ,
        )
    {
        return Err(TextureError::IncompleteCubemap);
    }
    image.texture_descriptor.size = Extent3d {
        width: dds.get_width(),
        height: dds.get_height(),
        depth_or_array_layers,
    }
    .physical_size(texture_format);
    let mip_map_level = match dds.get_num_mipmap_levels() {
        0 => {
            #[cfg(debug_assertions)]
            warn_once!(
                "Mipmap levels for texture {} are 0, bumping them to 1",
                name
            );
            1
        }
        t => t,
    };
    image.texture_descriptor.mip_level_count = mip_map_level;
    image.texture_descriptor.format = texture_format;
    image.texture_descriptor.dimension = if dds.get_depth() > 1 {
        TextureDimension::D3
    } else if image.is_compressed() || dds.get_height() > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    };
    if is_cubemap {
        let dimension = if image.texture_descriptor.size.depth_or_array_layers > 6 {
            TextureViewDimension::CubeArray
        } else {
            TextureViewDimension::Cube
        };
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(dimension),
            ..Default::default()
        });
    }
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
            D3DFormat::A16B16G16R16 => TextureFormat::Rgba16Unorm,
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

#[cfg(test)]
mod test {
    use wgpu::{util::TextureDataOrder, TextureDescriptor, TextureDimension};

    use crate::texture::CompressedImageFormats;

    use super::dds_buffer_to_image;

    /// `wgpu::create_texture_with_data` that reads from data structure but doesn't actually talk to your GPU
    fn fake_wgpu_create_texture_with_data(desc: &TextureDescriptor<'_>, data: &[u8]) {
        // Will return None only if it's a combined depth-stencil format
        // If so, default to 4, validation will fail later anyway since the depth or stencil
        // aspect needs to be written to individually
        let block_size = desc.format.block_copy_size(None).unwrap_or(4);
        let (block_width, block_height) = desc.format.block_dimensions();
        let layer_iterations = desc.array_layer_count();

        let outer_iteration;
        let inner_iteration;
        match TextureDataOrder::default() {
            TextureDataOrder::LayerMajor => {
                outer_iteration = layer_iterations;
                inner_iteration = desc.mip_level_count;
            }
            TextureDataOrder::MipMajor => {
                outer_iteration = desc.mip_level_count;
                inner_iteration = layer_iterations;
            }
        }

        let mut binary_offset = 0;
        for outer in 0..outer_iteration {
            for inner in 0..inner_iteration {
                let (_layer, mip) = match TextureDataOrder::default() {
                    TextureDataOrder::LayerMajor => (outer, inner),
                    TextureDataOrder::MipMajor => (inner, outer),
                };

                let mut mip_size = desc.mip_level_size(mip).unwrap();
                // copying layers separately
                if desc.dimension != TextureDimension::D3 {
                    mip_size.depth_or_array_layers = 1;
                }

                // When uploading mips of compressed textures and the mip is supposed to be
                // a size that isn't a multiple of the block size, the mip needs to be uploaded
                // as its "physical size" which is the size rounded up to the nearest block size.
                let mip_physical = mip_size.physical_size(desc.format);

                // All these calculations are performed on the physical size as that's the
                // data that exists in the buffer.
                let width_blocks = mip_physical.width / block_width;
                let height_blocks = mip_physical.height / block_height;

                let bytes_per_row = width_blocks * block_size;
                let data_size = bytes_per_row * height_blocks * mip_size.depth_or_array_layers;

                let end_offset = binary_offset + data_size as usize;

                assert!(binary_offset < data.len());
                assert!(end_offset <= data.len());
                // those asserts match how the data will be accessed by wgpu:
                // data[binary_offset..end_offset])

                binary_offset = end_offset;
            }
        }
    }

    #[test]
    fn dds_skybox() {
        let buffer: [u8; 224] = [
            0x44, 0x44, 0x53, 0x20, 0x7c, 0, 0, 0, 7, 0x10, 0x08, 0, 4, 0, 0, 0, 4, 0, 0, 0, 0x10,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x47, 0x49, 0x4d, 0x50, 0x2d, 0x44, 0x44, 0x53, 0x5c,
            0x09, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0x20, 0, 0, 0, 4, 0, 0, 0, 0x44, 0x58, 0x54, 0x35, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x08, 0x10, 0, 0, 0, 0xfe, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0x49, 0x92, 0x24, 0x49, 0x92, 0x24, 0xda, 0xd6,
            0x2f, 0x5b, 0x8a, 0, 0xff, 0x55, 0xff, 0xff, 0x49, 0x92, 0x24, 0x49, 0x92, 0x24, 0xd5,
            0x84, 0x8e, 0x3a, 0xb7, 0, 0xaa, 0x55, 0xff, 0xff, 0x49, 0x92, 0x24, 0x49, 0x92, 0x24,
            0xf5, 0x94, 0x6f, 0x32, 0x57, 0xb7, 0x8b, 0, 0xff, 0xff, 0x49, 0x92, 0x24, 0x49, 0x92,
            0x24, 0x2c, 0x3a, 0x49, 0x19, 0x28, 0xf7, 0xd7, 0xbe, 0xff, 0xff, 0x49, 0x92, 0x24,
            0x49, 0x92, 0x24, 0x16, 0x95, 0xae, 0x42, 0xfc, 0, 0xaa, 0x55, 0xff, 0xff, 0x49, 0x92,
            0x24, 0x49, 0x92, 0x24, 0xd8, 0xad, 0xae, 0x42, 0xaf, 0x0a, 0xaa, 0x55,
        ];
        let r = dds_buffer_to_image("".into(), &buffer, CompressedImageFormats::BC, true);
        assert!(r.is_ok());
        if let Ok(r) = r {
            fake_wgpu_create_texture_with_data(&r.texture_descriptor, &r.data);
        }
    }
}
