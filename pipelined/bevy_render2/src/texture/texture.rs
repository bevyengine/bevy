use super::{
    image_texture_conversion::image_to_texture, Extent3d, SamplerDescriptor, TextureDimension,
    TextureFormat,
};
use crate::render_resource::{SamplerId, TextureId};
use bevy_reflect::TypeUuid;
use thiserror::Error;

pub const TEXTURE_ASSET_INDEX: u64 = 0;
pub const SAMPLER_ASSET_INDEX: u64 = 1;

// TODO: this shouldn't live in the Texture type
#[derive(Debug, Clone)]
pub struct GpuData {
    pub texture_id: TextureId,
    pub sampler_id: SamplerId,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f"]
pub struct Texture {
    pub data: Vec<u8>,
    pub gpu_data: Option<GpuData>,
    pub size: Extent3d,
    pub format: TextureFormat,
    pub dimension: TextureDimension,
    pub sampler: SamplerDescriptor,
}

impl Default for Texture {
    fn default() -> Self {
        Texture {
            data: Default::default(),
            gpu_data: None,
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            format: TextureFormat::Rgba8UnormSrgb,
            dimension: TextureDimension::D2,
            sampler: Default::default(),
        }
    }
}

impl Texture {
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
        Self {
            data,
            size,
            format,
            dimension,
            ..Default::default()
        }
    }

    pub fn new_fill(
        size: Extent3d,
        dimension: TextureDimension,
        pixel: &[u8],
        format: TextureFormat,
    ) -> Self {
        let mut value = Texture {
            format,
            dimension,
            ..Default::default()
        };
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
            current_pixel.copy_from_slice(&pixel);
        }
        value
    }

    pub fn aspect_2d(&self) -> f32 {
        self.size.height as f32 / self.size.width as f32
    }

    pub fn resize(&mut self, size: Extent3d) {
        self.size = size;
        self.data
            .resize(size.volume() * self.format.pixel_size(), 0);
    }

    /// Changes the `size`, asserting that the total number of data elements (pixels) remains the
    /// same.
    pub fn reinterpret_size(&mut self, new_size: Extent3d) {
        assert!(
            new_size.volume() == self.size.volume(),
            "Incompatible sizes: old = {:?} new = {:?}",
            self.size,
            new_size
        );

        self.size = new_size;
    }

    /// Takes a 2D texture containing vertically stacked images of the same size, and reinterprets
    /// it as a 2D array texture, where each of the stacked images becomes one layer of the
    /// array. This is primarily for use with the `texture2DArray` shader uniform type.
    pub fn reinterpret_stacked_2d_as_array(&mut self, layers: u32) {
        // Must be a stacked image, and the height must be divisible by layers.
        assert!(self.dimension == TextureDimension::D2);
        assert!(self.size.depth_or_array_layers == 1);
        assert_eq!(self.size.height % layers, 0);

        self.reinterpret_size(Extent3d {
            width: self.size.width,
            height: self.size.height / layers,
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
                TextureFormat::R8Unorm => Some(image::DynamicImage::ImageLuma8(img.into_luma8())),
                TextureFormat::Rg8Unorm => {
                    Some(image::DynamicImage::ImageLumaA8(img.into_luma_alpha8()))
                }
                TextureFormat::Rgba8UnormSrgb => {
                    Some(image::DynamicImage::ImageRgba8(img.into_rgba8()))
                }
                TextureFormat::Bgra8UnormSrgb => {
                    Some(image::DynamicImage::ImageBgra8(img.into_bgra8()))
                }
                _ => None,
            })
            .map(super::image_texture_conversion::image_to_texture)
    }

    /// Load a bytes buffer in a [`Texture`], according to type `image_type`, using the `image`
    /// crate`
    pub fn from_buffer(buffer: &[u8], image_type: ImageType) -> Result<Texture, TextureError> {
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

        let dyn_img = image::load_from_memory_with_format(buffer, format)?;
        Ok(image_to_texture(dyn_img))
    }
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

/// Type of a raw image buffer
pub enum ImageType<'a> {
    /// Mime type of an image, for example `"image/png"`
    MimeType(&'a str),
    /// Extension of an image file, for example `"png"`
    Extension(&'a str),
}
