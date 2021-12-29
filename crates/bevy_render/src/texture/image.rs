use std::io::Cursor;

use super::image_texture_conversion::image_to_texture;
use crate::{
    render_asset::{PrepareAssetError, RenderAsset},
    render_resource::{Sampler, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
    texture::BevyDefault,
};
use bevy_asset::HandleUntyped;
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_math::{Size, Vec2};
use bevy_reflect::TypeUuid;
use ddsfile::{D3DFormat, Dds, DxgiFormat};
use thiserror::Error;
use wgpu::{
    Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, TextureDimension, TextureFormat,
    TextureViewDescriptor,
};

pub const TEXTURE_ASSET_INDEX: u64 = 0;
pub const SAMPLER_ASSET_INDEX: u64 = 1;
pub const DEFAULT_IMAGE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Image::TYPE_UUID, 13148262314052771789);

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f"]
pub struct Image {
    pub data: Vec<u8>,
    // TODO: this nesting makes accessing Image metadata verbose. Either flatten out descriptor or add accessors
    pub texture_descriptor: wgpu::TextureDescriptor<'static>,
    pub sampler_descriptor: wgpu::SamplerDescriptor<'static>,
}

impl Default for Image {
    fn default() -> Self {
        let format = wgpu::TextureFormat::bevy_default();
        let data = vec![255; format.pixel_size() as usize];
        Image {
            data,
            texture_descriptor: wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                format,
                dimension: wgpu::TextureDimension::D2,
                label: None,
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            },
            sampler_descriptor: wgpu::SamplerDescriptor::default(),
        }
    }
}

impl Image {
    /// Creates a new image from raw binary data and the corresponding metadata.
    ///
    /// # Panics
    /// Panics if the length of the `data`, volume of the `size` and the size of the `format`
    /// do not match.
    pub fn new(
        size: Extent3d,
        dimension: TextureDimension,
        data: Vec<u8>,
        format: TextureFormat,
    ) -> Self {
        debug_assert_eq!(
            size.volume() * format.pixel_size(),
            data.len(),
            "Pixel data, size and format have to match",
        );
        let mut image = Self {
            data,
            ..Default::default()
        };
        image.texture_descriptor.dimension = dimension;
        image.texture_descriptor.size = size;
        image.texture_descriptor.format = format;
        image
    }

    /// Creates a new image from raw binary data and the corresponding metadata, by filling
    /// the image data with the `pixel` data repeated multiple times.
    ///
    /// # Panics
    /// Panics if the size of the `format` is not a multiple of the length of the `pixel` data.
    /// do not match.
    pub fn new_fill(
        size: Extent3d,
        dimension: TextureDimension,
        pixel: &[u8],
        format: TextureFormat,
    ) -> Self {
        let mut value = Image::default();
        value.texture_descriptor.format = format;
        value.texture_descriptor.dimension = dimension;
        value.resize(size);

        debug_assert_eq!(
            pixel.len() % format.pixel_size(),
            0,
            "Must not have incomplete pixel data."
        );
        debug_assert!(
            pixel.len() <= value.data.len(),
            "Fill data must fit within pixel buffer."
        );

        for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
            current_pixel.copy_from_slice(pixel);
        }
        value
    }

    /// Returns the aspect ratio (height/width) of a 2D image.
    pub fn aspect_2d(&self) -> f32 {
        self.texture_descriptor.size.height as f32 / self.texture_descriptor.size.width as f32
    }

    /// Returns the size of a 2D image.
    pub fn size(&self) -> Vec2 {
        Vec2::new(
            self.texture_descriptor.size.width as f32,
            self.texture_descriptor.size.height as f32,
        )
    }

    /// Resizes the image to the new size, by removing information or appending 0 to the `data`.
    /// Does not properly resize the contents of the image, but only its internal `data` buffer.
    pub fn resize(&mut self, size: Extent3d) {
        self.texture_descriptor.size = size;
        self.data.resize(
            size.volume() * self.texture_descriptor.format.pixel_size(),
            0,
        );
    }

    /// Changes the `size`, asserting that the total number of data elements (pixels) remains the
    /// same.
    ///
    /// # Panics
    /// Panics if the `new_size` does not have the same volume as to old one.
    pub fn reinterpret_size(&mut self, new_size: Extent3d) {
        assert!(
            new_size.volume() == self.texture_descriptor.size.volume(),
            "Incompatible sizes: old = {:?} new = {:?}",
            self.texture_descriptor.size,
            new_size
        );

        self.texture_descriptor.size = new_size;
    }

    /// Takes a 2D image containing vertically stacked images of the same size, and reinterprets
    /// it as a 2D array texture, where each of the stacked images becomes one layer of the
    /// array. This is primarily for use with the `texture2DArray` shader uniform type.
    ///
    /// # Panics
    /// Panics if the texture is not 2D, has more than one layers or is not evenly dividable into
    /// the `layers`.
    pub fn reinterpret_stacked_2d_as_array(&mut self, layers: u32) {
        // Must be a stacked image, and the height must be divisible by layers.
        assert!(self.texture_descriptor.dimension == TextureDimension::D2);
        assert!(self.texture_descriptor.size.depth_or_array_layers == 1);
        assert_eq!(self.texture_descriptor.size.height % layers, 0);

        self.reinterpret_size(Extent3d {
            width: self.texture_descriptor.size.width,
            height: self.texture_descriptor.size.height / layers,
            depth_or_array_layers: layers,
        });
    }

    /// Convert a texture from a format to another
    /// Only a few formats are supported as input and output:
    /// - `TextureFormat::R8Unorm`
    /// - `TextureFormat::Rg8Unorm`
    /// - `TextureFormat::Rgba8UnormSrgb`
    /// - `TextureFormat::Bgra8UnormSrgb`
    pub fn convert(&self, new_format: TextureFormat) -> Option<Self> {
        super::image_texture_conversion::texture_to_image(self)
            .and_then(|img| match new_format {
                TextureFormat::R8Unorm => {
                    Some((image::DynamicImage::ImageLuma8(img.into_luma8()), false))
                }
                TextureFormat::Rg8Unorm => Some((
                    image::DynamicImage::ImageLumaA8(img.into_luma_alpha8()),
                    false,
                )),
                TextureFormat::Rgba8UnormSrgb => {
                    Some((image::DynamicImage::ImageRgba8(img.into_rgba8()), true))
                }
                TextureFormat::Bgra8UnormSrgb => {
                    Some((image::DynamicImage::ImageBgra8(img.into_bgra8()), true))
                }
                _ => None,
            })
            .map(|(dyn_img, is_srgb)| {
                super::image_texture_conversion::image_to_texture(dyn_img, is_srgb)
            })
    }

    /// Load a bytes buffer in a [`Image`], according to type `image_type`, using the `image`
    /// crate
    pub fn from_buffer(
        buffer: &[u8],
        image_type: ImageType,
        is_srgb: bool,
    ) -> Result<Image, TextureError> {
        let format = match image_type {
            ImageType::MimeType(mime_type) => match mime_type {
                "image/png" => Ok(image::ImageFormat::Png),
                "image/vnd-ms.dds" => Ok(image::ImageFormat::Dds),
                "image/x-targa" => Ok(image::ImageFormat::Tga),
                "image/x-tga" => Ok(image::ImageFormat::Tga),
                "image/jpeg" => Ok(image::ImageFormat::Jpeg),
                "image/bmp" => Ok(image::ImageFormat::Bmp),
                "image/x-bmp" => Ok(image::ImageFormat::Bmp),
                _ => Err(TextureError::InvalidImageMimeType(mime_type.to_string())),
            },
            ImageType::Extension(extension) => image::ImageFormat::from_extension(extension)
                .ok_or_else(|| TextureError::InvalidImageMimeType(extension.to_string())),
        }?;

        // Load the image in the expected format.
        // Some formats like PNG allow for R or RG textures too, so the texture
        // format needs to be determined. For RGB textures an alpha channel
        // needs to be added, so the image data needs to be converted in those
        // cases.

        match format {
            image::ImageFormat::Dds => Ok(dds_buffer_to_image(buffer, is_srgb)),
            _ => {
                let dyn_img = image::load_from_memory_with_format(buffer, format)?;
                Ok(image_to_texture(dyn_img, is_srgb))
            }
        }
    }

    /// Whether the texture format is compressed or uncompressed
    pub fn is_compressed(&self) -> bool {
        let format_description = self.texture_descriptor.format.describe();
        format_description
            .required_features
            .contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC_LDR)
            || format_description
                .required_features
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
            || format_description
                .required_features
                .contains(wgpu::Features::TEXTURE_COMPRESSION_ETC2)
    }
}

fn dds_buffer_to_image(buffer: &[u8], is_srgb: bool) -> Image {
    let mut cursor = Cursor::new(buffer);
    let dds = Dds::read(&mut cursor).expect("Failed to parse DDS file");
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: dds.get_width(),
        height: dds.get_height(),
        depth_or_array_layers: if dds.get_num_array_layers() > 1 {
            dds.get_num_array_layers()
        } else {
            dds.get_depth()
        },
    };
    image.texture_descriptor.mip_level_count = dds.get_num_mipmap_levels();
    image.texture_descriptor.format = dds_format_to_texture_format(&dds, is_srgb)
        .unwrap_or_else(|| panic!("Unsupported DDS format: {:?}", dds));
    image.texture_descriptor.dimension = if dds.get_depth() > 1 {
        TextureDimension::D3
    } else if image.is_compressed() || dds.get_height() > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    };
    image.data = dds.data;
    image
}

fn dds_format_to_texture_format(dds: &Dds, is_srgb: bool) -> Option<TextureFormat> {
    Some(if let Some(d3d_format) = dds.get_d3d_format() {
        match d3d_format {
            D3DFormat::A8B8G8R8 => {
                if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }
            }
            D3DFormat::G16R16 => TextureFormat::Rg16Uint,
            D3DFormat::A2B10G10R10 => TextureFormat::Rgb10a2Unorm,
            D3DFormat::A1R5G5B5 => return None,
            D3DFormat::R5G6B5 => return None,
            D3DFormat::A8 => TextureFormat::R8Unorm,
            D3DFormat::A8R8G8B8 => {
                if is_srgb {
                    TextureFormat::Bgra8UnormSrgb
                } else {
                    TextureFormat::Bgra8Unorm
                }
            }
            // FIXME: Map to argb format and user has to know to ignore the alpha channel?
            D3DFormat::X8R8G8B8 => return None,
            // FIXME: Map to argb format and user has to know to ignore the alpha channel?
            D3DFormat::X8B8G8R8 => return None,
            D3DFormat::A2R10G10B10 => return None,
            D3DFormat::R8G8B8 => return None,
            D3DFormat::X1R5G5B5 => return None,
            D3DFormat::A4R4G4B4 => return None,
            D3DFormat::X4R4G4B4 => return None,
            D3DFormat::A8R3G3B2 => return None,
            D3DFormat::A8L8 => TextureFormat::Rg8Uint,
            D3DFormat::L16 => TextureFormat::R16Uint,
            D3DFormat::L8 => TextureFormat::R8Uint,
            D3DFormat::A4L4 => return None,
            D3DFormat::DXT1 => {
                if is_srgb {
                    TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    TextureFormat::Bc1RgbaUnorm
                }
            }
            D3DFormat::DXT3 => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            D3DFormat::DXT5 => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            D3DFormat::R8G8_B8G8 => return None,
            D3DFormat::G8R8_G8B8 => return None,
            D3DFormat::A16B16G16R16 => TextureFormat::Rgba16Uint,
            D3DFormat::Q16W16V16U16 => TextureFormat::Rgba16Sint,
            D3DFormat::R16F => TextureFormat::R16Float,
            D3DFormat::G16R16F => TextureFormat::Rg16Float,
            D3DFormat::A16B16G16R16F => TextureFormat::Rgba16Float,
            D3DFormat::R32F => TextureFormat::R32Float,
            D3DFormat::G32R32F => TextureFormat::Rg32Float,
            D3DFormat::A32B32G32R32F => TextureFormat::Rgba32Float,
            D3DFormat::DXT2 => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            D3DFormat::DXT4 => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            D3DFormat::UYVY => return None,
            D3DFormat::YUY2 => return None,
            D3DFormat::CXV8U8 => return None,
        }
    } else if let Some(dxgi_format) = dds.get_dxgi_format() {
        match dxgi_format {
            DxgiFormat::Unknown => return None,
            DxgiFormat::R32G32B32A32_Typeless => TextureFormat::Rgba32Float,
            DxgiFormat::R32G32B32A32_Float => TextureFormat::Rgba32Float,
            DxgiFormat::R32G32B32A32_UInt => TextureFormat::Rgba32Uint,
            DxgiFormat::R32G32B32A32_SInt => TextureFormat::Rgba32Sint,
            DxgiFormat::R32G32B32_Typeless => return None,
            DxgiFormat::R32G32B32_Float => return None,
            DxgiFormat::R32G32B32_UInt => return None,
            DxgiFormat::R32G32B32_SInt => return None,
            DxgiFormat::R16G16B16A16_Typeless => TextureFormat::Rgba16Float,
            DxgiFormat::R16G16B16A16_Float => TextureFormat::Rgba16Float,
            DxgiFormat::R16G16B16A16_UNorm => TextureFormat::Rgba16Unorm,
            DxgiFormat::R16G16B16A16_UInt => TextureFormat::Rgba16Uint,
            DxgiFormat::R16G16B16A16_SNorm => TextureFormat::Rgba16Snorm,
            DxgiFormat::R16G16B16A16_SInt => TextureFormat::Rgba16Sint,
            DxgiFormat::R32G32_Typeless => TextureFormat::Rg32Float,
            DxgiFormat::R32G32_Float => TextureFormat::Rg32Float,
            DxgiFormat::R32G32_UInt => TextureFormat::Rg32Uint,
            DxgiFormat::R32G32_SInt => TextureFormat::Rg32Sint,
            DxgiFormat::R32G8X24_Typeless => return None,
            DxgiFormat::D32_Float_S8X24_UInt => return None,
            DxgiFormat::R32_Float_X8X24_Typeless => return None,
            DxgiFormat::X32_Typeless_G8X24_UInt => return None,
            DxgiFormat::R10G10B10A2_Typeless => TextureFormat::Rgb10a2Unorm,
            DxgiFormat::R10G10B10A2_UNorm => TextureFormat::Rgb10a2Unorm,
            DxgiFormat::R10G10B10A2_UInt => return None,
            DxgiFormat::R11G11B10_Float => TextureFormat::Rg11b10Float,
            DxgiFormat::R8G8B8A8_Typeless => {
                if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }
            }
            DxgiFormat::R8G8B8A8_UNorm => {
                if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }
            }
            DxgiFormat::R8G8B8A8_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }
            }
            DxgiFormat::R8G8B8A8_UInt => TextureFormat::Rgba8Uint,
            DxgiFormat::R8G8B8A8_SNorm => TextureFormat::Rgba8Snorm,
            DxgiFormat::R8G8B8A8_SInt => TextureFormat::Rgba8Sint,
            DxgiFormat::R16G16_Typeless => TextureFormat::Rg16Float,
            DxgiFormat::R16G16_Float => TextureFormat::Rg16Float,
            DxgiFormat::R16G16_UNorm => TextureFormat::Rg16Unorm,
            DxgiFormat::R16G16_UInt => TextureFormat::Rg16Uint,
            DxgiFormat::R16G16_SNorm => TextureFormat::Rg16Snorm,
            DxgiFormat::R16G16_SInt => TextureFormat::Rg16Sint,
            DxgiFormat::R32_Typeless => TextureFormat::R32Float,
            DxgiFormat::D32_Float => TextureFormat::Depth32Float,
            DxgiFormat::R32_Float => TextureFormat::R32Float,
            DxgiFormat::R32_UInt => TextureFormat::R32Uint,
            DxgiFormat::R32_SInt => TextureFormat::R32Sint,
            // FIXME: Is this correct?
            DxgiFormat::R24G8_Typeless => TextureFormat::Depth24PlusStencil8,
            DxgiFormat::D24_UNorm_S8_UInt => TextureFormat::Depth24PlusStencil8,
            // FIXME: Is this correct?
            DxgiFormat::R24_UNorm_X8_Typeless => TextureFormat::Depth24Plus,
            DxgiFormat::X24_Typeless_G8_UInt => return None,
            DxgiFormat::R8G8_Typeless => TextureFormat::Rg8Unorm,
            DxgiFormat::R8G8_UNorm => TextureFormat::Rg8Unorm,
            DxgiFormat::R8G8_UInt => TextureFormat::Rg8Uint,
            DxgiFormat::R8G8_SNorm => TextureFormat::Rg8Snorm,
            DxgiFormat::R8G8_SInt => TextureFormat::Rg8Sint,
            DxgiFormat::R16_Typeless => TextureFormat::R16Float,
            DxgiFormat::R16_Float => TextureFormat::R16Float,
            DxgiFormat::D16_UNorm => return None,
            DxgiFormat::R16_UNorm => TextureFormat::R16Unorm,
            DxgiFormat::R16_UInt => TextureFormat::R16Uint,
            DxgiFormat::R16_SNorm => TextureFormat::R16Snorm,
            DxgiFormat::R16_SInt => TextureFormat::R16Sint,
            DxgiFormat::R8_Typeless => TextureFormat::R8Unorm,
            DxgiFormat::R8_UNorm => TextureFormat::R8Unorm,
            DxgiFormat::R8_UInt => TextureFormat::R8Uint,
            DxgiFormat::R8_SNorm => TextureFormat::R8Snorm,
            DxgiFormat::R8_SInt => TextureFormat::R8Sint,
            DxgiFormat::A8_UNorm => return None,
            DxgiFormat::R1_UNorm => return None,
            DxgiFormat::R9G9B9E5_SharedExp => TextureFormat::Rgb9e5Ufloat,
            DxgiFormat::R8G8_B8G8_UNorm => return None,
            DxgiFormat::G8R8_G8B8_UNorm => return None,
            DxgiFormat::BC1_Typeless => {
                if is_srgb {
                    TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    TextureFormat::Bc1RgbaUnorm
                }
            }
            DxgiFormat::BC1_UNorm => {
                if is_srgb {
                    TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    TextureFormat::Bc1RgbaUnorm
                }
            }
            DxgiFormat::BC1_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    TextureFormat::Bc1RgbaUnorm
                }
            }
            DxgiFormat::BC2_Typeless => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            DxgiFormat::BC2_UNorm => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            DxgiFormat::BC2_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc2RgbaUnormSrgb
                } else {
                    TextureFormat::Bc2RgbaUnorm
                }
            }
            DxgiFormat::BC3_Typeless => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            DxgiFormat::BC3_UNorm => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            DxgiFormat::BC3_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    TextureFormat::Bc3RgbaUnorm
                }
            }
            DxgiFormat::BC4_Typeless => TextureFormat::Bc4RUnorm,
            DxgiFormat::BC4_UNorm => TextureFormat::Bc4RUnorm,
            DxgiFormat::BC4_SNorm => TextureFormat::Bc4RSnorm,
            DxgiFormat::BC5_Typeless => TextureFormat::Bc5RgUnorm,
            DxgiFormat::BC5_UNorm => TextureFormat::Bc5RgUnorm,
            DxgiFormat::BC5_SNorm => TextureFormat::Bc5RgSnorm,
            DxgiFormat::B5G6R5_UNorm => return None,
            DxgiFormat::B5G5R5A1_UNorm => return None,
            DxgiFormat::B8G8R8A8_UNorm => {
                if is_srgb {
                    TextureFormat::Bgra8UnormSrgb
                } else {
                    TextureFormat::Bgra8Unorm
                }
            }
            DxgiFormat::B8G8R8X8_UNorm => return None,
            DxgiFormat::R10G10B10_XR_Bias_A2_UNorm => return None,
            DxgiFormat::B8G8R8A8_Typeless => {
                if is_srgb {
                    TextureFormat::Bgra8UnormSrgb
                } else {
                    TextureFormat::Bgra8Unorm
                }
            }
            DxgiFormat::B8G8R8A8_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bgra8UnormSrgb
                } else {
                    TextureFormat::Bgra8Unorm
                }
            }
            DxgiFormat::B8G8R8X8_Typeless => return None,
            DxgiFormat::B8G8R8X8_UNorm_sRGB => return None,
            DxgiFormat::BC6H_Typeless => TextureFormat::Bc6hRgbUfloat,
            DxgiFormat::BC6H_UF16 => TextureFormat::Bc6hRgbUfloat,
            DxgiFormat::BC6H_SF16 => TextureFormat::Bc6hRgbSfloat,
            DxgiFormat::BC7_Typeless => {
                if is_srgb {
                    TextureFormat::Bc7RgbaUnormSrgb
                } else {
                    TextureFormat::Bc7RgbaUnorm
                }
            }
            DxgiFormat::BC7_UNorm => {
                if is_srgb {
                    TextureFormat::Bc7RgbaUnormSrgb
                } else {
                    TextureFormat::Bc7RgbaUnorm
                }
            }
            DxgiFormat::BC7_UNorm_sRGB => {
                if is_srgb {
                    TextureFormat::Bc7RgbaUnormSrgb
                } else {
                    TextureFormat::Bc7RgbaUnorm
                }
            }
            DxgiFormat::AYUV => return None,
            DxgiFormat::Y410 => return None,
            DxgiFormat::Y416 => return None,
            DxgiFormat::NV12 => return None,
            DxgiFormat::P010 => return None,
            DxgiFormat::P016 => return None,
            DxgiFormat::Format_420_Opaque => return None,
            DxgiFormat::YUY2 => return None,
            DxgiFormat::Y210 => return None,
            DxgiFormat::Y216 => return None,
            DxgiFormat::NV11 => return None,
            DxgiFormat::AI44 => return None,
            DxgiFormat::IA44 => return None,
            DxgiFormat::P8 => return None,
            DxgiFormat::A8P8 => return None,
            DxgiFormat::B4G4R4A4_UNorm => return None,
            DxgiFormat::P208 => return None,
            DxgiFormat::V208 => return None,
            DxgiFormat::V408 => return None,
            DxgiFormat::Force_UInt => return None,
        }
    } else {
        panic!("Invalid dds format?")
    })
}

/// An error that occurs when loading a texture
#[derive(Error, Debug)]
pub enum TextureError {
    #[error("invalid image mime type")]
    InvalidImageMimeType(String),
    #[error("invalid image extension")]
    InvalidImageExtension(String),
    #[error("failed to load an image: {0}")]
    ImageError(#[from] image::ImageError),
}

/// The type of a raw image buffer.
pub enum ImageType<'a> {
    /// The mime type of an image, for example `"image/png"`.
    MimeType(&'a str),
    /// The extension of an image file, for example `"png"`.
    Extension(&'a str),
}

/// Used to calculate the volume of an item.
pub trait Volume {
    fn volume(&self) -> usize;
}

impl Volume for Extent3d {
    /// Calculates the volume of the [`Extent3d`].
    fn volume(&self) -> usize {
        (self.width * self.height * self.depth_or_array_layers) as usize
    }
}

/// Information about the pixel size in bytes and the number of different components.
pub struct PixelInfo {
    /// The size of a component of a pixel in bytes.
    pub type_size: usize,
    /// The amount of different components (color channels).
    pub num_components: usize,
}

/// Extends the wgpu [`TextureFormat`] with information about the pixel.
pub trait TextureFormatPixelInfo {
    /// Returns the pixel information of the format.
    fn pixel_info(&self) -> PixelInfo;
    /// Returns the size of a pixel of the format.
    fn pixel_size(&self) -> usize {
        let info = self.pixel_info();
        info.type_size * info.num_components
    }
}

impl TextureFormatPixelInfo for TextureFormat {
    fn pixel_info(&self) -> PixelInfo {
        let type_size = match self {
            // 8bit
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint
            | TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb => 1,

            // 16bit
            TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Float
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Float
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float => 2,

            // 32bit
            TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float
            | TextureFormat::Rg32Uint
            | TextureFormat::Rg32Sint
            | TextureFormat::Rg32Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float
            | TextureFormat::Depth32Float => 4,

            // special cases
            TextureFormat::Rgb10a2Unorm => 4,
            TextureFormat::Rg11b10Float => 4,
            TextureFormat::Depth24Plus => 3, // FIXME is this correct?
            TextureFormat::Depth24PlusStencil8 => 4,
            // TODO: this is not good! this is a temporary step while porting bevy_render to direct wgpu usage
            _ => panic!("cannot get pixel info for type"),
        };

        let components = match self {
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint
            | TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Float
            | TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float => 1,

            TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Float
            | TextureFormat::Rg32Uint
            | TextureFormat::Rg32Sint
            | TextureFormat::Rg32Float => 2,

            TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float => 4,

            // special cases
            TextureFormat::Rgb10a2Unorm
            | TextureFormat::Rg11b10Float
            | TextureFormat::Depth32Float
            | TextureFormat::Depth24Plus
            | TextureFormat::Depth24PlusStencil8 => 1,
            // TODO: this is not good! this is a temporary step while porting bevy_render to direct wgpu usage
            _ => panic!("cannot get pixel info for type"),
        };

        PixelInfo {
            type_size,
            num_components: components,
        }
    }
}

/// The GPU-representation of an [`Image`].
/// Consists of the [`Texture`], its [`TextureView`] and the corresponding [`Sampler`], and the texture's [`Size`].
#[derive(Debug, Clone)]
pub struct GpuImage {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub sampler: Sampler,
    pub size: Size,
}

impl RenderAsset for Image {
    type ExtractedAsset = Image;
    type PreparedAsset = GpuImage;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    /// Clones the Image.
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    /// Converts the extracted image into a [`GpuImage`].
    fn prepare_asset(
        image: Self::ExtractedAsset,
        (render_device, render_queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let texture = if image.texture_descriptor.mip_level_count > 1 || image.is_compressed() {
            render_device.create_texture_with_data(
                render_queue,
                &image.texture_descriptor,
                &image.data,
            )
        } else {
            let texture = render_device.create_texture(&image.texture_descriptor);
            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        std::num::NonZeroU32::new(
                            image.texture_descriptor.size.width * format_size as u32,
                        )
                        .unwrap(),
                    ),
                    rows_per_image: if image.texture_descriptor.size.depth_or_array_layers > 1 {
                        std::num::NonZeroU32::new(image.texture_descriptor.size.height)
                    } else {
                        None
                    },
                },
                image.texture_descriptor.size,
            );
            texture
        };

        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let size = Size::new(
            image.texture_descriptor.size.width as f32,
            image.texture_descriptor.size.height as f32,
        );
        let sampler = render_device.create_sampler(&image.sampler_descriptor);
        Ok(GpuImage {
            texture,
            texture_view,
            sampler,
            size,
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn image_size() {
        let size = Extent3d {
            width: 200,
            height: 100,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
        );
        assert_eq!(
            Vec2::new(size.width as f32, size.height as f32),
            image.size()
        );
    }
    #[test]
    fn image_default_size() {
        let image = Image::default();
        assert_eq!(Vec2::new(1.0, 1.0), image.size());
    }
}
